//! LLVM IR emitter — translates MIR into textual LLVM IR.
//!
//! This is the first Phase 14 backend increment: Agam user code can now be
//! lowered directly to `.ll` without going through C first. The supported MIR
//! surface intentionally covers the core scalar/string subset and leaves more
//! advanced runtime-heavy operations on the existing C backend for now.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::fmt::Write;

use agam_mir::ir::*;
use agam_sema::types::{FloatSize, IntSize, Type, builtin_type_by_id};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LlvmIntType {
    bits: u16,
    signed: bool,
}

impl LlvmIntType {
    fn from_size(size: IntSize, signed: bool) -> Self {
        let bits = match size {
            IntSize::I8 => 8,
            IntSize::I16 => 16,
            IntSize::I32 => 32,
            IntSize::I64 | IntSize::ISize => 64,
            IntSize::I128 => 128,
        };
        Self { bits, signed }
    }

    fn ir(self) -> &'static str {
        match self.bits {
            8 => "i8",
            16 => "i16",
            32 => "i32",
            64 => "i64",
            128 => "i128",
            _ => "i64",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LlvmFloatType {
    F32,
    F64,
}

impl LlvmFloatType {
    fn ir(self) -> &'static str {
        match self {
            LlvmFloatType::F32 => "float",
            LlvmFloatType::F64 => "double",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LlvmType {
    Int(LlvmIntType),
    Float(LlvmFloatType),
    Bool,
    Str,
    OpaquePtr,
}

impl LlvmType {
    fn ir(self) -> &'static str {
        match self {
            LlvmType::Int(int_ty) => int_ty.ir(),
            LlvmType::Float(float_ty) => float_ty.ir(),
            LlvmType::Bool => "i1",
            LlvmType::Str | LlvmType::OpaquePtr => "i8*",
        }
    }

    fn default_value(self) -> &'static str {
        match self {
            LlvmType::Int(_) => "0",
            LlvmType::Float(_) => "0.0",
            LlvmType::Bool => "false",
            LlvmType::Str | LlvmType::OpaquePtr => "null",
        }
    }

    fn default_int() -> Self {
        LlvmType::Int(LlvmIntType {
            bits: 32,
            signed: true,
        })
    }

    fn int_spec(self) -> Option<LlvmIntType> {
        match self {
            LlvmType::Int(int_ty) => Some(int_ty),
            _ => None,
        }
    }

    fn float_spec(self) -> Option<LlvmFloatType> {
        match self {
            LlvmType::Float(float_ty) => Some(float_ty),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct BuiltinSig {
    return_ty: LlvmType,
}

#[derive(Clone)]
struct FunctionLayout {
    params: Vec<LlvmType>,
    return_ty: LlvmType,
    value_types: HashMap<ValueId, LlvmType>,
    value_signs: HashMap<ValueId, SignInfo>,
    local_types: HashMap<String, LlvmType>,
    value_int_flags: HashMap<ValueId, IntArithFlags>,
}

#[derive(Clone)]
struct ValueRef {
    ty: LlvmType,
    repr: String,
    sign: SignInfo,
}

impl ValueRef {
    fn new(ty: LlvmType, repr: impl Into<String>, sign: SignInfo) -> Self {
        Self {
            ty,
            repr: repr.into(),
            sign,
        }
    }
}

struct GlobalString {
    name: String,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LlvmEmitOptions {
    target_triple: Option<String>,
    data_layout: Option<String>,
}

impl LlvmEmitOptions {
    fn from_env() -> Self {
        Self {
            target_triple: env::var("AGAM_LLVM_TARGET_TRIPLE")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            data_layout: env::var("AGAM_LLVM_DATA_LAYOUT")
                .ok()
                .filter(|value| !value.trim().is_empty()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct FunctionAttrs {
    nounwind: bool,
    nofree: bool,
    norecurse: bool,
    nosync: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
enum SignInfo {
    #[default]
    Unknown,
    NonNegative,
    Positive,
}

impl SignInfo {
    fn is_nonnegative(self) -> bool {
        matches!(self, SignInfo::NonNegative | SignInfo::Positive)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct IntArithFlags {
    nuw: bool,
    nsw: bool,
}

fn merge_sign(left: SignInfo, right: SignInfo) -> SignInfo {
    left.min(right)
}

fn default_sign_for_type(ty: LlvmType) -> SignInfo {
    match ty {
        LlvmType::Bool => SignInfo::NonNegative,
        LlvmType::Int(int_ty) if !int_ty.signed => SignInfo::NonNegative,
        _ => SignInfo::Unknown,
    }
}

fn int_const_sign(value: i64) -> SignInfo {
    if value > 0 {
        SignInfo::Positive
    } else if value >= 0 {
        SignInfo::NonNegative
    } else {
        SignInfo::Unknown
    }
}

fn bool_const_sign(value: bool) -> SignInfo {
    if value {
        SignInfo::Positive
    } else {
        SignInfo::NonNegative
    }
}

pub fn emit_llvm(module: &MirModule) -> Result<String, String> {
    emit_llvm_with_options(module, LlvmEmitOptions::from_env())
}

fn emit_llvm_with_options(module: &MirModule, options: LlvmEmitOptions) -> Result<String, String> {
    let layouts = analyze_module(module);
    let mut emitter = LlvmEmitter::new(module, layouts, options);
    emitter.emit_module(module)
}

fn analyze_module(module: &MirModule) -> HashMap<String, FunctionLayout> {
    let mut return_types: HashMap<String, LlvmType> = module
        .functions
        .iter()
        .map(|func| {
            (
                func.name.clone(),
                infer_llvm_type_from_type_id(func.return_ty).unwrap_or_else(LlvmType::default_int),
            )
        })
        .collect();

    let mut layouts = HashMap::new();

    loop {
        let mut changed = false;

        for func in &module.functions {
            let layout = analyze_function(func, &return_types);
            let prev = return_types.insert(func.name.clone(), layout.return_ty);
            changed |= prev != Some(layout.return_ty);
            layouts.insert(func.name.clone(), layout);
        }

        if !changed {
            break;
        }
    }

    layouts
}

fn analyze_function(
    func: &MirFunction,
    return_types: &HashMap<String, LlvmType>,
) -> FunctionLayout {
    let mut layout = FunctionLayout {
        params: func
            .params
            .iter()
            .map(|param| {
                infer_llvm_type_from_type_id(param.ty).unwrap_or_else(LlvmType::default_int)
            })
            .collect(),
        return_ty: infer_llvm_type_from_type_id(func.return_ty)
            .unwrap_or_else(LlvmType::default_int),
        value_types: HashMap::new(),
        value_signs: HashMap::new(),
        local_types: HashMap::new(),
        value_int_flags: HashMap::new(),
    };

    for (index, param) in func.params.iter().enumerate() {
        let ty = layout
            .params
            .get(index)
            .copied()
            .unwrap_or_else(LlvmType::default_int);
        layout.value_types.insert(param.value, ty);
        layout
            .value_signs
            .insert(param.value, default_sign_for_type(ty));
        layout.local_types.insert(param.name.clone(), ty);
    }

    for block in &func.blocks {
        for instr in &block.instructions {
            let inferred = match &instr.op {
                Op::ConstInt(_) => LlvmType::default_int(),
                Op::ConstFloat(_) => LlvmType::Float(LlvmFloatType::F64),
                Op::ConstBool(_) => LlvmType::Bool,
                Op::ConstString(_) => LlvmType::Str,
                Op::Unit => LlvmType::default_int(),
                Op::Copy(value) => value_type(&layout, *value),
                Op::BinOp { op, left, right } => {
                    infer_binop_type(*op, value_type(&layout, *left), value_type(&layout, *right))
                }
                Op::UnOp { op, operand } => infer_unop_type(*op, value_type(&layout, *operand)),
                Op::Call { callee, .. } => {
                    if is_print_builtin(callee) {
                        LlvmType::default_int()
                    } else if let Some(sig) = builtin_signature(callee) {
                        sig.return_ty
                    } else {
                        return_types
                            .get(callee)
                            .copied()
                            .unwrap_or_else(LlvmType::default_int)
                    }
                }
                Op::LoadLocal(name) => layout
                    .local_types
                    .get(name)
                    .copied()
                    .or_else(|| infer_llvm_type_from_type_id(instr.ty))
                    .unwrap_or_else(LlvmType::default_int),
                Op::StoreLocal { name, value } => {
                    let ty = layout
                        .local_types
                        .get(name)
                        .copied()
                        .or_else(|| infer_llvm_type_from_type_id(instr.ty))
                        .unwrap_or_else(|| value_type(&layout, *value));
                    layout.local_types.insert(name.clone(), ty);
                    ty
                }
                Op::Alloca { name, ty } => {
                    let ty = infer_llvm_type_from_type_id(*ty)
                        .or_else(|| layout.local_types.get(name).copied())
                        .unwrap_or_else(LlvmType::default_int);
                    layout.local_types.entry(name.clone()).or_insert(ty);
                    ty
                }
                Op::GetField { object, .. } => value_type(&layout, *object),
                Op::GetIndex { object, .. } => value_type(&layout, *object),
                Op::Phi(entries) => entries
                    .iter()
                    .map(|(_, value)| value_type(&layout, *value))
                    .reduce(merge_type)
                    .unwrap_or_else(LlvmType::default_int),
                Op::Cast { target_ty, value } => infer_llvm_type_from_type_id(*target_ty)
                    .unwrap_or_else(|| value_type(&layout, *value)),
            };

            layout.value_types.insert(instr.result, inferred);
        }
    }

    let mut return_values = Vec::new();
    for block in &func.blocks {
        if let Terminator::Return(value) = &block.terminator {
            return_values.push(value_type(&layout, *value));
        }
    }
    if !return_values.is_empty() {
        layout.return_ty = return_values
            .into_iter()
            .reduce(merge_type)
            .unwrap_or(layout.return_ty);
    }

    let proven_local_signs = infer_proven_local_signs(func);
    let mut local_signs: HashMap<String, SignInfo> = func
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            let ty = layout
                .params
                .get(index)
                .copied()
                .unwrap_or_else(LlvmType::default_int);
            let sign = if let Some(sign) = proven_local_signs.get(&param.name).copied() {
                sign
            } else {
                default_sign_for_type(ty)
            };
            (param.name.clone(), sign)
        })
        .collect();
    for (name, sign) in &proven_local_signs {
        local_signs.entry(name.clone()).or_insert(*sign);
    }
    let mut value_signs = layout.value_signs.clone();

    loop {
        let mut changed = false;

        for block in &func.blocks {
            for instr in &block.instructions {
                let result_ty = value_type(&layout, instr.result);
                let inferred_sign =
                    match &instr.op {
                        Op::ConstInt(val) => int_const_sign(*val),
                        Op::ConstFloat(_) => default_sign_for_type(result_ty),
                        Op::ConstBool(val) => bool_const_sign(*val),
                        Op::ConstString(_) => default_sign_for_type(result_ty),
                        Op::Unit => default_sign_for_type(result_ty),
                        Op::Copy(value) => value_signs
                            .get(value)
                            .copied()
                            .unwrap_or_else(|| default_sign_for_type(value_type(&layout, *value))),
                        Op::BinOp { op, left, right } => infer_binop_sign(
                            *op,
                            result_ty,
                            value_signs.get(left).copied().unwrap_or_else(|| {
                                default_sign_for_type(value_type(&layout, *left))
                            }),
                            value_signs.get(right).copied().unwrap_or_else(|| {
                                default_sign_for_type(value_type(&layout, *right))
                            }),
                        ),
                        Op::UnOp { op, operand } => infer_unop_sign(
                            *op,
                            result_ty,
                            value_signs.get(operand).copied().unwrap_or_else(|| {
                                default_sign_for_type(value_type(&layout, *operand))
                            }),
                        ),
                        Op::Call { callee, .. } => infer_call_sign(callee, result_ty),
                        Op::LoadLocal(name) => local_signs
                            .get(name)
                            .copied()
                            .unwrap_or_else(|| default_sign_for_type(result_ty)),
                        Op::StoreLocal { name, value } => {
                            let stored_sign = if let Some(sign) = proven_local_signs.get(name) {
                                *sign
                            } else {
                                value_signs.get(value).copied().unwrap_or_else(|| {
                                    default_sign_for_type(value_type(&layout, *value))
                                })
                            };
                            let merged = if let Some(existing) = local_signs.get(name).copied() {
                                merge_sign(existing, stored_sign)
                            } else {
                                stored_sign
                            };
                            if local_signs.get(name).copied() != Some(merged) {
                                local_signs.insert(name.clone(), merged);
                                changed = true;
                            }
                            merged
                        }
                        Op::Alloca { name, .. } => {
                            let sign = if let Some(sign) = proven_local_signs.get(name) {
                                *sign
                            } else {
                                default_sign_for_type(
                                    layout
                                        .local_types
                                        .get(name)
                                        .copied()
                                        .unwrap_or_else(LlvmType::default_int),
                                )
                            };
                            if sign != SignInfo::Unknown
                                && local_signs.get(name).copied() != Some(sign)
                            {
                                local_signs.insert(name.clone(), sign);
                                changed = true;
                            }
                            sign
                        }
                        Op::GetField { object, .. } => value_signs
                            .get(object)
                            .copied()
                            .unwrap_or_else(|| default_sign_for_type(value_type(&layout, *object))),
                        Op::GetIndex { object, .. } => value_signs
                            .get(object)
                            .copied()
                            .unwrap_or_else(|| default_sign_for_type(value_type(&layout, *object))),
                        Op::Phi(entries) => entries
                            .iter()
                            .map(|(_, value)| {
                                value_signs.get(value).copied().unwrap_or_else(|| {
                                    default_sign_for_type(value_type(&layout, *value))
                                })
                            })
                            .reduce(merge_sign)
                            .unwrap_or_else(|| default_sign_for_type(result_ty)),
                        Op::Cast { value, .. } => infer_cast_sign(
                            value_type(&layout, *value),
                            result_ty,
                            value_signs.get(value).copied().unwrap_or_else(|| {
                                default_sign_for_type(value_type(&layout, *value))
                            }),
                        ),
                    };

                if value_signs.get(&instr.result).copied() != Some(inferred_sign) {
                    value_signs.insert(instr.result, inferred_sign);
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    layout.value_signs = value_signs;
    layout.value_int_flags = infer_safe_int_arith_flags(func, &layout, &proven_local_signs);

    layout
}

fn infer_llvm_type_from_type_id(type_id: agam_sema::symbol::TypeId) -> Option<LlvmType> {
    match builtin_type_by_id(type_id)? {
        Type::Bool => Some(LlvmType::Bool),
        Type::Str => Some(LlvmType::Str),
        Type::Float(FloatSize::F32) => Some(LlvmType::Float(LlvmFloatType::F32)),
        Type::Float(FloatSize::F64) => Some(LlvmType::Float(LlvmFloatType::F64)),
        Type::Int(size) => Some(LlvmType::Int(LlvmIntType::from_size(size, true))),
        Type::UInt(size) => Some(LlvmType::Int(LlvmIntType::from_size(size, false))),
        _ => None,
    }
}

fn builtin_signature(name: &str) -> Option<BuiltinSig> {
    match name {
        "argc" => Some(BuiltinSig {
            return_ty: LlvmType::default_int(),
        }),
        "argv" => Some(BuiltinSig {
            return_ty: LlvmType::Str,
        }),
        "parse_int" => Some(BuiltinSig {
            return_ty: LlvmType::default_int(),
        }),
        "clock" => Some(BuiltinSig {
            return_ty: LlvmType::Float(LlvmFloatType::F64),
        }),
        "adam" | "dataframe_mean" | "tensor_checksum" => Some(BuiltinSig {
            return_ty: LlvmType::Float(LlvmFloatType::F64),
        }),
        "dataframe_build_sin"
        | "dataframe_filter_gt"
        | "dataframe_sort"
        | "dataframe_group_by"
        | "tensor_fill_rand"
        | "dense_layer"
        | "conv2d" => Some(BuiltinSig {
            return_ty: LlvmType::OpaquePtr,
        }),
        "dataframe_free" | "tensor_free" | "len" | "has_next" | "next" => Some(BuiltinSig {
            return_ty: LlvmType::default_int(),
        }),
        _ => None,
    }
}

fn is_print_builtin(name: &str) -> bool {
    matches!(name, "print" | "println" | "print_int" | "print_str")
}

fn has_runtime_helper_definition(name: &str) -> bool {
    matches!(name, "argc" | "argv" | "parse_int")
}

fn builtin_arg_types(name: &str) -> Option<Vec<LlvmType>> {
    match name {
        "argc" | "clock" => Some(Vec::new()),
        "argv" => Some(vec![LlvmType::default_int()]),
        "parse_int" => Some(vec![LlvmType::Str]),
        "len" | "has_next" | "next" => Some(vec![LlvmType::OpaquePtr]),
        _ => None,
    }
}

fn infer_binop_type(op: MirBinOp, left: LlvmType, right: LlvmType) -> LlvmType {
    if matches!(
        op,
        MirBinOp::Eq
            | MirBinOp::NotEq
            | MirBinOp::Lt
            | MirBinOp::LtEq
            | MirBinOp::Gt
            | MirBinOp::GtEq
            | MirBinOp::And
            | MirBinOp::Or
    ) {
        LlvmType::Bool
    } else if op == MirBinOp::Add && (left == LlvmType::Str || right == LlvmType::Str) {
        LlvmType::Str
    } else if left == LlvmType::OpaquePtr || right == LlvmType::OpaquePtr {
        LlvmType::OpaquePtr
    } else if let Some(float_ty) = common_float_type(left, right) {
        LlvmType::Float(float_ty)
    } else if let Some(int_ty) = common_int_type(left, right) {
        LlvmType::Int(int_ty)
    } else {
        LlvmType::default_int()
    }
}

fn infer_unop_type(op: MirUnOp, operand: LlvmType) -> LlvmType {
    match op {
        MirUnOp::Not => LlvmType::Bool,
        MirUnOp::Neg if operand.float_spec().is_some() => operand,
        _ => operand,
    }
}

fn merge_type(left: LlvmType, right: LlvmType) -> LlvmType {
    if left == right {
        left
    } else if left == LlvmType::OpaquePtr || right == LlvmType::OpaquePtr {
        LlvmType::OpaquePtr
    } else if left == LlvmType::Str || right == LlvmType::Str {
        LlvmType::Str
    } else if let Some(float_ty) = common_float_type(left, right) {
        LlvmType::Float(float_ty)
    } else if let Some(int_ty) = common_int_type(left, right) {
        LlvmType::Int(int_ty)
    } else if left == LlvmType::Bool || right == LlvmType::Bool {
        LlvmType::Bool
    } else {
        LlvmType::default_int()
    }
}

fn value_type(layout: &FunctionLayout, value: ValueId) -> LlvmType {
    layout
        .value_types
        .get(&value)
        .copied()
        .unwrap_or_else(LlvmType::default_int)
}

fn value_sign(layout: &FunctionLayout, value: ValueId) -> SignInfo {
    layout
        .value_signs
        .get(&value)
        .copied()
        .unwrap_or_else(|| default_sign_for_type(value_type(layout, value)))
}

fn infer_call_sign(callee: &str, result_ty: LlvmType) -> SignInfo {
    match callee {
        "argc" | "len" | "has_next" => SignInfo::NonNegative,
        "print" | "println" | "print_int" | "print_str" => SignInfo::NonNegative,
        _ => default_sign_for_type(result_ty),
    }
}

fn infer_binop_sign(
    op: MirBinOp,
    result_ty: LlvmType,
    left: SignInfo,
    right: SignInfo,
) -> SignInfo {
    match op {
        MirBinOp::Eq
        | MirBinOp::NotEq
        | MirBinOp::Lt
        | MirBinOp::LtEq
        | MirBinOp::Gt
        | MirBinOp::GtEq
        | MirBinOp::And
        | MirBinOp::Or => SignInfo::NonNegative,
        MirBinOp::Div | MirBinOp::Mod
            if matches!(result_ty, LlvmType::Int(LlvmIntType { signed: true, .. }))
                && left.is_nonnegative()
                && right.is_nonnegative() =>
        {
            SignInfo::NonNegative
        }
        MirBinOp::Shr
            if matches!(result_ty, LlvmType::Int(LlvmIntType { signed: true, .. }))
                && left.is_nonnegative() =>
        {
            SignInfo::NonNegative
        }
        MirBinOp::BitAnd if left.is_nonnegative() && right.is_nonnegative() => {
            SignInfo::NonNegative
        }
        _ => default_sign_for_type(result_ty),
    }
}

fn infer_unop_sign(op: MirUnOp, result_ty: LlvmType, operand: SignInfo) -> SignInfo {
    match op {
        MirUnOp::Not => SignInfo::NonNegative,
        MirUnOp::BitNot => default_sign_for_type(result_ty),
        MirUnOp::Neg => {
            let _ = operand;
            default_sign_for_type(result_ty)
        }
    }
}

fn infer_cast_sign(source_ty: LlvmType, target_ty: LlvmType, source_sign: SignInfo) -> SignInfo {
    match target_ty {
        LlvmType::Bool => SignInfo::NonNegative,
        LlvmType::Int(target_int) if !target_int.signed => SignInfo::NonNegative,
        LlvmType::Int(target_int) => match source_ty {
            LlvmType::Bool => SignInfo::NonNegative,
            LlvmType::Int(source_int) if source_int.bits <= target_int.bits => source_sign,
            _ => default_sign_for_type(target_ty),
        },
        LlvmType::Float(_) => match source_ty {
            LlvmType::Bool => SignInfo::NonNegative,
            LlvmType::Int(_) | LlvmType::Float(_) => source_sign,
            _ => default_sign_for_type(target_ty),
        },
        _ => default_sign_for_type(target_ty),
    }
}

fn infer_proven_local_signs(func: &MirFunction) -> HashMap<String, SignInfo> {
    let instrs: HashMap<ValueId, &Instruction> = func
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .map(|instr| (instr.result, instr))
        .collect();
    let blocks: HashMap<BlockId, &BasicBlock> =
        func.blocks.iter().map(|block| (block.id, block)).collect();
    let mut stores: HashMap<String, Vec<(BlockId, ValueId)>> = HashMap::new();

    for block in &func.blocks {
        for instr in &block.instructions {
            if let Op::StoreLocal { name, value } = &instr.op {
                stores
                    .entry(name.clone())
                    .or_default()
                    .push((block.id, *value));
            }
        }
    }

    let mut proven = HashMap::new();

    loop {
        let mut changed = false;

        for (name, store_sites) in &stores {
            let mut seed_sign = None;
            let mut all_safe = true;

            for (block_id, value) in store_sites {
                let sign = seed_value_sign(&instrs, *value, &proven);
                if sign != SignInfo::Unknown {
                    seed_sign = Some(match seed_sign {
                        Some(existing) => merge_sign(existing, sign),
                        None => sign,
                    });
                    continue;
                }

                if is_safe_increment_store(&blocks, &instrs, *block_id, name, *value) {
                    continue;
                }

                all_safe = false;
                break;
            }

            if let Some(sign) = seed_sign {
                if all_safe && proven.get(name).copied() != Some(sign) {
                    proven.insert(name.clone(), sign);
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    proven
}

fn seed_value_sign(
    instrs: &HashMap<ValueId, &Instruction>,
    value: ValueId,
    proven_locals: &HashMap<String, SignInfo>,
) -> SignInfo {
    match instrs.get(&value).map(|instr| &instr.op) {
        Some(Op::ConstInt(val)) => int_const_sign(*val),
        Some(Op::ConstBool(val)) => bool_const_sign(*val),
        Some(Op::Call { callee, .. }) => infer_call_sign(callee, LlvmType::default_int()),
        Some(Op::Copy(source)) => seed_value_sign(instrs, *source, proven_locals),
        Some(Op::Cast { value, .. }) => seed_value_sign(instrs, *value, proven_locals),
        Some(Op::LoadLocal(name)) => proven_locals.get(name).copied().unwrap_or_default(),
        _ => SignInfo::Unknown,
    }
}

fn is_safe_increment_store(
    blocks: &HashMap<BlockId, &BasicBlock>,
    instrs: &HashMap<ValueId, &Instruction>,
    block_id: BlockId,
    local_name: &str,
    value: ValueId,
) -> bool {
    let Some(header_id) = (match blocks.get(&block_id).map(|block| &block.terminator) {
        Some(Terminator::Jump(target)) => Some(*target),
        _ => None,
    }) else {
        return false;
    };

    matches_increment_by_one(instrs, local_name, value)
        && classify_increment_guard(blocks, instrs, header_id, local_name).is_some()
}

fn matches_increment_by_one(
    instrs: &HashMap<ValueId, &Instruction>,
    local_name: &str,
    value: ValueId,
) -> bool {
    increment_add_value(instrs, local_name, value).is_some()
}

fn increment_add_value(
    instrs: &HashMap<ValueId, &Instruction>,
    local_name: &str,
    value: ValueId,
) -> Option<ValueId> {
    match instrs.get(&value).map(|instr| &instr.op) {
        Some(Op::BinOp {
            op: MirBinOp::Add,
            left,
            right,
        }) => {
            if (load_local_name(instrs, *left).as_deref() == Some(local_name)
                && const_int_value(instrs, *right) == Some(1))
                || (load_local_name(instrs, *right).as_deref() == Some(local_name)
                    && const_int_value(instrs, *left) == Some(1))
            {
                Some(value)
            } else {
                None
            }
        }
        Some(Op::Copy(source)) => increment_add_value(instrs, local_name, *source),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IncrementGuardKind {
    StrictUpper,
    SquareUpper,
}

fn classify_increment_guard(
    blocks: &HashMap<BlockId, &BasicBlock>,
    instrs: &HashMap<ValueId, &Instruction>,
    header_id: BlockId,
    local_name: &str,
) -> Option<IncrementGuardKind> {
    let Some(block) = blocks.get(&header_id) else {
        return None;
    };
    let Terminator::Branch { condition, .. } = &block.terminator else {
        return None;
    };

    match instrs.get(condition).map(|instr| &instr.op) {
        Some(Op::BinOp {
            op: MirBinOp::Lt,
            left,
            right: _,
        }) if load_local_name(instrs, *left).as_deref() == Some(local_name) => {
            Some(IncrementGuardKind::StrictUpper)
        }
        Some(Op::BinOp {
            op: MirBinOp::Gt,
            left: _,
            right,
        }) if load_local_name(instrs, *right).as_deref() == Some(local_name) => {
            Some(IncrementGuardKind::StrictUpper)
        }
        Some(Op::BinOp {
            op: MirBinOp::LtEq,
            left,
            right: _,
        }) if is_square_of_local(instrs, *left, local_name) => Some(IncrementGuardKind::SquareUpper),
        Some(Op::BinOp {
            op: MirBinOp::GtEq,
            left: _,
            right,
        }) if is_square_of_local(instrs, *right, local_name) => Some(IncrementGuardKind::SquareUpper),
        _ => None,
    }
}

fn is_square_of_local(
    instrs: &HashMap<ValueId, &Instruction>,
    value: ValueId,
    local_name: &str,
) -> bool {
    match instrs.get(&value).map(|instr| &instr.op) {
        Some(Op::BinOp {
            op: MirBinOp::Mul,
            left,
            right,
        }) => {
            load_local_name(instrs, *left).as_deref() == Some(local_name)
                && load_local_name(instrs, *right).as_deref() == Some(local_name)
        }
        Some(Op::Copy(source)) => is_square_of_local(instrs, *source, local_name),
        _ => false,
    }
}

fn load_local_name(instrs: &HashMap<ValueId, &Instruction>, value: ValueId) -> Option<String> {
    match instrs.get(&value).map(|instr| &instr.op) {
        Some(Op::LoadLocal(name)) => Some(name.clone()),
        Some(Op::Copy(source)) => load_local_name(instrs, *source),
        _ => None,
    }
}

fn const_int_value(instrs: &HashMap<ValueId, &Instruction>, value: ValueId) -> Option<i64> {
    match instrs.get(&value).map(|instr| &instr.op) {
        Some(Op::ConstInt(val)) => Some(*val),
        Some(Op::Copy(source)) => const_int_value(instrs, *source),
        _ => None,
    }
}

fn infer_safe_int_arith_flags(
    func: &MirFunction,
    layout: &FunctionLayout,
    proven_local_signs: &HashMap<String, SignInfo>,
) -> HashMap<ValueId, IntArithFlags> {
    let instrs: HashMap<ValueId, &Instruction> = func
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .map(|instr| (instr.result, instr))
        .collect();
    let blocks: HashMap<BlockId, &BasicBlock> =
        func.blocks.iter().map(|block| (block.id, block)).collect();
    let mut flags = HashMap::new();

    for block in &func.blocks {
        for instr in &block.instructions {
            let Op::StoreLocal { name, value } = &instr.op else {
                continue;
            };
            let Some(add_value) = increment_add_value(&instrs, name, *value) else {
                continue;
            };
            let Some(IncrementGuardKind::StrictUpper) =
                (match blocks.get(&block.id).map(|block| &block.terminator) {
                    Some(Terminator::Jump(target)) => {
                        classify_increment_guard(&blocks, &instrs, *target, name)
                    }
                    _ => None,
                })
            else {
                continue;
            };

            let local_ty = layout
                .local_types
                .get(name)
                .copied()
                .unwrap_or_else(LlvmType::default_int);
            let Some(int_ty) = local_ty.int_spec() else {
                continue;
            };
            let local_sign = proven_local_signs
                .get(name)
                .copied()
                .unwrap_or_else(|| default_sign_for_type(local_ty));
            if !local_sign.is_nonnegative() {
                continue;
            }

            flags.insert(
                add_value,
                IntArithFlags {
                    nuw: true,
                    nsw: int_ty.signed,
                },
            );
        }
    }

    flags
}

fn common_int_type(left: LlvmType, right: LlvmType) -> Option<LlvmIntType> {
    match (left.int_spec(), right.int_spec()) {
        (Some(left), Some(right)) => Some(LlvmIntType {
            bits: left.bits.max(right.bits),
            signed: left.signed || right.signed,
        }),
        (Some(int_ty), None) | (None, Some(int_ty)) => Some(int_ty),
        _ => None,
    }
}

fn common_float_type(left: LlvmType, right: LlvmType) -> Option<LlvmFloatType> {
    match (left.float_spec(), right.float_spec()) {
        (Some(LlvmFloatType::F64), _) | (_, Some(LlvmFloatType::F64)) => Some(LlvmFloatType::F64),
        (Some(LlvmFloatType::F32), Some(LlvmFloatType::F32)) => Some(LlvmFloatType::F32),
        (Some(float_ty), None) | (None, Some(float_ty)) => Some(float_ty),
        _ => None,
    }
}

struct LlvmEmitter {
    options: LlvmEmitOptions,
    layouts: HashMap<String, FunctionLayout>,
    function_attrs: HashMap<String, FunctionAttrs>,
    user_functions: HashSet<String>,
    external_decls: BTreeMap<String, String>,
    globals: Vec<GlobalString>,
    string_pool: HashMap<Vec<u8>, String>,
    next_string_id: usize,
    next_temp_id: usize,
}

impl LlvmEmitter {
    fn new(
        module: &MirModule,
        layouts: HashMap<String, FunctionLayout>,
        options: LlvmEmitOptions,
    ) -> Self {
        let function_attrs = analyze_function_attrs(module, &layouts);
        Self {
            options,
            layouts,
            function_attrs,
            user_functions: module
                .functions
                .iter()
                .map(|func| func.name.clone())
                .collect(),
            external_decls: BTreeMap::new(),
            globals: Vec::new(),
            string_pool: HashMap::new(),
            next_string_id: 0,
            next_temp_id: 0,
        }
    }

    fn emit_module(&mut self, module: &MirModule) -> Result<String, String> {
        let mut functions = String::new();
        for func in &module.functions {
            let layout = self
                .layouts
                .get(&func.name)
                .cloned()
                .ok_or_else(|| format!("missing LLVM layout for `{}`", func.name))?;
            let body = self.emit_function(func, &layout)?;
            functions.push_str(&body);
            functions.push('\n');
        }

        let mut output = String::new();
        writeln!(output, "; Generated by agamc — Agam Compiler").unwrap();
        if let Some(data_layout) = &self.options.data_layout {
            writeln!(
                output,
                "target datalayout = \"{}\"",
                escape_llvm_attr_value(data_layout)
            )
            .unwrap();
        }
        if let Some(target_triple) = &self.options.target_triple {
            writeln!(
                output,
                "target triple = \"{}\"",
                escape_llvm_attr_value(target_triple)
            )
            .unwrap();
        }
        let default_int_ir = LlvmType::default_int().ir();
        writeln!(
            output,
            "@agam_argc_storage = internal local_unnamed_addr global {} 0",
            default_int_ir
        )
        .unwrap();
        writeln!(
            output,
            "@agam_argv_storage = internal local_unnamed_addr global i8** null"
        )
        .unwrap();
        writeln!(
            output,
            "declare noundef i32 @printf(i8* nocapture noundef readonly, ...) local_unnamed_addr #0"
        )
        .unwrap();
        writeln!(
            output,
            "declare noundef i64 @strtoll(i8* nocapture noundef readonly, i8** nocapture, i32 noundef) local_unnamed_addr #1"
        )
        .unwrap();
        for decl in self.external_decls.values() {
            writeln!(output, "{decl}").unwrap();
        }
        if !self.globals.is_empty() {
            writeln!(output).unwrap();
            for global in &self.globals {
                let len = global.bytes.len();
                let bytes = escape_llvm_bytes(&global.bytes);
                writeln!(
                    output,
                    "{} = private unnamed_addr constant [{} x i8] c\"{}\"",
                    global.name, len, bytes
                )
                .unwrap();
            }
        }
        writeln!(output).unwrap();
        output.push_str(
            "define noundef i32 @agam_argc() local_unnamed_addr #2 {\n\
             entry:\n\
               %0 = load i32, i32* @agam_argc_storage\n\
               ret i32 %0\n\
             }\n\n\
             define noundef i8* @agam_argv(i32 noundef %index) local_unnamed_addr #3 {\n\
             entry:\n\
               %0 = load i8**, i8*** @agam_argv_storage\n\
               %1 = sext i32 %index to i64\n\
               %2 = getelementptr inbounds i8*, i8** %0, i64 %1\n\
               %3 = load i8*, i8** %2\n\
               ret i8* %3\n\
             }\n\n\
             define noundef i32 @agam_parse_int(i8* nocapture noundef readonly %s) local_unnamed_addr #1 {\n\
             entry:\n\
               %0 = call i64 @strtoll(i8* noundef %s, i8** null, i32 noundef 10)\n\
               %1 = trunc i64 %0 to i32\n\
               ret i32 %1\n\
             }\n\n",
        );
        output.push_str(&functions);
        output.push_str(
            "attributes #0 = { nofree nounwind }\n\
             attributes #1 = { nofree nounwind willreturn }\n\
             attributes #2 = { nofree norecurse nosync nounwind willreturn }\n\
             attributes #3 = { nofree norecurse nosync nounwind willreturn }\n",
        );
        Ok(output)
    }

    fn emit_function(
        &mut self,
        func: &MirFunction,
        layout: &FunctionLayout,
    ) -> Result<String, String> {
        let mut out = String::new();
        let mut values: HashMap<ValueId, ValueRef> = HashMap::new();
        let mut locals: HashMap<String, (LlvmType, String)> = HashMap::new();
        let mut emitted_locals = HashSet::new();
        let attrs = self
            .function_attrs
            .get(&func.name)
            .copied()
            .unwrap_or_default();
        let fn_attr_suffix = format_function_attrs(attrs);

        if func.name == "main" {
            writeln!(
                out,
                "define noundef i32 @main(i32 noundef %argc, i8** noundef %argv) local_unnamed_addr{} {{",
                fn_attr_suffix
            )
            .unwrap();
        } else {
            write!(
                out,
                "define noundef {} @{}(",
                layout.return_ty.ir(),
                mangle_name(&func.name)
            )
            .unwrap();
            for (i, _param) in func.params.iter().enumerate() {
                if i > 0 {
                    write!(out, ", ").unwrap();
                }
                let ty = layout
                    .params
                    .get(i)
                    .copied()
                    .unwrap_or_else(LlvmType::default_int);
                write!(out, "{} noundef %p{}", ty.ir(), i).unwrap();
            }
            writeln!(out, ") local_unnamed_addr{} {{", fn_attr_suffix).unwrap();
        }

        for block in &func.blocks {
            writeln!(out, "block_{}:", block.id.0).unwrap();

            if block.id == func.entry {
                if func.name == "main" {
                    writeln!(out, "  store i32 %argc, i32* @agam_argc_storage").unwrap();
                    writeln!(out, "  store i8** %argv, i8*** @agam_argv_storage").unwrap();
                }
                for (i, param) in func.params.iter().enumerate() {
                    let ty = layout
                        .params
                        .get(i)
                        .copied()
                        .unwrap_or_else(LlvmType::default_int);
                    let local_name = format!("%local_{}", sanitize_name(&param.name));
                    writeln!(out, "  {} = alloca {}", local_name, ty.ir()).unwrap();
                    writeln!(
                        out,
                        "  store {} %p{}, {}* {}",
                        ty.ir(),
                        i,
                        ty.ir(),
                        local_name
                    )
                    .unwrap();
                    locals.insert(param.name.clone(), (ty, local_name.clone()));
                    emitted_locals.insert(param.name.clone());
                    values.insert(
                        param.value,
                        ValueRef::new(ty, format!("%p{}", i), value_sign(layout, param.value)),
                    );
                }
            }

            for instr in &block.instructions {
                self.emit_instruction(
                    &mut out,
                    instr,
                    layout,
                    &mut values,
                    &mut locals,
                    &mut emitted_locals,
                )?;
            }

            self.emit_terminator(
                &mut out,
                &block.terminator,
                layout,
                func.name == "main",
                &values,
            )?;
        }

        writeln!(out, "}}").unwrap();
        Ok(out)
    }

    fn emit_instruction(
        &mut self,
        out: &mut String,
        instr: &Instruction,
        layout: &FunctionLayout,
        values: &mut HashMap<ValueId, ValueRef>,
        locals: &mut HashMap<String, (LlvmType, String)>,
        emitted_locals: &mut HashSet<String>,
    ) -> Result<(), String> {
        let result_name = format!("%v{}", instr.result.0);
        let result_ty = value_type(layout, instr.result);
        let result_sign = value_sign(layout, instr.result);

        match &instr.op {
            Op::ConstInt(val) => {
                values.insert(
                    instr.result,
                    ValueRef::new(result_ty, val.to_string(), result_sign),
                );
            }
            Op::ConstFloat(val) => {
                values.insert(
                    instr.result,
                    ValueRef::new(result_ty, format!("{val}"), result_sign),
                );
            }
            Op::ConstBool(val) => {
                values.insert(
                    instr.result,
                    ValueRef::new(result_ty, if *val { "true" } else { "false" }, result_sign),
                );
            }
            Op::ConstString(val) => {
                let repr = self.intern_string_constant(val);
                values.insert(instr.result, ValueRef::new(result_ty, repr, result_sign));
            }
            Op::Unit => {
                values.insert(
                    instr.result,
                    ValueRef::new(result_ty, result_ty.default_value(), result_sign),
                );
            }
            Op::Copy(value) => {
                let source = get_value(values, *value)?;
                values.insert(instr.result, source);
            }
            Op::Alloca { name, .. } => {
                let local_ty = layout
                    .local_types
                    .get(name)
                    .copied()
                    .unwrap_or_else(LlvmType::default_int);
                if !emitted_locals.contains(name) {
                    let ptr_name = format!("%local_{}", sanitize_name(name));
                    writeln!(out, "  {} = alloca {}", ptr_name, local_ty.ir()).unwrap();
                    locals.insert(name.clone(), (local_ty, ptr_name));
                    emitted_locals.insert(name.clone());
                }
                values.insert(
                    instr.result,
                    ValueRef::new(result_ty, result_ty.default_value(), result_sign),
                );
            }
            Op::StoreLocal { name, value } => {
                let stored = get_value(values, *value)?;
                let (local_ty, ptr_name) = locals
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("store to undeclared local `{name}` in LLVM emitter"))?;
                let casted = self.coerce_value(out, &stored, local_ty)?;
                writeln!(
                    out,
                    "  store {} {}, {}* {}",
                    local_ty.ir(),
                    casted.repr,
                    local_ty.ir(),
                    ptr_name
                )
                .unwrap();
                values.insert(
                    instr.result,
                    ValueRef::new(local_ty, casted.repr, result_sign),
                );
            }
            Op::LoadLocal(name) => {
                let (local_ty, ptr_name) = locals.get(name).cloned().ok_or_else(|| {
                    format!("load from undeclared local `{name}` in LLVM emitter")
                })?;
                writeln!(
                    out,
                    "  {} = load {}, {}* {}",
                    result_name,
                    local_ty.ir(),
                    local_ty.ir(),
                    ptr_name
                )
                .unwrap();
                values.insert(
                    instr.result,
                    ValueRef::new(local_ty, result_name.clone(), result_sign),
                );
            }
            Op::BinOp { op, left, right } => {
                let lhs = get_value(values, *left)?;
                let rhs = get_value(values, *right)?;
                if *op == MirBinOp::Add && result_ty == LlvmType::Str {
                    self.register_external_decl(
                        "agam_str_concat",
                        "declare i8* @agam_str_concat(i8*, i8*)",
                    );
                    let lhs = self.coerce_value(out, &lhs, LlvmType::Str)?;
                    let rhs = self.coerce_value(out, &rhs, LlvmType::Str)?;
                    writeln!(
                        out,
                        "  {} = call i8* @agam_str_concat(i8* {}, i8* {})",
                        result_name, lhs.repr, rhs.repr
                    )
                    .unwrap();
                    values.insert(
                        instr.result,
                        ValueRef::new(LlvmType::Str, result_name.clone(), result_sign),
                    );
                } else {
                    let emitted = self.emit_binop(
                        out,
                        instr.result,
                        *op,
                        result_ty,
                        &result_name,
                        &lhs,
                        &rhs,
                        layout,
                    )?;
                    values.insert(instr.result, emitted);
                }
            }
            Op::UnOp { op, operand } => {
                let value = get_value(values, *operand)?;
                let emitted = self.emit_unop(out, *op, result_ty, &result_name, &value)?;
                values.insert(instr.result, emitted);
            }
            Op::Call { callee, args } => {
                let arg_values: Vec<ValueRef> = args
                    .iter()
                    .map(|arg| get_value(values, *arg))
                    .collect::<Result<_, _>>()?;
                let expected_params = self
                    .layouts
                    .get(callee)
                    .map(|layout| layout.params.clone())
                    .or_else(|| builtin_arg_types(callee));
                let coerced_args: Vec<ValueRef> = if let Some(expected_params) = &expected_params {
                    arg_values
                        .iter()
                        .enumerate()
                        .map(|(index, arg)| {
                            if let Some(expected_ty) = expected_params.get(index).copied() {
                                self.coerce_value(out, arg, expected_ty)
                            } else {
                                Ok(arg.clone())
                            }
                        })
                        .collect::<Result<_, _>>()?
                } else {
                    arg_values.clone()
                };

                if is_print_builtin(callee) {
                    self.emit_print_call(out, &coerced_args)?;
                    values.insert(
                        instr.result,
                        ValueRef::new(LlvmType::default_int(), "0", result_sign),
                    );
                } else {
                    let symbol = mangle_name(callee);
                    let arg_list = coerced_args
                        .iter()
                        .map(|arg| format!("{} {}", arg.ty.ir(), arg.repr))
                        .collect::<Vec<_>>()
                        .join(", ");
                    if !self.user_functions.contains(callee)
                        && !has_runtime_helper_definition(callee)
                    {
                        let decl = format!(
                            "declare noundef {} @{}({})",
                            result_ty.ir(),
                            symbol,
                            coerced_args
                                .iter()
                                .map(|arg| format!("{} noundef", arg.ty.ir()))
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        self.register_external_decl(&symbol, &decl);
                    }
                    writeln!(
                        out,
                        "  {} = call {} @{}({})",
                        result_name,
                        format!("noundef {}", result_ty.ir()),
                        symbol,
                        arg_list
                    )
                    .unwrap();
                    values.insert(
                        instr.result,
                        ValueRef::new(result_ty, result_name.clone(), result_sign),
                    );
                }
            }
            Op::Cast { value, target_ty } => {
                let source = get_value(values, *value)?;
                let target = infer_llvm_type_from_type_id(*target_ty).unwrap_or(result_ty);
                let casted = self.coerce_value(out, &source, target)?;
                values.insert(instr.result, casted);
            }
            Op::GetField { field, .. } => {
                return Err(format!(
                    "LLVM backend does not yet support field access for `.{field}`"
                ));
            }
            Op::GetIndex { .. } => {
                return Err("LLVM backend does not yet support indexed aggregate access".into());
            }
            Op::Phi(_) => {
                return Err("LLVM backend does not yet support MIR phi nodes".into());
            }
        }

        Ok(())
    }

    fn emit_binop(
        &mut self,
        out: &mut String,
        result_id: ValueId,
        op: MirBinOp,
        result_ty: LlvmType,
        result_name: &str,
        lhs: &ValueRef,
        rhs: &ValueRef,
        layout: &FunctionLayout,
    ) -> Result<ValueRef, String> {
        if matches!(op, MirBinOp::And | MirBinOp::Or) {
            let lhs = self.coerce_value(out, lhs, LlvmType::Bool)?;
            let rhs = self.coerce_value(out, rhs, LlvmType::Bool)?;
            let opcode = match op {
                MirBinOp::And => "and",
                MirBinOp::Or => "or",
                _ => unreachable!(),
            };
            writeln!(
                out,
                "  {} = {} {} {}, {}",
                result_name,
                opcode,
                LlvmType::Bool.ir(),
                lhs.repr,
                rhs.repr
            )
            .unwrap();
            return Ok(ValueRef::new(
                LlvmType::Bool,
                result_name,
                SignInfo::NonNegative,
            ));
        }

        if matches!(
            op,
            MirBinOp::Eq
                | MirBinOp::NotEq
                | MirBinOp::Lt
                | MirBinOp::LtEq
                | MirBinOp::Gt
                | MirBinOp::GtEq
        ) {
            if let Some(float_ty) = common_float_type(lhs.ty, rhs.ty) {
                let target_ty = LlvmType::Float(float_ty);
                let lhs = self.coerce_value(out, lhs, target_ty)?;
                let rhs = self.coerce_value(out, rhs, target_ty)?;
                let opcode = match op {
                    MirBinOp::Eq => "fcmp oeq",
                    MirBinOp::NotEq => "fcmp one",
                    MirBinOp::Lt => "fcmp olt",
                    MirBinOp::LtEq => "fcmp ole",
                    MirBinOp::Gt => "fcmp ogt",
                    MirBinOp::GtEq => "fcmp oge",
                    _ => unreachable!(),
                };
                writeln!(
                    out,
                    "  {} = {} {} {}, {}",
                    result_name,
                    opcode,
                    target_ty.ir(),
                    lhs.repr,
                    rhs.repr
                )
                .unwrap();
                return Ok(ValueRef::new(
                    LlvmType::Bool,
                    result_name,
                    SignInfo::NonNegative,
                ));
            }

            if lhs.ty == LlvmType::Bool && rhs.ty == LlvmType::Bool {
                let lhs = self.coerce_value(out, lhs, LlvmType::Bool)?;
                let rhs = self.coerce_value(out, rhs, LlvmType::Bool)?;
                let opcode = match op {
                    MirBinOp::Eq => "icmp eq",
                    MirBinOp::NotEq => "icmp ne",
                    MirBinOp::Lt => "icmp ult",
                    MirBinOp::LtEq => "icmp ule",
                    MirBinOp::Gt => "icmp ugt",
                    MirBinOp::GtEq => "icmp uge",
                    _ => unreachable!(),
                };
                writeln!(
                    out,
                    "  {} = {} {} {}, {}",
                    result_name,
                    opcode,
                    LlvmType::Bool.ir(),
                    lhs.repr,
                    rhs.repr
                )
                .unwrap();
                return Ok(ValueRef::new(
                    LlvmType::Bool,
                    result_name,
                    SignInfo::NonNegative,
                ));
            }

            let int_ty = common_int_type(lhs.ty, rhs.ty)
                .or_else(|| result_ty.int_spec())
                .unwrap_or_else(|| LlvmType::default_int().int_spec().unwrap());
            let target_ty = LlvmType::Int(int_ty);
            let lhs = self.coerce_value(out, lhs, target_ty)?;
            let rhs = self.coerce_value(out, rhs, target_ty)?;
            let use_unsigned_compare =
                !int_ty.signed || (lhs.sign.is_nonnegative() && rhs.sign.is_nonnegative());
            let opcode = match op {
                MirBinOp::Eq => "icmp eq",
                MirBinOp::NotEq => "icmp ne",
                MirBinOp::Lt => {
                    if use_unsigned_compare {
                        "icmp ult"
                    } else {
                        "icmp slt"
                    }
                }
                MirBinOp::LtEq => {
                    if use_unsigned_compare {
                        "icmp ule"
                    } else {
                        "icmp sle"
                    }
                }
                MirBinOp::Gt => {
                    if use_unsigned_compare {
                        "icmp ugt"
                    } else {
                        "icmp sgt"
                    }
                }
                MirBinOp::GtEq => {
                    if use_unsigned_compare {
                        "icmp uge"
                    } else {
                        "icmp sge"
                    }
                }
                _ => unreachable!(),
            };
            writeln!(
                out,
                "  {} = {} {} {}, {}",
                result_name,
                opcode,
                target_ty.ir(),
                lhs.repr,
                rhs.repr
            )
            .unwrap();
            return Ok(ValueRef::new(
                LlvmType::Bool,
                result_name,
                SignInfo::NonNegative,
            ));
        }

        if let Some(float_ty) = result_ty
            .float_spec()
            .or_else(|| common_float_type(lhs.ty, rhs.ty))
        {
            let target_ty = LlvmType::Float(float_ty);
            let lhs = self.coerce_value(out, lhs, target_ty)?;
            let rhs = self.coerce_value(out, rhs, target_ty)?;
            let opcode = match op {
                MirBinOp::Add => "fadd",
                MirBinOp::Sub => "fsub",
                MirBinOp::Mul => "fmul",
                MirBinOp::Div => "fdiv",
                _ => {
                    return Err(format!(
                        "unsupported float binary op `{op:?}` in LLVM emitter"
                    ));
                }
            };
            writeln!(
                out,
                "  {} = {} {} {}, {}",
                result_name,
                opcode,
                target_ty.ir(),
                lhs.repr,
                rhs.repr
            )
            .unwrap();
            return Ok(ValueRef::new(
                target_ty,
                result_name,
                infer_binop_sign(op, target_ty, lhs.sign, rhs.sign),
            ));
        }

        let int_ty = result_ty
            .int_spec()
            .or_else(|| common_int_type(lhs.ty, rhs.ty))
            .unwrap_or_else(|| LlvmType::default_int().int_spec().unwrap());
        let target_ty = LlvmType::Int(int_ty);
        let lhs = self.coerce_value(out, lhs, target_ty)?;
        let rhs = self.coerce_value(out, rhs, target_ty)?;
        let use_unsigned_math =
            !int_ty.signed || (lhs.sign.is_nonnegative() && rhs.sign.is_nonnegative());
        let opcode = match op {
            MirBinOp::Add => "add",
            MirBinOp::Sub => "sub",
            MirBinOp::Mul => "mul",
            MirBinOp::Div => {
                if use_unsigned_math {
                    "udiv"
                } else {
                    "sdiv"
                }
            }
            MirBinOp::Mod => {
                if use_unsigned_math {
                    "urem"
                } else {
                    "srem"
                }
            }
            MirBinOp::BitAnd => "and",
            MirBinOp::BitOr => "or",
            MirBinOp::BitXor => "xor",
            MirBinOp::Shl => "shl",
            MirBinOp::Shr => {
                if !int_ty.signed || lhs.sign.is_nonnegative() {
                    "lshr"
                } else {
                    "ashr"
                }
            }
            _ => {
                return Err(format!(
                    "unsupported integer binary op `{op:?}` in LLVM emitter"
                ));
            }
        };
        let flags = if matches!(op, MirBinOp::Add | MirBinOp::Sub | MirBinOp::Mul | MirBinOp::Shl)
        {
            format_int_arith_flags(
                layout
                    .value_int_flags
                    .get(&result_id)
                    .copied()
                    .unwrap_or_default(),
            )
        } else {
            String::new()
        };
        writeln!(
            out,
            "  {} = {}{} {} {}, {}",
            result_name,
            opcode,
            flags,
            target_ty.ir(),
            lhs.repr,
            rhs.repr
        )
        .unwrap();
        let final_ty = if result_ty == LlvmType::OpaquePtr {
            LlvmType::OpaquePtr
        } else {
            target_ty
        };
        Ok(ValueRef::new(
            final_ty,
            result_name,
            infer_binop_sign(op, final_ty, lhs.sign, rhs.sign),
        ))
    }

    fn emit_unop(
        &mut self,
        out: &mut String,
        op: MirUnOp,
        result_ty: LlvmType,
        result_name: &str,
        value: &ValueRef,
    ) -> Result<ValueRef, String> {
        match op {
            MirUnOp::Neg if result_ty.float_spec().is_some() => {
                let target_ty = result_ty;
                let value = self.coerce_value(out, value, target_ty)?;
                writeln!(
                    out,
                    "  {} = fsub {} -0.0, {}",
                    result_name,
                    target_ty.ir(),
                    value.repr
                )
                .unwrap();
                Ok(ValueRef::new(
                    target_ty,
                    result_name,
                    infer_unop_sign(op, target_ty, value.sign),
                ))
            }
            MirUnOp::Neg => {
                let target_ty = result_ty
                    .int_spec()
                    .map(LlvmType::Int)
                    .or_else(|| value.ty.int_spec().map(LlvmType::Int))
                    .unwrap_or_else(LlvmType::default_int);
                let value = self.coerce_value(out, value, target_ty)?;
                writeln!(
                    out,
                    "  {} = sub {} 0, {}",
                    result_name,
                    target_ty.ir(),
                    value.repr
                )
                .unwrap();
                Ok(ValueRef::new(
                    target_ty,
                    result_name,
                    infer_unop_sign(op, target_ty, value.sign),
                ))
            }
            MirUnOp::Not => {
                let value = self.coerce_value(out, value, LlvmType::Bool)?;
                writeln!(
                    out,
                    "  {} = xor {} {}, true",
                    result_name,
                    LlvmType::Bool.ir(),
                    value.repr
                )
                .unwrap();
                Ok(ValueRef::new(
                    LlvmType::Bool,
                    result_name,
                    SignInfo::NonNegative,
                ))
            }
            MirUnOp::BitNot => {
                let target_ty = result_ty
                    .int_spec()
                    .map(LlvmType::Int)
                    .or_else(|| value.ty.int_spec().map(LlvmType::Int))
                    .unwrap_or_else(LlvmType::default_int);
                let value = self.coerce_value(out, value, target_ty)?;
                writeln!(
                    out,
                    "  {} = xor {} {}, -1",
                    result_name,
                    target_ty.ir(),
                    value.repr
                )
                .unwrap();
                Ok(ValueRef::new(
                    target_ty,
                    result_name,
                    infer_unop_sign(op, target_ty, value.sign),
                ))
            }
        }
    }

    fn emit_print_call(&mut self, out: &mut String, args: &[ValueRef]) -> Result<(), String> {
        let mut format = String::new();
        let mut call_args = Vec::new();

        if args.is_empty() {
            format.push('\n');
        } else {
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    format.push(' ');
                }
                match arg.ty {
                    LlvmType::Str | LlvmType::OpaquePtr => {
                        format.push_str("%s");
                        call_args.push(format!("i8* {}", arg.repr));
                    }
                    LlvmType::Float(float_ty) => {
                        let promoted = if float_ty == LlvmFloatType::F32 {
                            self.coerce_value(out, arg, LlvmType::Float(LlvmFloatType::F64))?
                        } else {
                            arg.clone()
                        };
                        format.push_str("%.17g");
                        call_args.push(format!("double {}", promoted.repr));
                    }
                    LlvmType::Bool => {
                        format.push_str("%s");
                        let arg = self.bool_to_string(out, arg)?;
                        call_args.push(format!("i8* {}", arg.repr));
                    }
                    LlvmType::Int(int_ty) => {
                        if int_ty.bits > 64 {
                            return Err(
                                "LLVM backend does not yet support printing integers wider than 64 bits"
                                    .into(),
                            );
                        }
                        let promoted = self.coerce_value(
                            out,
                            arg,
                            LlvmType::Int(LlvmIntType {
                                bits: 64,
                                signed: int_ty.signed,
                            }),
                        )?;
                        format.push_str(if int_ty.signed { "%lld" } else { "%llu" });
                        call_args.push(format!("i64 {}", promoted.repr));
                    }
                }
            }
            format.push('\n');
        }

        let fmt = self.intern_string_constant(&format);
        let mut params = vec![format!("i8* {fmt}")];
        params.extend(call_args);
        writeln!(
            out,
            "  %tmp{} = call i32 (i8*, ...) @printf({})",
            self.fresh_temp_id(),
            params.join(", ")
        )
        .unwrap();
        Ok(())
    }

    fn emit_terminator(
        &mut self,
        out: &mut String,
        term: &Terminator,
        layout: &FunctionLayout,
        is_main: bool,
        values: &HashMap<ValueId, ValueRef>,
    ) -> Result<(), String> {
        match term {
            Terminator::Return(value) => {
                let returned = get_value(values, *value)?;
                if is_main {
                    let returned = self.coerce_value(out, &returned, LlvmType::default_int())?;
                    writeln!(out, "  ret i32 {}", returned.repr).unwrap();
                } else {
                    let returned = self.coerce_value(out, &returned, layout.return_ty)?;
                    writeln!(out, "  ret {} {}", layout.return_ty.ir(), returned.repr).unwrap();
                }
            }
            Terminator::ReturnVoid => {
                if is_main {
                    writeln!(out, "  ret i32 0").unwrap();
                } else {
                    writeln!(
                        out,
                        "  ret {} {}",
                        layout.return_ty.ir(),
                        layout.return_ty.default_value()
                    )
                    .unwrap();
                }
            }
            Terminator::Jump(block) => {
                writeln!(out, "  br label %block_{}", block.0).unwrap();
            }
            Terminator::Branch {
                condition,
                then_block,
                else_block,
            } => {
                let cond = get_value(values, *condition)?;
                let cond = self.coerce_value(out, &cond, LlvmType::Bool)?;
                writeln!(
                    out,
                    "  br i1 {}, label %block_{}, label %block_{}",
                    cond.repr, then_block.0, else_block.0
                )
                .unwrap();
            }
            Terminator::Unreachable => {
                writeln!(out, "  unreachable").unwrap();
            }
        }
        Ok(())
    }

    fn bool_to_string(&mut self, out: &mut String, value: &ValueRef) -> Result<ValueRef, String> {
        let value = self.coerce_value(out, value, LlvmType::Bool)?;
        let true_str = self.intern_string_constant("true");
        let false_str = self.intern_string_constant("false");
        let temp = format!("%tmp{}", self.fresh_temp_id());
        writeln!(
            out,
            "  {} = select i1 {}, i8* {}, i8* {}",
            temp, value.repr, true_str, false_str
        )
        .unwrap();
        Ok(ValueRef::new(
            LlvmType::Str,
            temp,
            default_sign_for_type(LlvmType::Str),
        ))
    }

    fn coerce_value(
        &mut self,
        out: &mut String,
        value: &ValueRef,
        target: LlvmType,
    ) -> Result<ValueRef, String> {
        if value.ty == target {
            return Ok(value.clone());
        }

        let temp = format!("%tmp{}", self.fresh_temp_id());
        match (value.ty, target) {
            (LlvmType::Bool, LlvmType::Int(int_ty)) => {
                writeln!(
                    out,
                    "  {} = zext i1 {} to {}",
                    temp,
                    value.repr,
                    int_ty.ir()
                )
                .unwrap();
            }
            (LlvmType::Int(int_ty), LlvmType::Bool) => {
                writeln!(
                    out,
                    "  {} = icmp ne {} {}, 0",
                    temp,
                    int_ty.ir(),
                    value.repr
                )
                .unwrap();
            }
            (LlvmType::Int(int_ty), LlvmType::Float(float_ty)) => {
                let opcode = if int_ty.signed { "sitofp" } else { "uitofp" };
                writeln!(
                    out,
                    "  {} = {} {} {} to {}",
                    temp,
                    opcode,
                    int_ty.ir(),
                    value.repr,
                    float_ty.ir()
                )
                .unwrap();
            }
            (LlvmType::Float(float_ty), LlvmType::Int(int_ty)) => {
                let opcode = if int_ty.signed { "fptosi" } else { "fptoui" };
                writeln!(
                    out,
                    "  {} = {} {} {} to {}",
                    temp,
                    opcode,
                    float_ty.ir(),
                    value.repr,
                    int_ty.ir()
                )
                .unwrap();
            }
            (LlvmType::Bool, LlvmType::Float(float_ty)) => {
                writeln!(
                    out,
                    "  {} = uitofp i1 {} to {}",
                    temp,
                    value.repr,
                    float_ty.ir()
                )
                .unwrap();
            }
            (LlvmType::Float(float_ty), LlvmType::Bool) => {
                writeln!(
                    out,
                    "  {} = fcmp one {} {}, 0.0",
                    temp,
                    float_ty.ir(),
                    value.repr
                )
                .unwrap();
            }
            (LlvmType::Int(source), LlvmType::Int(target_int)) => {
                if source.bits == target_int.bits {
                    return Ok(ValueRef::new(
                        target,
                        value.repr.clone(),
                        infer_cast_sign(value.ty, target, value.sign),
                    ));
                }
                let opcode = if source.bits < target_int.bits {
                    if source.signed { "sext" } else { "zext" }
                } else {
                    "trunc"
                };
                writeln!(
                    out,
                    "  {} = {} {} {} to {}",
                    temp,
                    opcode,
                    source.ir(),
                    value.repr,
                    target_int.ir()
                )
                .unwrap();
            }
            (LlvmType::Float(source), LlvmType::Float(target_float)) => {
                let opcode = match (source, target_float) {
                    (LlvmFloatType::F32, LlvmFloatType::F64) => "fpext",
                    (LlvmFloatType::F64, LlvmFloatType::F32) => "fptrunc",
                    _ => {
                        return Ok(ValueRef::new(
                            target,
                            value.repr.clone(),
                            infer_cast_sign(value.ty, target, value.sign),
                        ));
                    }
                };
                writeln!(
                    out,
                    "  {} = {} {} {} to {}",
                    temp,
                    opcode,
                    source.ir(),
                    value.repr,
                    target_float.ir()
                )
                .unwrap();
            }
            (LlvmType::Str, LlvmType::OpaquePtr)
            | (LlvmType::OpaquePtr, LlvmType::Str)
            | (LlvmType::Str, LlvmType::Str)
            | (LlvmType::OpaquePtr, LlvmType::OpaquePtr) => {
                return Ok(ValueRef::new(
                    target,
                    value.repr.clone(),
                    infer_cast_sign(value.ty, target, value.sign),
                ));
            }
            _ => {
                return Err(format!(
                    "unsupported LLVM cast from {:?} to {:?}",
                    value.ty, target
                ));
            }
        }

        Ok(ValueRef::new(
            target,
            temp,
            infer_cast_sign(value.ty, target, value.sign),
        ))
    }

    fn intern_string_constant(&mut self, value: &str) -> String {
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0);
        let name = if let Some(name) = self.string_pool.get(&bytes) {
            name.clone()
        } else {
            let name = format!("@.str.{}", self.next_string_id);
            self.next_string_id += 1;
            self.globals.push(GlobalString {
                name: name.clone(),
                bytes: bytes.clone(),
            });
            self.string_pool.insert(bytes.clone(), name.clone());
            name
        };
        gep_string_ptr(&name, bytes.len())
    }

    fn register_external_decl(&mut self, symbol: &str, decl: &str) {
        self.external_decls
            .entry(symbol.into())
            .or_insert_with(|| decl.into());
    }

    fn fresh_temp_id(&mut self) -> usize {
        let id = self.next_temp_id;
        self.next_temp_id += 1;
        id
    }
}

fn get_value(values: &HashMap<ValueId, ValueRef>, value: ValueId) -> Result<ValueRef, String> {
    values
        .get(&value)
        .cloned()
        .ok_or_else(|| format!("unknown LLVM value `__v{}`", value.0))
}

fn mangle_name(name: &str) -> String {
    if name == "main" {
        "main".into()
    } else {
        format!("agam_{}", sanitize_name(name))
    }
}

fn analyze_function_attrs(
    module: &MirModule,
    layouts: &HashMap<String, FunctionLayout>,
) -> HashMap<String, FunctionAttrs> {
    let user_functions: HashSet<&str> = module
        .functions
        .iter()
        .map(|func| func.name.as_str())
        .collect();
    let mut attrs = HashMap::new();

    for func in &module.functions {
        let layout = layouts.get(&func.name);
        let mut fn_attrs = FunctionAttrs::default();
        let mut can_be_nounwind = true;
        let mut can_be_nofree = true;
        let mut can_be_norecurse = true;
        let mut can_be_nosync = true;

        for block in &func.blocks {
            for instr in &block.instructions {
                match &instr.op {
                    Op::BinOp {
                        op: MirBinOp::Add, ..
                    } => {
                        let result_ty = layout
                            .and_then(|layout| layout.value_types.get(&instr.result).copied())
                            .unwrap_or_else(LlvmType::default_int);
                        if result_ty == LlvmType::Str {
                            can_be_nofree = false;
                        }
                    }
                    Op::Call { callee, .. } => {
                        if user_functions.contains(callee.as_str()) {
                            can_be_norecurse = false;
                            can_be_nounwind = false;
                            can_be_nofree = false;
                            can_be_nosync = false;
                        } else if matches!(
                            callee.as_str(),
                            "print" | "println" | "print_int" | "print_str"
                        ) {
                            can_be_nounwind = false;
                            can_be_nofree = false;
                            can_be_nosync = false;
                        } else if callee == "parse_int" {
                            can_be_nosync = false;
                        } else if !matches!(callee.as_str(), "argc" | "argv") {
                            can_be_nounwind = false;
                            can_be_nofree = false;
                            can_be_nosync = false;
                        }
                    }
                    _ => {}
                }
            }
        }

        fn_attrs.nounwind = can_be_nounwind;
        fn_attrs.nofree = can_be_nofree;
        fn_attrs.norecurse = can_be_norecurse;
        fn_attrs.nosync = can_be_nosync;
        attrs.insert(func.name.clone(), fn_attrs);
    }

    attrs
}

fn format_function_attrs(attrs: FunctionAttrs) -> String {
    let mut parts = Vec::new();
    if attrs.nofree {
        parts.push("nofree");
    }
    if attrs.norecurse {
        parts.push("norecurse");
    }
    if attrs.nosync {
        parts.push("nosync");
    }
    if attrs.nounwind {
        parts.push("nounwind");
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" {}", parts.join(" "))
    }
}

fn format_int_arith_flags(flags: IntArithFlags) -> String {
    let mut parts = Vec::new();
    if flags.nuw {
        parts.push("nuw");
    }
    if flags.nsw {
        parts.push("nsw");
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" {}", parts.join(" "))
    }
}

fn sanitize_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() { "_".into() } else { out }
}

fn escape_llvm_attr_value(value: &str) -> String {
    value.replace('\\', "\\5C").replace('"', "\\22")
}

fn gep_string_ptr(name: &str, len: usize) -> String {
    format!("getelementptr inbounds ([{len} x i8], [{len} x i8]* {name}, i64 0, i64 0)")
}

fn escape_llvm_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &byte in bytes {
        match byte {
            b'\\' => out.push_str("\\5C"),
            b'"' => out.push_str("\\22"),
            0x20..=0x7e => out.push(char::from(byte)),
            _ => {
                write!(out, "\\{:02X}", byte).unwrap();
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;
    use agam_mir::lower::MirLowering;

    fn compile_to_llvm(source: &str) -> String {
        compile_to_llvm_with_options(source, LlvmEmitOptions::default())
    }

    fn compile_to_llvm_with_options(source: &str, options: LlvmEmitOptions) -> String {
        let source_id = SourceId(0);
        let mut lexer = Lexer::new(source, source_id);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == agam_lexer::TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        let mut parser = agam_parser::Parser::new(tokens);
        let module = parser.parse_module(source_id).expect("parse failed");

        let mut hir_lower = HirLowering::new();
        let hir = hir_lower.lower_module(&module);

        let mut mir_lower = MirLowering::new();
        let mut mir = mir_lower.lower_module(&hir);
        agam_mir::opt::optimize_module(&mut mir);

        emit_llvm_with_options(&mir, options).expect("LLVM emission failed")
    }

    #[test]
    fn test_emit_main_function() {
        let llvm = compile_to_llvm("fn main(): return 42");
        assert!(llvm.contains("define noundef i32 @main"));
        assert!(llvm.contains("ret i32"));
    }

    #[test]
    fn test_emit_print_call() {
        let llvm = compile_to_llvm("fn main(): print(42)");
        assert!(llvm.contains("declare noundef i32 @printf(i8* nocapture noundef readonly, ...)"));
        assert!(llvm.contains("@.str."));
        assert!(llvm.contains("call i32 (i8*, ...) @printf"));
    }

    #[test]
    fn test_emit_string_concat_call() {
        let llvm =
            compile_to_llvm("fn main() { let name = \"World\"; println(\"Hello, \" + name); }");
        assert!(llvm.contains("declare i8* @agam_str_concat(i8*, i8*)"));
        assert!(llvm.contains("call i8* @agam_str_concat"));
    }

    #[test]
    fn test_emit_integer_compare_for_bool_result() {
        let llvm = compile_to_llvm("fn main() { let ok = 1 < 2; print(ok); }");
        assert!(llvm.contains("icmp slt i32") || llvm.contains("i8* getelementptr inbounds"));
    }

    #[test]
    fn test_emit_unsigned_div_rem_for_proven_nonnegative_signed_values() {
        let llvm = compile_to_llvm(
            "fn work() -> i32 { let a: i32 = argc(); let b: i32 = argc(); return (a % b) / b; } fn main() { return 0; }",
        );
        assert!(llvm.contains("urem i32"));
        assert!(llvm.contains("udiv i32"));
    }

    #[test]
    fn test_emit_non_main_function() {
        let llvm = compile_to_llvm(
            "fn add(a: i32) -> i32 { return a + 1; } fn main() { return add(41); }",
        );
        assert!(llvm.contains("define noundef i32 @agam_add(i32 noundef %p0)"));
        assert!(llvm.contains("call noundef i32 @agam_add") || llvm.contains("ret i32 42"));
    }

    #[test]
    fn test_emit_explicit_i64_function() {
        let llvm =
            compile_to_llvm("fn add(a: i64) -> i64 { return a + 1; } fn main() { return 0; }");
        assert!(llvm.contains("define noundef i64 @agam_add(i64 noundef %p0)"));
        assert!(llvm.contains("add i64 %p0, 1") || llvm.contains("add i64"));
    }

    #[test]
    fn test_explicit_i64_local_does_not_collapse_to_i32() {
        let llvm =
            compile_to_llvm("fn add(a: i64) -> i64 { let x: i64 = a; x = x + 1; return x; }");
        assert!(llvm.contains("alloca i64"));
        assert!(llvm.contains("load i64"));
        assert!(llvm.contains("store i64"));
        assert!(llvm.contains("add i64"));
    }

    #[test]
    fn test_emit_target_metadata_when_configured() {
        let llvm = compile_to_llvm_with_options(
            "fn main(): return 0",
            LlvmEmitOptions {
                target_triple: Some("x86_64-pc-linux-gnu".into()),
                data_layout: Some(
                    "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
                        .into(),
                ),
            },
        );
        assert!(llvm.contains("target triple = \"x86_64-pc-linux-gnu\""));
        assert!(llvm.contains("target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128\""));
    }

    #[test]
    fn test_emit_nuw_nsw_for_proven_strict_loop_counter_increment() {
        let llvm = compile_to_llvm(
            "fn count(n: i64) -> i64 { let mut i: i64 = 0; while i < n { i = i + 1; } return i; }",
        );
        assert!(llvm.contains("add nuw nsw i64"));
    }

    #[test]
    fn test_do_not_emit_nuw_nsw_without_loop_proof() {
        let llvm =
            compile_to_llvm("fn add(a: i64) -> i64 { return a + 1; } fn main() { return 0; }");
        assert!(llvm.contains("add i64"));
        assert!(!llvm.contains("add nuw nsw i64"));
    }
}
