//! LLVM IR emitter — translates MIR into textual LLVM IR.
//!
//! This is the first Phase 14 backend increment: Agam user code can now be
//! lowered directly to `.ll` without going through C first. The supported MIR
//! surface intentionally covers the core scalar/string subset and leaves more
//! advanced runtime-heavy operations on the existing C backend for now.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::fmt::Write;
use std::hash::{Hash, Hasher};

use agam_mir::analysis::{
    CallCacheAnalysis, CallCacheMode as MirCallCacheMode, CallCacheRejectReason, CallCacheRequest,
};
use agam_mir::ir::*;
use agam_profile::{CallCacheSpecializationPlan, StableScalarValueProfile};
use agam_sema::types::{FloatSize, IntSize, Type, builtin_type_by_id};

const MAX_CALL_CACHE_ARGS: usize = 4;
const MAX_PROFILED_CALL_CACHE_KEYS: usize = 64;
const DEFAULT_CALL_CACHE_CAPACITY: usize = 256;
const DEFAULT_CALL_CACHE_WARMUP: u64 = 32;
const LLVM_CALL_CACHE_PROFILE_ENV: &str = "AGAM_LLVM_CALL_CACHE_PROFILE_OUT";
const LLVM_CALL_CACHE_PROFILE_HEADER: &str = "AGAM_LLVM_CALL_CACHE_PROFILE_V5\n";
const LLVM_CALL_CACHE_PROFILE_FUNCTION_LINE_FMT: &str = "FN\t%s\t%llu\t%llu\t%llu\t%u\t%u\t%llu\n";
const LLVM_CALL_CACHE_PROFILE_SPECIALIZATION_LINE_FMT: &str = "SP\t%s\t%llu\t%llu\n";
const LLVM_CALL_CACHE_PROFILE_STABLE_LINE_FMT: &str = "SV\t%s\t%u\t%llu\t%llu\n";
const LLVM_CALL_CACHE_PROFILE_REUSE_LINE_FMT: &str = "RD\t%s\t%llu\t%llu\t%llu\n";

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
pub struct LlvmEmitOptions {
    pub target_triple: Option<String>,
    pub data_layout: Option<String>,
    pub call_cache: bool,
    pub call_cache_only: Vec<String>,
    pub call_cache_exclude: Vec<String>,
    pub call_cache_optimize: bool,
    pub call_cache_optimize_only: Vec<String>,
    pub call_cache_specializations: Vec<CallCacheSpecializationPlan>,
    pub call_cache_capacity: usize,
    pub call_cache_warmup: u64,
}

impl LlvmEmitOptions {
    pub fn from_env() -> Self {
        Self {
            target_triple: env::var("AGAM_LLVM_TARGET_TRIPLE")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            data_layout: env::var("AGAM_LLVM_DATA_LAYOUT")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            call_cache: false,
            call_cache_only: Vec::new(),
            call_cache_exclude: Vec::new(),
            call_cache_optimize: false,
            call_cache_optimize_only: Vec::new(),
            call_cache_specializations: Vec::new(),
            call_cache_capacity: DEFAULT_CALL_CACHE_CAPACITY,
            call_cache_warmup: DEFAULT_CALL_CACHE_WARMUP,
        }
    }
}

#[derive(Clone, Default)]
struct CallCachePlan {
    cacheable_functions: Vec<String>,
    cacheable_set: HashSet<String>,
    optimized_functions: HashSet<String>,
}

impl CallCachePlan {
    fn contains(&self, name: &str) -> bool {
        self.cacheable_set.contains(name)
    }

    fn is_optimized(&self, name: &str) -> bool {
        self.optimized_functions.contains(name)
    }
}

#[derive(Clone)]
struct LlvmFunctionSpecialization {
    clone_name: String,
    stable_values: Vec<StableScalarValueProfile>,
}

impl LlvmFunctionSpecialization {
    fn stable_bits_for(&self, index: usize) -> Option<u64> {
        self.stable_values
            .iter()
            .find(|value| value.index == index)
            .map(|value| value.raw_bits)
    }
}

#[derive(Clone, Default)]
struct SpecializationRegistry {
    by_function: HashMap<String, Vec<LlvmFunctionSpecialization>>,
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

pub fn emit_llvm_with_options(
    module: &MirModule,
    mut options: LlvmEmitOptions,
) -> Result<String, String> {
    if options.call_cache_capacity == 0 {
        options.call_cache_capacity = 1;
    }
    let layouts = analyze_module(module);
    let call_cache_analysis = build_call_cache_analysis(module, &layouts, &options);
    let call_cache_plan = call_cache_plan_from_analysis(&call_cache_analysis);
    let mut emitter = LlvmEmitter::new(module, layouts, call_cache_plan, options);
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

pub fn analyze_call_cache(module: &MirModule, options: &LlvmEmitOptions) -> CallCacheAnalysis {
    let layouts = analyze_module(module);
    build_call_cache_analysis(module, &layouts, options)
}

fn build_call_cache_analysis(
    module: &MirModule,
    layouts: &HashMap<String, FunctionLayout>,
    options: &LlvmEmitOptions,
) -> CallCacheAnalysis {
    let support_reasons = module
        .functions
        .iter()
        .map(|function| {
            let reasons = layouts
                .get(&function.name)
                .map(llvm_call_cache_support_reasons)
                .unwrap_or_else(|| {
                    vec![CallCacheRejectReason::UnsupportedReturnType {
                        description: "function layout analysis failed".into(),
                    }]
                });
            (function.name.clone(), reasons)
        })
        .collect();
    agam_mir::analysis::analyze_call_cache(
        module,
        &call_cache_request_from_options(options),
        &support_reasons,
    )
}

fn call_cache_request_from_options(options: &LlvmEmitOptions) -> CallCacheRequest {
    let mut include_only = options
        .call_cache_only
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    include_only.extend(options.call_cache_optimize_only.iter().cloned());

    CallCacheRequest {
        enable_all: options.call_cache || options.call_cache_optimize,
        optimize_all: options.call_cache_optimize,
        include_only,
        optimize_only: options.call_cache_optimize_only.iter().cloned().collect(),
        exclude: options.call_cache_exclude.iter().cloned().collect(),
    }
}

fn call_cache_plan_from_analysis(analysis: &CallCacheAnalysis) -> CallCachePlan {
    let cacheable_functions: Vec<String> = analysis
        .functions
        .iter()
        .filter_map(|function| function.mode.map(|_| function.name.clone()))
        .collect();
    let cacheable_set = cacheable_functions.iter().cloned().collect();
    let optimized_functions = analysis
        .functions
        .iter()
        .filter_map(|function| match function.mode {
            Some(MirCallCacheMode::Optimize) => Some(function.name.clone()),
            _ => None,
        })
        .collect();

    CallCachePlan {
        cacheable_functions,
        cacheable_set,
        optimized_functions,
    }
}

fn llvm_call_cache_support_reasons(layout: &FunctionLayout) -> Vec<CallCacheRejectReason> {
    let mut reasons = Vec::new();

    if layout.params.len() > MAX_CALL_CACHE_ARGS {
        reasons.push(CallCacheRejectReason::TooManyArguments {
            actual: layout.params.len(),
            max_supported: MAX_CALL_CACHE_ARGS,
        });
    }
    if !supports_call_cache_type(layout.return_ty) {
        reasons.push(CallCacheRejectReason::UnsupportedReturnType {
            description: describe_llvm_call_cache_type(layout.return_ty),
        });
    }
    for (index, ty) in layout.params.iter().copied().enumerate() {
        if !supports_call_cache_type(ty) {
            reasons.push(CallCacheRejectReason::UnsupportedParameterType {
                index,
                description: describe_llvm_call_cache_type(ty),
            });
        }
    }

    reasons
}

fn describe_llvm_call_cache_type(ty: LlvmType) -> String {
    match ty {
        LlvmType::Str => {
            "strings are pointer-backed and do not have a stable scalar cache encoding yet".into()
        }
        LlvmType::OpaquePtr => {
            "pointer-like values carry unstable aliasing and identity for deterministic cache keys"
                .into()
        }
        LlvmType::Int(int_ty) if int_ty.bits > 64 => format!(
            "{}-bit integers are wider than the current 64-bit cache encoding",
            int_ty.bits
        ),
        _ => "the current runtime cache only supports scalar bool/int/float values".into(),
    }
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

fn supports_call_cache_type(ty: LlvmType) -> bool {
    match ty {
        LlvmType::Int(int_ty) => int_ty.bits <= 64,
        LlvmType::Float(LlvmFloatType::F32 | LlvmFloatType::F64) | LlvmType::Bool => true,
        LlvmType::Str | LlvmType::OpaquePtr => false,
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
        }) if is_square_of_local(instrs, *left, local_name) => {
            Some(IncrementGuardKind::SquareUpper)
        }
        Some(Op::BinOp {
            op: MirBinOp::GtEq,
            left: _,
            right,
        }) if is_square_of_local(instrs, *right, local_name) => {
            Some(IncrementGuardKind::SquareUpper)
        }
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
    call_cache_plan: CallCachePlan,
    specializations: SpecializationRegistry,
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
        call_cache_plan: CallCachePlan,
        options: LlvmEmitOptions,
    ) -> Self {
        let specializations = build_specialization_registry(
            module,
            &layouts,
            &call_cache_plan,
            &options.call_cache_specializations,
        );
        let function_attrs = analyze_function_attrs(module, &layouts);
        Self {
            options,
            layouts,
            call_cache_plan,
            specializations,
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
            let emitted_name = if func.name == "main" {
                "main".to_string()
            } else {
                mangle_name(&func.name)
            };
            let body = self.emit_function(func, &layout, &emitted_name, None)?;
            functions.push_str(&body);
            functions.push('\n');
            if let Some(specializations) = self.specializations.by_function.get(&func.name).cloned()
            {
                for specialization in specializations {
                    let clone_body = self.emit_function(
                        func,
                        &layout,
                        &specialization.clone_name,
                        Some(&specialization),
                    )?;
                    functions.push_str(&clone_body);
                    functions.push('\n');
                }
            }
        }
        self.prepare_call_cache_profile_export();

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
        self.emit_call_cache_globals(&mut output)?;
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
        self.emit_call_cache_wrappers(&mut output)?;
        self.emit_call_cache_profile_export(&mut output)?;
        output.push_str(
            "attributes #0 = { nofree nounwind }\n\
             attributes #1 = { nofree nounwind willreturn }\n\
             attributes #2 = { nofree norecurse nosync nounwind willreturn }\n\
             attributes #3 = { nofree norecurse nosync nounwind willreturn }\n",
        );
        Ok(output)
    }

    fn emit_call_cache_globals(&self, out: &mut String) -> Result<(), String> {
        if self.call_cache_plan.cacheable_functions.is_empty() {
            return Ok(());
        }

        writeln!(out).unwrap();
        for name in &self.call_cache_plan.cacheable_functions {
            let globals = call_cache_global_names(name);
            writeln!(out, "{} = internal global i64 0, align 8", globals.calls).unwrap();
            writeln!(out, "{} = internal global i64 0, align 8", globals.hits).unwrap();
            writeln!(out, "{} = internal global i64 0, align 8", globals.stores).unwrap();
            writeln!(out, "{} = internal global i32 0, align 4", globals.len).unwrap();
            writeln!(
                out,
                "{} = internal global [{} x [{} x i64]] zeroinitializer, align 16",
                globals.keys, self.options.call_cache_capacity, MAX_CALL_CACHE_ARGS
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global [{} x i64] zeroinitializer, align 8",
                globals.values, self.options.call_cache_capacity
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global [{} x i64] zeroinitializer, align 8",
                globals.ages, self.options.call_cache_capacity
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global [{} x i64] zeroinitializer, align 8",
                globals.entry_hits, self.options.call_cache_capacity
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global i32 0, align 4",
                globals.profile_unique_keys
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global i64 0, align 8",
                globals.profile_hottest_key_hits
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global [{} x [{} x i64]] zeroinitializer, align 16",
                globals.profile_observed_keys, MAX_PROFILED_CALL_CACHE_KEYS, MAX_CALL_CACHE_ARGS
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global [{} x i64] zeroinitializer, align 8",
                globals.profile_observed_hits, MAX_PROFILED_CALL_CACHE_KEYS
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global [{} x i64] zeroinitializer, align 8",
                globals.profile_observed_last_seen, MAX_PROFILED_CALL_CACHE_KEYS
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global [{} x i64] zeroinitializer, align 8",
                globals.profile_observed_reuse, MAX_PROFILED_CALL_CACHE_KEYS
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global [{} x i64] zeroinitializer, align 16",
                globals.profile_stable_values, MAX_CALL_CACHE_ARGS
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global [{} x i64] zeroinitializer, align 16",
                globals.profile_stable_scores, MAX_CALL_CACHE_ARGS
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global [{} x i64] zeroinitializer, align 16",
                globals.profile_stable_matches, MAX_CALL_CACHE_ARGS
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global i64 0, align 8",
                globals.profile_reuse_total
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global i64 0, align 8",
                globals.profile_reuse_samples
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global i64 0, align 8",
                globals.profile_reuse_max
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global i64 0, align 8",
                globals.profile_specialization_hits
            )
            .unwrap();
            writeln!(
                out,
                "{} = internal global i64 0, align 8",
                globals.profile_specialization_fallbacks
            )
            .unwrap();
            if self.call_cache_plan.is_optimized(name) {
                writeln!(
                    out,
                    "{} = internal global [{} x i32] zeroinitializer, align 4",
                    globals.scores, self.options.call_cache_capacity
                )
                .unwrap();
                writeln!(
                    out,
                    "{} = internal global i1 false, align 1",
                    globals.pending_valid
                )
                .unwrap();
                writeln!(
                    out,
                    "{} = internal global i32 0, align 4",
                    globals.pending_count
                )
                .unwrap();
                writeln!(
                    out,
                    "{} = internal global i64 0, align 8",
                    globals.pending_last_seen
                )
                .unwrap();
                writeln!(
                    out,
                    "{} = internal global [{} x i64] zeroinitializer, align 16",
                    globals.pending_keys, MAX_CALL_CACHE_ARGS
                )
                .unwrap();
            }
        }
        Ok(())
    }

    fn prepare_call_cache_profile_export(&mut self) {
        if self.call_cache_plan.cacheable_functions.is_empty() {
            return;
        }

        self.register_external_decl(
            "getenv",
            "declare noundef i8* @getenv(i8* nocapture noundef readonly) local_unnamed_addr #0",
        );
        self.register_external_decl(
            "fopen",
            "declare noundef i8* @fopen(i8* nocapture noundef readonly, i8* nocapture noundef readonly) local_unnamed_addr #0",
        );
        self.register_external_decl(
            "fprintf",
            "declare noundef i32 @fprintf(i8*, i8* nocapture noundef readonly, ...) local_unnamed_addr #0",
        );
        self.register_external_decl(
            "fclose",
            "declare noundef i32 @fclose(i8*) local_unnamed_addr #0",
        );
        let _ = self.intern_string_constant(LLVM_CALL_CACHE_PROFILE_ENV);
        let _ = self.intern_string_constant("w");
        let _ = self.intern_string_constant(LLVM_CALL_CACHE_PROFILE_HEADER);
        let _ = self.intern_string_constant(LLVM_CALL_CACHE_PROFILE_FUNCTION_LINE_FMT);
        let _ = self.intern_string_constant(LLVM_CALL_CACHE_PROFILE_SPECIALIZATION_LINE_FMT);
        let _ = self.intern_string_constant(LLVM_CALL_CACHE_PROFILE_STABLE_LINE_FMT);
        let _ = self.intern_string_constant(LLVM_CALL_CACHE_PROFILE_REUSE_LINE_FMT);
        let cacheable_names = self.call_cache_plan.cacheable_functions.clone();
        for name in &cacheable_names {
            let _ = self.intern_string_constant(name);
        }
    }

    fn emit_call_cache_profile_export(&mut self, out: &mut String) -> Result<(), String> {
        if self.call_cache_plan.cacheable_functions.is_empty() {
            return Ok(());
        }

        let env_name = self.intern_string_constant(LLVM_CALL_CACHE_PROFILE_ENV);
        let mode = self.intern_string_constant("w");
        let header = self.intern_string_constant(LLVM_CALL_CACHE_PROFILE_HEADER);
        let function_line_fmt =
            self.intern_string_constant(LLVM_CALL_CACHE_PROFILE_FUNCTION_LINE_FMT);
        let specialization_line_fmt =
            self.intern_string_constant(LLVM_CALL_CACHE_PROFILE_SPECIALIZATION_LINE_FMT);
        let stable_line_fmt = self.intern_string_constant(LLVM_CALL_CACHE_PROFILE_STABLE_LINE_FMT);
        let reuse_line_fmt = self.intern_string_constant(LLVM_CALL_CACHE_PROFILE_REUSE_LINE_FMT);

        writeln!(
            out,
            "define void @agam_call_cache_dump_profiles() local_unnamed_addr #0 {{"
        )
        .unwrap();
        writeln!(out, "entry:").unwrap();
        writeln!(out, "  %p0 = call i8* @getenv(i8* {env_name})").unwrap();
        writeln!(out, "  %p1 = icmp eq i8* %p0, null").unwrap();
        writeln!(out, "  br i1 %p1, label %ret, label %open").unwrap();
        writeln!(out, "open:").unwrap();
        writeln!(out, "  %p2 = call i8* @fopen(i8* %p0, i8* {mode})").unwrap();
        writeln!(out, "  %p3 = icmp eq i8* %p2, null").unwrap();
        writeln!(out, "  br i1 %p3, label %ret, label %write").unwrap();
        writeln!(out, "write:").unwrap();
        writeln!(
            out,
            "  %p4 = call i32 (i8*, i8*, ...) @fprintf(i8* %p2, i8* {header})"
        )
        .unwrap();

        let cacheable_names = self.call_cache_plan.cacheable_functions.clone();
        let mut next_temp = 10usize;
        for name in &cacheable_names {
            let globals = call_cache_global_names(name);
            let name_ptr = self.intern_string_constant(name);
            let layout = self.layouts.get(name).ok_or_else(|| {
                format!("missing LLVM layout for call-cache profile export `{name}`")
            })?;
            let calls_tmp = fresh_call_cache_temp(&mut next_temp);
            let hits_tmp = fresh_call_cache_temp(&mut next_temp);
            let stores_tmp = fresh_call_cache_temp(&mut next_temp);
            let len_tmp = fresh_call_cache_temp(&mut next_temp);
            let unique_keys_tmp = fresh_call_cache_temp(&mut next_temp);
            let hottest_key_hits_tmp = fresh_call_cache_temp(&mut next_temp);
            let write_tmp = fresh_call_cache_temp(&mut next_temp);
            let specialization_hits_tmp = fresh_call_cache_temp(&mut next_temp);
            let specialization_fallbacks_tmp = fresh_call_cache_temp(&mut next_temp);
            let specialization_write_tmp = fresh_call_cache_temp(&mut next_temp);
            let reuse_total_tmp = fresh_call_cache_temp(&mut next_temp);
            let reuse_samples_tmp = fresh_call_cache_temp(&mut next_temp);
            let reuse_max_tmp = fresh_call_cache_temp(&mut next_temp);
            let reuse_has_samples_tmp = fresh_call_cache_temp(&mut next_temp);
            let reuse_safe_samples_tmp = fresh_call_cache_temp(&mut next_temp);
            let reuse_avg_tmp = fresh_call_cache_temp(&mut next_temp);
            let reuse_write_tmp = fresh_call_cache_temp(&mut next_temp);
            writeln!(out, "  {calls_tmp} = load i64, i64* {}", globals.calls).unwrap();
            writeln!(out, "  {hits_tmp} = load i64, i64* {}", globals.hits).unwrap();
            writeln!(out, "  {stores_tmp} = load i64, i64* {}", globals.stores).unwrap();
            writeln!(out, "  {len_tmp} = load i32, i32* {}", globals.len).unwrap();
            writeln!(
                out,
                "  {unique_keys_tmp} = load i32, i32* {}",
                globals.profile_unique_keys
            )
            .unwrap();
            writeln!(
                out,
                "  {hottest_key_hits_tmp} = load i64, i64* {}",
                globals.profile_hottest_key_hits
            )
            .unwrap();
            writeln!(
                out,
                "  {specialization_hits_tmp} = load i64, i64* {}",
                globals.profile_specialization_hits
            )
            .unwrap();
            writeln!(
                out,
                "  {specialization_fallbacks_tmp} = load i64, i64* {}",
                globals.profile_specialization_fallbacks
            )
            .unwrap();
            writeln!(
                out,
                "  {reuse_total_tmp} = load i64, i64* {}",
                globals.profile_reuse_total
            )
            .unwrap();
            writeln!(
                out,
                "  {reuse_samples_tmp} = load i64, i64* {}",
                globals.profile_reuse_samples
            )
            .unwrap();
            writeln!(
                out,
                "  {reuse_max_tmp} = load i64, i64* {}",
                globals.profile_reuse_max
            )
            .unwrap();
            writeln!(
                out,
                "  {reuse_has_samples_tmp} = icmp ne i64 {reuse_samples_tmp}, 0"
            )
            .unwrap();
            writeln!(
                out,
                "  {reuse_safe_samples_tmp} = select i1 {reuse_has_samples_tmp}, i64 {reuse_samples_tmp}, i64 1"
            )
            .unwrap();
            writeln!(
                out,
                "  {reuse_avg_tmp} = udiv i64 {reuse_total_tmp}, {reuse_safe_samples_tmp}"
            )
            .unwrap();
            writeln!(
                out,
                "  {write_tmp} = call i32 (i8*, i8*, ...) @fprintf(i8* %p2, i8* {function_line_fmt}, i8* {name_ptr}, i64 {calls_tmp}, i64 {hits_tmp}, i64 {stores_tmp}, i32 {len_tmp}, i32 {unique_keys_tmp}, i64 {hottest_key_hits_tmp})"
            )
            .unwrap();
            writeln!(
                out,
                "  {specialization_write_tmp} = call i32 (i8*, i8*, ...) @fprintf(i8* %p2, i8* {specialization_line_fmt}, i8* {name_ptr}, i64 {specialization_hits_tmp}, i64 {specialization_fallbacks_tmp})"
            )
            .unwrap();
            for arg_index in 0..layout.params.len() {
                let stable_value_ptr = fresh_call_cache_temp(&mut next_temp);
                let stable_value = fresh_call_cache_temp(&mut next_temp);
                let stable_matches_ptr = fresh_call_cache_temp(&mut next_temp);
                let stable_matches = fresh_call_cache_temp(&mut next_temp);
                let stable_write = fresh_call_cache_temp(&mut next_temp);
                writeln!(
                    out,
                    "  {stable_value_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {}",
                    MAX_CALL_CACHE_ARGS,
                    MAX_CALL_CACHE_ARGS,
                    globals.profile_stable_values,
                    arg_index
                )
                .unwrap();
                writeln!(out, "  {stable_value} = load i64, i64* {stable_value_ptr}").unwrap();
                writeln!(
                    out,
                    "  {stable_matches_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {}",
                    MAX_CALL_CACHE_ARGS,
                    MAX_CALL_CACHE_ARGS,
                    globals.profile_stable_matches,
                    arg_index
                )
                .unwrap();
                writeln!(
                    out,
                    "  {stable_matches} = load i64, i64* {stable_matches_ptr}"
                )
                .unwrap();
                writeln!(
                    out,
                    "  {stable_write} = call i32 (i8*, i8*, ...) @fprintf(i8* %p2, i8* {stable_line_fmt}, i8* {name_ptr}, i32 {}, i64 {stable_value}, i64 {stable_matches})",
                    arg_index
                )
                .unwrap();
            }
            writeln!(
                out,
                "  {reuse_write_tmp} = call i32 (i8*, i8*, ...) @fprintf(i8* %p2, i8* {reuse_line_fmt}, i8* {name_ptr}, i64 {reuse_avg_tmp}, i64 {reuse_max_tmp}, i64 {reuse_samples_tmp})"
            )
            .unwrap();
        }

        writeln!(out, "  %p5 = call i32 @fclose(i8* %p2)").unwrap();
        writeln!(out, "  br label %ret").unwrap();
        writeln!(out, "ret:").unwrap();
        writeln!(out, "  ret void").unwrap();
        writeln!(out, "}}\n").unwrap();
        Ok(())
    }

    fn emit_call_cache_wrappers(&self, out: &mut String) -> Result<(), String> {
        for name in &self.call_cache_plan.cacheable_functions {
            let layout = self
                .layouts
                .get(name)
                .ok_or_else(|| format!("missing LLVM layout for call-cache wrapper `{name}`"))?;
            emit_call_cache_wrapper_ir(
                out,
                name,
                layout,
                self.options.call_cache_capacity,
                self.options.call_cache_warmup,
                self.call_cache_plan.is_optimized(name),
                self.specializations
                    .by_function
                    .get(name)
                    .map(|specializations| specializations.as_slice())
                    .unwrap_or(&[]),
            )?;
        }
        Ok(())
    }

    fn call_target_symbol(&self, callee: &str) -> String {
        if self.call_cache_plan.contains(callee) {
            call_cache_wrapper_name(callee)
        } else {
            mangle_name(callee)
        }
    }

    fn emit_function(
        &mut self,
        func: &MirFunction,
        layout: &FunctionLayout,
        emitted_name: &str,
        specialization: Option<&LlvmFunctionSpecialization>,
    ) -> Result<String, String> {
        let mut out = String::new();
        let mut values: HashMap<ValueId, ValueRef> = HashMap::new();
        let mut locals: HashMap<String, (LlvmType, String)> = HashMap::new();
        let mut emitted_locals = HashSet::new();
        let mut specialization_temp_id = 0usize;
        let attrs = self
            .function_attrs
            .get(&func.name)
            .copied()
            .unwrap_or_default();
        let fn_attr_suffix = format_function_attrs(attrs);
        let is_main = func.name == "main" && specialization.is_none();

        if is_main {
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
                emitted_name
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
                if is_main {
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
                    let bound_value = if let Some(raw_bits) =
                        specialization.and_then(|spec| spec.stable_bits_for(i))
                    {
                        emit_call_cache_decode_value(
                            &mut out,
                            &mut specialization_temp_id,
                            ty,
                            &llvm_i64_literal(raw_bits),
                        )?
                    } else {
                        format!("%p{}", i)
                    };
                    writeln!(
                        out,
                        "  store {} {}, {}* {}",
                        ty.ir(),
                        bound_value,
                        ty.ir(),
                        local_name
                    )
                    .unwrap();
                    locals.insert(param.name.clone(), (ty, local_name.clone()));
                    emitted_locals.insert(param.name.clone());
                    values.insert(
                        param.value,
                        ValueRef::new(ty, bound_value, value_sign(layout, param.value)),
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
                    // Emit escape analysis annotation as an LLVM IR comment.
                    let escape_comment = if instr.metadata.stack_promote {
                        " ; stack-promoted, noalias"
                    } else {
                        match instr.metadata.escape_state {
                            Some(EscapeState::GlobalEscape) => " ; escapes: global",
                            Some(EscapeState::ArgEscape) => " ; escapes: arg",
                            _ => "",
                        }
                    };
                    writeln!(
                        out,
                        "  {} = alloca {}{}",
                        ptr_name,
                        local_ty.ir(),
                        escape_comment,
                    )
                    .unwrap();
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
                    let symbol = self.call_target_symbol(callee);
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
        let flags = if matches!(
            op,
            MirBinOp::Add | MirBinOp::Sub | MirBinOp::Mul | MirBinOp::Shl
        ) {
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
                    if !self.call_cache_plan.cacheable_functions.is_empty() {
                        writeln!(out, "  call void @agam_call_cache_dump_profiles()").unwrap();
                    }
                    writeln!(out, "  ret i32 {}", returned.repr).unwrap();
                } else {
                    let returned = self.coerce_value(out, &returned, layout.return_ty)?;
                    writeln!(out, "  ret {} {}", layout.return_ty.ir(), returned.repr).unwrap();
                }
            }
            Terminator::ReturnVoid => {
                if is_main {
                    if !self.call_cache_plan.cacheable_functions.is_empty() {
                        writeln!(out, "  call void @agam_call_cache_dump_profiles()").unwrap();
                    }
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

struct CallCacheGlobalNames {
    calls: String,
    hits: String,
    stores: String,
    len: String,
    keys: String,
    values: String,
    scores: String,
    ages: String,
    entry_hits: String,
    pending_valid: String,
    pending_count: String,
    pending_last_seen: String,
    pending_keys: String,
    profile_unique_keys: String,
    profile_hottest_key_hits: String,
    profile_observed_keys: String,
    profile_observed_hits: String,
    profile_observed_last_seen: String,
    profile_observed_reuse: String,
    profile_stable_values: String,
    profile_stable_scores: String,
    profile_stable_matches: String,
    profile_reuse_total: String,
    profile_reuse_samples: String,
    profile_reuse_max: String,
    profile_specialization_hits: String,
    profile_specialization_fallbacks: String,
}

fn call_cache_wrapper_name(name: &str) -> String {
    format!("agam_call_cache_{}", sanitize_name(name))
}

fn call_cache_global_names(name: &str) -> CallCacheGlobalNames {
    let base = sanitize_name(name);
    CallCacheGlobalNames {
        calls: format!("@agam_call_cache_{}_calls", base),
        hits: format!("@agam_call_cache_{}_hits", base),
        stores: format!("@agam_call_cache_{}_stores", base),
        len: format!("@agam_call_cache_{}_len", base),
        keys: format!("@agam_call_cache_{}_keys", base),
        values: format!("@agam_call_cache_{}_values", base),
        scores: format!("@agam_call_cache_{}_scores", base),
        ages: format!("@agam_call_cache_{}_ages", base),
        entry_hits: format!("@agam_call_cache_{}_entry_hits", base),
        pending_valid: format!("@agam_call_cache_{}_pending_valid", base),
        pending_count: format!("@agam_call_cache_{}_pending_count", base),
        pending_last_seen: format!("@agam_call_cache_{}_pending_last_seen", base),
        pending_keys: format!("@agam_call_cache_{}_pending_keys", base),
        profile_unique_keys: format!("@agam_call_cache_{}_profile_unique_keys", base),
        profile_hottest_key_hits: format!("@agam_call_cache_{}_profile_hottest_key_hits", base),
        profile_observed_keys: format!("@agam_call_cache_{}_profile_observed_keys", base),
        profile_observed_hits: format!("@agam_call_cache_{}_profile_observed_hits", base),
        profile_observed_last_seen: format!("@agam_call_cache_{}_profile_observed_last_seen", base),
        profile_observed_reuse: format!("@agam_call_cache_{}_profile_observed_reuse", base),
        profile_stable_values: format!("@agam_call_cache_{}_profile_stable_values", base),
        profile_stable_scores: format!("@agam_call_cache_{}_profile_stable_scores", base),
        profile_stable_matches: format!("@agam_call_cache_{}_profile_stable_matches", base),
        profile_reuse_total: format!("@agam_call_cache_{}_profile_reuse_total", base),
        profile_reuse_samples: format!("@agam_call_cache_{}_profile_reuse_samples", base),
        profile_reuse_max: format!("@agam_call_cache_{}_profile_reuse_max", base),
        profile_specialization_hits: format!(
            "@agam_call_cache_{}_profile_specialization_hits",
            base
        ),
        profile_specialization_fallbacks: format!(
            "@agam_call_cache_{}_profile_specialization_fallbacks",
            base
        ),
    }
}

fn emit_call_cache_wrapper_ir(
    out: &mut String,
    name: &str,
    layout: &FunctionLayout,
    capacity: usize,
    warmup: u64,
    optimize: bool,
    specializations: &[LlvmFunctionSpecialization],
) -> Result<(), String> {
    if optimize {
        emit_optimized_call_cache_wrapper_ir(out, name, layout, capacity, warmup, specializations)
    } else {
        emit_basic_call_cache_wrapper_ir(out, name, layout, capacity, warmup, specializations)
    }
}

fn fresh_call_cache_label(next_temp: &mut usize, prefix: &str) -> String {
    let value = format!("{prefix}_{}", *next_temp);
    *next_temp += 1;
    value
}

fn emit_increment_i64_global(out: &mut String, next_temp: &mut usize, global: &str) {
    let current = fresh_call_cache_temp(next_temp);
    let next = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {current} = load i64, i64* {global}").unwrap();
    writeln!(out, "  {next} = add i64 {current}, 1").unwrap();
    writeln!(out, "  store i64 {next}, i64* {global}").unwrap();
}

fn emit_update_i64_global_max(
    out: &mut String,
    next_temp: &mut usize,
    global: &str,
    candidate: &str,
) {
    let current = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {current} = load i64, i64* {global}").unwrap();
    let better = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {better} = icmp ugt i64 {candidate}, {current}").unwrap();
    let updated = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {updated} = select i1 {better}, i64 {candidate}, i64 {current}"
    )
    .unwrap();
    writeln!(out, "  store i64 {updated}, i64* {global}").unwrap();
}

fn emit_increment_call_cache_entry_hits(
    out: &mut String,
    next_temp: &mut usize,
    globals: &CallCacheGlobalNames,
    capacity: usize,
    index_i64: &str,
) {
    let entry_hits_ptr = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {entry_hits_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {index_i64}",
        capacity, capacity, globals.entry_hits
    )
    .unwrap();
    let entry_hits_old = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {entry_hits_old} = load i64, i64* {entry_hits_ptr}").unwrap();
    let entry_hits_new = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {entry_hits_new} = add i64 {entry_hits_old}, 1").unwrap();
    writeln!(out, "  store i64 {entry_hits_new}, i64* {entry_hits_ptr}").unwrap();
    emit_update_i64_global_max(
        out,
        next_temp,
        &globals.profile_hottest_key_hits,
        &entry_hits_new,
    );
}

fn emit_store_call_cache_entry_hits(
    out: &mut String,
    next_temp: &mut usize,
    globals: &CallCacheGlobalNames,
    capacity: usize,
    index_i64: &str,
    hits: &str,
) {
    let entry_hits_ptr = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {entry_hits_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {index_i64}",
        capacity, capacity, globals.entry_hits
    )
    .unwrap();
    writeln!(out, "  store i64 {hits}, i64* {entry_hits_ptr}").unwrap();
    emit_update_i64_global_max(out, next_temp, &globals.profile_hottest_key_hits, hits);
}

fn emit_call_cache_observe_key(
    out: &mut String,
    next_temp: &mut usize,
    globals: &CallCacheGlobalNames,
    predecessor_label: &str,
    encoded_args: &[String],
    calls_new: &str,
) -> (String, String) {
    let scan_label = fresh_call_cache_label(next_temp, "observe_scan");
    let check_label = fresh_call_cache_label(next_temp, "observe_check");
    let hit_label = fresh_call_cache_label(next_temp, "observe_hit");
    let next_label = fresh_call_cache_label(next_temp, "observe_next");
    let insert_check_label = fresh_call_cache_label(next_temp, "observe_insert_check");
    let insert_label = fresh_call_cache_label(next_temp, "observe_insert");
    let done_label = fresh_call_cache_label(next_temp, "observe_done");

    let candidate_hits_slot = fresh_call_cache_temp(next_temp);
    let candidate_reuse_slot = fresh_call_cache_temp(next_temp);
    let victim_index_slot = fresh_call_cache_temp(next_temp);
    let victim_hits_slot = fresh_call_cache_temp(next_temp);
    let victim_last_seen_slot = fresh_call_cache_temp(next_temp);
    let victim_scan_index_slot = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {candidate_hits_slot} = alloca i32").unwrap();
    writeln!(out, "  {candidate_reuse_slot} = alloca i64").unwrap();
    writeln!(out, "  {victim_index_slot} = alloca i32").unwrap();
    writeln!(out, "  {victim_hits_slot} = alloca i64").unwrap();
    writeln!(out, "  {victim_last_seen_slot} = alloca i64").unwrap();
    writeln!(out, "  {victim_scan_index_slot} = alloca i32").unwrap();
    writeln!(out, "  store i32 1, i32* {candidate_hits_slot}").unwrap();
    writeln!(out, "  store i64 0, i64* {candidate_reuse_slot}").unwrap();

    let observed_len = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_len} = load i32, i32* {}",
        globals.profile_unique_keys
    )
    .unwrap();
    let has_observed = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {has_observed} = icmp ne i32 {observed_len}, 0").unwrap();
    writeln!(
        out,
        "  br i1 {has_observed}, label %{scan_label}, label %{insert_check_label}"
    )
    .unwrap();

    let next_index = fresh_call_cache_temp(next_temp);
    writeln!(out, "{scan_label}:").unwrap();
    let scan_index = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {scan_index} = phi i32 [ 0, %{predecessor_label} ], [ {next_index}, %{next_label} ]"
    )
    .unwrap();
    let scan_done = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {scan_done} = icmp eq i32 {scan_index}, {observed_len}"
    )
    .unwrap();
    writeln!(
        out,
        "  br i1 {scan_done}, label %{insert_check_label}, label %{check_label}"
    )
    .unwrap();

    writeln!(out, "{check_label}:").unwrap();
    let scan_index_i64 = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {scan_index_i64} = zext i32 {scan_index} to i64").unwrap();
    if encoded_args.is_empty() {
        writeln!(out, "  br label %{hit_label}").unwrap();
    } else {
        let mut combined_match = String::new();
        for (arg_index, arg_bits) in encoded_args.iter().enumerate() {
            let arg_ptr = fresh_call_cache_temp(next_temp);
            writeln!(
                out,
                "  {arg_ptr} = getelementptr inbounds [{} x [{} x i64]], [{} x [{} x i64]]* {}, i64 0, i64 {scan_index_i64}, i64 {}",
                MAX_PROFILED_CALL_CACHE_KEYS,
                MAX_CALL_CACHE_ARGS,
                MAX_PROFILED_CALL_CACHE_KEYS,
                MAX_CALL_CACHE_ARGS,
                globals.profile_observed_keys,
                arg_index
            )
            .unwrap();
            let arg_loaded = fresh_call_cache_temp(next_temp);
            writeln!(out, "  {arg_loaded} = load i64, i64* {arg_ptr}").unwrap();
            let arg_eq = fresh_call_cache_temp(next_temp);
            writeln!(out, "  {arg_eq} = icmp eq i64 {arg_loaded}, {arg_bits}").unwrap();
            if combined_match.is_empty() {
                combined_match = arg_eq;
            } else {
                let new_match = fresh_call_cache_temp(next_temp);
                writeln!(out, "  {new_match} = and i1 {combined_match}, {arg_eq}").unwrap();
                combined_match = new_match;
            }
        }
        writeln!(
            out,
            "  br i1 {combined_match}, label %{hit_label}, label %{next_label}"
        )
        .unwrap();
    }

    writeln!(out, "{hit_label}:").unwrap();
    let observed_hits_ptr = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_hits_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {scan_index_i64}",
        MAX_PROFILED_CALL_CACHE_KEYS,
        MAX_PROFILED_CALL_CACHE_KEYS,
        globals.profile_observed_hits
    )
    .unwrap();
    let observed_hits_old = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_hits_old} = load i64, i64* {observed_hits_ptr}"
    )
    .unwrap();
    let observed_hits_new = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_hits_new} = add i64 {observed_hits_old}, 1"
    )
    .unwrap();
    writeln!(
        out,
        "  store i64 {observed_hits_new}, i64* {observed_hits_ptr}"
    )
    .unwrap();
    let observed_hits_new_i32 = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_hits_new_i32} = trunc i64 {observed_hits_new} to i32"
    )
    .unwrap();
    writeln!(
        out,
        "  store i32 {observed_hits_new_i32}, i32* {candidate_hits_slot}"
    )
    .unwrap();
    let observed_last_seen_ptr = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_last_seen_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {scan_index_i64}",
        MAX_PROFILED_CALL_CACHE_KEYS,
        MAX_PROFILED_CALL_CACHE_KEYS,
        globals.profile_observed_last_seen
    )
    .unwrap();
    let observed_last_seen_old = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_last_seen_old} = load i64, i64* {observed_last_seen_ptr}"
    )
    .unwrap();
    let observed_reuse = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_reuse} = sub i64 {calls_new}, {observed_last_seen_old}"
    )
    .unwrap();
    let observed_reuse_ptr = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_reuse_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {scan_index_i64}",
        MAX_PROFILED_CALL_CACHE_KEYS,
        MAX_PROFILED_CALL_CACHE_KEYS,
        globals.profile_observed_reuse
    )
    .unwrap();
    writeln!(
        out,
        "  store i64 {observed_reuse}, i64* {observed_reuse_ptr}"
    )
    .unwrap();
    writeln!(
        out,
        "  store i64 {observed_reuse}, i64* {candidate_reuse_slot}"
    )
    .unwrap();
    emit_call_cache_reuse_profile_update(out, next_temp, globals, &observed_reuse);
    writeln!(
        out,
        "  store i64 {calls_new}, i64* {observed_last_seen_ptr}"
    )
    .unwrap();
    emit_update_i64_global_max(
        out,
        next_temp,
        &globals.profile_hottest_key_hits,
        &observed_hits_new,
    );
    writeln!(out, "  br label %{done_label}").unwrap();

    writeln!(out, "{next_label}:").unwrap();
    writeln!(out, "  {next_index} = add nuw i32 {scan_index}, 1").unwrap();
    writeln!(out, "  br label %{scan_label}").unwrap();

    writeln!(out, "{insert_check_label}:").unwrap();
    let observed_has_room = fresh_call_cache_temp(next_temp);
    let victim_init_label = fresh_call_cache_label(next_temp, "observe_victim_init");
    let victim_scan_label = fresh_call_cache_label(next_temp, "observe_victim_scan");
    let victim_check_label = fresh_call_cache_label(next_temp, "observe_victim_check");
    let victim_replace_label = fresh_call_cache_label(next_temp, "observe_victim_replace");
    let victim_next_label = fresh_call_cache_label(next_temp, "observe_victim_next");
    let victim_done_label = fresh_call_cache_label(next_temp, "observe_victim_done");
    let insert_commit_label = fresh_call_cache_label(next_temp, "observe_insert_commit");
    writeln!(
        out,
        "  {observed_has_room} = icmp ult i32 {observed_len}, {}",
        MAX_PROFILED_CALL_CACHE_KEYS as u32
    )
    .unwrap();
    writeln!(
        out,
        "  br i1 {observed_has_room}, label %{insert_label}, label %{victim_init_label}"
    )
    .unwrap();

    writeln!(out, "{victim_init_label}:").unwrap();
    writeln!(out, "  store i32 0, i32* {victim_index_slot}").unwrap();
    let first_hits_ptr = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {first_hits_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 0",
        MAX_PROFILED_CALL_CACHE_KEYS, MAX_PROFILED_CALL_CACHE_KEYS, globals.profile_observed_hits
    )
    .unwrap();
    let first_hits = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {first_hits} = load i64, i64* {first_hits_ptr}").unwrap();
    writeln!(out, "  store i64 {first_hits}, i64* {victim_hits_slot}").unwrap();
    let first_last_seen_ptr = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {first_last_seen_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 0",
        MAX_PROFILED_CALL_CACHE_KEYS,
        MAX_PROFILED_CALL_CACHE_KEYS,
        globals.profile_observed_last_seen
    )
    .unwrap();
    let first_last_seen = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {first_last_seen} = load i64, i64* {first_last_seen_ptr}"
    )
    .unwrap();
    writeln!(
        out,
        "  store i64 {first_last_seen}, i64* {victim_last_seen_slot}"
    )
    .unwrap();
    writeln!(out, "  store i32 1, i32* {victim_scan_index_slot}").unwrap();
    writeln!(out, "  br label %{victim_scan_label}").unwrap();

    writeln!(out, "{victim_scan_label}:").unwrap();
    let victim_scan_index = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {victim_scan_index} = load i32, i32* {victim_scan_index_slot}"
    )
    .unwrap();
    let victim_scan_done = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {victim_scan_done} = icmp eq i32 {victim_scan_index}, {observed_len}"
    )
    .unwrap();
    writeln!(
        out,
        "  br i1 {victim_scan_done}, label %{victim_done_label}, label %{victim_check_label}"
    )
    .unwrap();

    writeln!(out, "{victim_check_label}:").unwrap();
    let victim_scan_index_i64 = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {victim_scan_index_i64} = zext i32 {victim_scan_index} to i64"
    )
    .unwrap();
    let victim_hits_ptr = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {victim_hits_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {victim_scan_index_i64}",
        MAX_PROFILED_CALL_CACHE_KEYS,
        MAX_PROFILED_CALL_CACHE_KEYS,
        globals.profile_observed_hits
    )
    .unwrap();
    let victim_hits = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {victim_hits} = load i64, i64* {victim_hits_ptr}").unwrap();
    let best_hits = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {best_hits} = load i64, i64* {victim_hits_slot}").unwrap();
    let lower_hits = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {lower_hits} = icmp ult i64 {victim_hits}, {best_hits}"
    )
    .unwrap();
    let same_hits = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {same_hits} = icmp eq i64 {victim_hits}, {best_hits}"
    )
    .unwrap();
    let victim_last_seen_ptr = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {victim_last_seen_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {victim_scan_index_i64}",
        MAX_PROFILED_CALL_CACHE_KEYS,
        MAX_PROFILED_CALL_CACHE_KEYS,
        globals.profile_observed_last_seen
    )
    .unwrap();
    let victim_last_seen = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {victim_last_seen} = load i64, i64* {victim_last_seen_ptr}"
    )
    .unwrap();
    let best_last_seen = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {best_last_seen} = load i64, i64* {victim_last_seen_slot}"
    )
    .unwrap();
    let older_victim = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {older_victim} = icmp ult i64 {victim_last_seen}, {best_last_seen}"
    )
    .unwrap();
    let same_hits_older = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {same_hits_older} = and i1 {same_hits}, {older_victim}"
    )
    .unwrap();
    let replace_victim = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {replace_victim} = or i1 {lower_hits}, {same_hits_older}"
    )
    .unwrap();
    writeln!(
        out,
        "  br i1 {replace_victim}, label %{victim_replace_label}, label %{victim_next_label}"
    )
    .unwrap();

    writeln!(out, "{victim_replace_label}:").unwrap();
    writeln!(
        out,
        "  store i32 {victim_scan_index}, i32* {victim_index_slot}"
    )
    .unwrap();
    writeln!(out, "  store i64 {victim_hits}, i64* {victim_hits_slot}").unwrap();
    writeln!(
        out,
        "  store i64 {victim_last_seen}, i64* {victim_last_seen_slot}"
    )
    .unwrap();
    writeln!(out, "  br label %{victim_next_label}").unwrap();

    writeln!(out, "{victim_next_label}:").unwrap();
    let next_victim_index = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {next_victim_index} = add nuw i32 {victim_scan_index}, 1"
    )
    .unwrap();
    writeln!(
        out,
        "  store i32 {next_victim_index}, i32* {victim_scan_index_slot}"
    )
    .unwrap();
    writeln!(out, "  br label %{victim_scan_label}").unwrap();

    writeln!(out, "{victim_done_label}:").unwrap();
    let victim_index = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {victim_index} = load i32, i32* {victim_index_slot}").unwrap();
    writeln!(out, "  br label %{insert_label}").unwrap();

    writeln!(out, "{insert_label}:").unwrap();
    let observed_index = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_index} = phi i32 [ {observed_len}, %{insert_check_label} ], [ {victim_index}, %{victim_done_label} ]"
    )
    .unwrap();
    let observed_index_i64 = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_index_i64} = zext i32 {observed_index} to i64"
    )
    .unwrap();
    for (arg_index, arg_bits) in encoded_args.iter().enumerate() {
        let arg_ptr = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {arg_ptr} = getelementptr inbounds [{} x [{} x i64]], [{} x [{} x i64]]* {}, i64 0, i64 {observed_index_i64}, i64 {}",
            MAX_PROFILED_CALL_CACHE_KEYS,
            MAX_CALL_CACHE_ARGS,
            MAX_PROFILED_CALL_CACHE_KEYS,
            MAX_CALL_CACHE_ARGS,
            globals.profile_observed_keys,
            arg_index
        )
        .unwrap();
        writeln!(out, "  store i64 {arg_bits}, i64* {arg_ptr}").unwrap();
    }
    let observed_hits_ptr = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_hits_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {observed_index_i64}",
        MAX_PROFILED_CALL_CACHE_KEYS,
        MAX_PROFILED_CALL_CACHE_KEYS,
        globals.profile_observed_hits
    )
    .unwrap();
    writeln!(out, "  store i64 1, i64* {observed_hits_ptr}").unwrap();
    writeln!(out, "  store i32 1, i32* {candidate_hits_slot}").unwrap();
    let observed_last_seen_ptr = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_last_seen_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {observed_index_i64}",
        MAX_PROFILED_CALL_CACHE_KEYS,
        MAX_PROFILED_CALL_CACHE_KEYS,
        globals.profile_observed_last_seen
    )
    .unwrap();
    writeln!(
        out,
        "  store i64 {calls_new}, i64* {observed_last_seen_ptr}"
    )
    .unwrap();
    let observed_reuse_ptr = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {observed_reuse_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {observed_index_i64}",
        MAX_PROFILED_CALL_CACHE_KEYS,
        MAX_PROFILED_CALL_CACHE_KEYS,
        globals.profile_observed_reuse
    )
    .unwrap();
    writeln!(out, "  store i64 0, i64* {observed_reuse_ptr}").unwrap();
    writeln!(out, "  store i64 0, i64* {candidate_reuse_slot}").unwrap();
    emit_update_i64_global_max(out, next_temp, &globals.profile_hottest_key_hits, "1");
    writeln!(
        out,
        "  br i1 {observed_has_room}, label %{insert_commit_label}, label %{done_label}"
    )
    .unwrap();

    writeln!(out, "{insert_commit_label}:").unwrap();
    let observed_len_next = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {observed_len_next} = add nuw i32 {observed_len}, 1").unwrap();
    writeln!(
        out,
        "  store i32 {observed_len_next}, i32* {}",
        globals.profile_unique_keys
    )
    .unwrap();
    writeln!(out, "  br label %{done_label}").unwrap();

    writeln!(out, "{done_label}:").unwrap();
    let candidate_hits = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {candidate_hits} = load i32, i32* {candidate_hits_slot}"
    )
    .unwrap();
    let candidate_reuse = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {candidate_reuse} = load i64, i64* {candidate_reuse_slot}"
    )
    .unwrap();
    (candidate_hits, candidate_reuse)
}

fn emit_specialization_feedback_adjusted_score(
    out: &mut String,
    next_temp: &mut usize,
    globals: &CallCacheGlobalNames,
    base_score: &str,
) -> String {
    let hits = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {hits} = load i64, i64* {}",
        globals.profile_specialization_hits
    )
    .unwrap();
    let fallbacks = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {fallbacks} = load i64, i64* {}",
        globals.profile_specialization_fallbacks
    )
    .unwrap();
    let attempts = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {attempts} = add i64 {hits}, {fallbacks}").unwrap();
    let enough_samples = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {enough_samples} = icmp uge i64 {attempts}, {}",
        agam_profile::SPECIALIZATION_FEEDBACK_MIN_ATTEMPTS
    )
    .unwrap();
    let favorable_weighted_hits = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {favorable_weighted_hits} = mul i64 {hits}, {}",
        agam_profile::SPECIALIZATION_FEEDBACK_FAVORABLE_MULTIPLIER
    )
    .unwrap();
    let favorable = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {favorable} = icmp uge i64 {favorable_weighted_hits}, {attempts}"
    )
    .unwrap();
    let no_hits = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {no_hits} = icmp eq i64 {hits}, 0").unwrap();
    let unfavorable_weighted_hits = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {unfavorable_weighted_hits} = mul i64 {hits}, {}",
        agam_profile::SPECIALIZATION_FEEDBACK_UNFAVORABLE_MULTIPLIER
    )
    .unwrap();
    let weak_matches = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {weak_matches} = icmp ult i64 {unfavorable_weighted_hits}, {attempts}"
    )
    .unwrap();
    let unfavorable = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {unfavorable} = or i1 {no_hits}, {weak_matches}").unwrap();
    let favorable_with_samples = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {favorable_with_samples} = and i1 {enough_samples}, {favorable}"
    )
    .unwrap();
    let unfavorable_with_samples = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {unfavorable_with_samples} = and i1 {enough_samples}, {unfavorable}"
    )
    .unwrap();
    let favored_score = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {favored_score} = add i32 {base_score}, {}",
        agam_profile::SPECIALIZATION_FEEDBACK_FAVORABLE_SCORE_BONUS
    )
    .unwrap();
    let penalty_applies = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {penalty_applies} = icmp uge i32 {base_score}, {}",
        agam_profile::SPECIALIZATION_FEEDBACK_UNFAVORABLE_SCORE_PENALTY
    )
    .unwrap();
    let penalized_sub = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {penalized_sub} = sub i32 {base_score}, {}",
        agam_profile::SPECIALIZATION_FEEDBACK_UNFAVORABLE_SCORE_PENALTY
    )
    .unwrap();
    let penalized_score = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {penalized_score} = select i1 {penalty_applies}, i32 {penalized_sub}, i32 0"
    )
    .unwrap();
    let after_unfavorable = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {after_unfavorable} = select i1 {unfavorable_with_samples}, i32 {penalized_score}, i32 {base_score}"
    )
    .unwrap();
    let adjusted_score = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {adjusted_score} = select i1 {favorable_with_samples}, i32 {favored_score}, i32 {after_unfavorable}"
    )
    .unwrap();
    adjusted_score
}

fn emit_adaptive_stable_argument_slot_count(
    out: &mut String,
    next_temp: &mut usize,
    globals: &CallCacheGlobalNames,
    param_count: usize,
    total_calls: &str,
) -> String {
    if param_count == 0 {
        return "0".into();
    }

    let enough_calls = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {enough_calls} = icmp uge i64 {total_calls}, {}",
        agam_profile::ADAPTIVE_ADMISSION_MIN_CALLS_FOR_DOMINANT_HOT_KEY
    )
    .unwrap();
    let stable_call_target = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {stable_call_target} = mul i64 {total_calls}, 3").unwrap();

    let mut stable_slots = String::from("0");
    for arg_index in 0..param_count {
        let stable_matches_ptr = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {stable_matches_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {}",
            MAX_CALL_CACHE_ARGS,
            MAX_CALL_CACHE_ARGS,
            globals.profile_stable_matches,
            arg_index
        )
        .unwrap();
        let stable_matches = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {stable_matches} = load i64, i64* {stable_matches_ptr}"
        )
        .unwrap();
        let weighted_stable_matches = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {weighted_stable_matches} = mul i64 {stable_matches}, 4"
        )
        .unwrap();
        let stable_enough = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {stable_enough} = icmp uge i64 {weighted_stable_matches}, {stable_call_target}"
        )
        .unwrap();
        let stable_slot = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {stable_slot} = and i1 {enough_calls}, {stable_enough}"
        )
        .unwrap();
        let next_stable_slots = fresh_call_cache_temp(next_temp);
        writeln!(out, "  {next_stable_slots} = add i32 {stable_slots}, 1").unwrap();
        let stable_slots_value = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {stable_slots_value} = select i1 {stable_slot}, i32 {next_stable_slots}, i32 {stable_slots}"
        )
        .unwrap();
        stable_slots = stable_slots_value;
    }

    stable_slots
}

fn emit_optimized_admission_score(
    out: &mut String,
    next_temp: &mut usize,
    layout: &FunctionLayout,
    globals: &CallCacheGlobalNames,
    capacity: usize,
    total_calls: &str,
    cached_entries: &str,
    candidate_hits: &str,
    candidate_reuse_distance: &str,
) -> String {
    let repeated_enough = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {repeated_enough} = icmp uge i32 {candidate_hits}, {}",
        agam_profile::ADAPTIVE_ADMISSION_MIN_CANDIDATE_HITS as u32
    )
    .unwrap();
    let repeated_score = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {repeated_score} = select i1 {repeated_enough}, i32 {}, i32 0",
        agam_profile::ADAPTIVE_ADMISSION_REPEATED_ARGUMENTS_SCORE
    )
    .unwrap();

    let candidate_hot = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {candidate_hot} = icmp uge i32 {candidate_hits}, {}",
        agam_profile::ADAPTIVE_ADMISSION_HOT_CANDIDATE_HITS as u32
    )
    .unwrap();
    let hot_bonus = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {hot_bonus} = select i1 {candidate_hot}, i32 {}, i32 0",
        agam_profile::ADAPTIVE_ADMISSION_ALREADY_HOT_SCORE
    )
    .unwrap();
    let score_after_hot = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {score_after_hot} = add i32 {repeated_score}, {hot_bonus}"
    )
    .unwrap();

    let short_reuse = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {short_reuse} = icmp ule i64 {candidate_reuse_distance}, {}",
        (capacity.max(2) as u64) * agam_profile::ADAPTIVE_ADMISSION_SHORT_REUSE_WINDOW_MULTIPLIER
    )
    .unwrap();
    let medium_reuse = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {medium_reuse} = icmp ule i64 {candidate_reuse_distance}, {}",
        (capacity.max(2) as u64) * agam_profile::ADAPTIVE_ADMISSION_MEDIUM_REUSE_WINDOW_MULTIPLIER
    )
    .unwrap();
    let medium_reuse_bonus = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {medium_reuse_bonus} = select i1 {medium_reuse}, i32 {}, i32 0",
        agam_profile::ADAPTIVE_ADMISSION_MEDIUM_REUSE_SCORE
    )
    .unwrap();
    let reuse_bonus = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {reuse_bonus} = select i1 {short_reuse}, i32 {}, i32 {medium_reuse_bonus}",
        agam_profile::ADAPTIVE_ADMISSION_SHORT_REUSE_SCORE
    )
    .unwrap();
    let score_after_reuse = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {score_after_reuse} = add i32 {score_after_hot}, {reuse_bonus}"
    )
    .unwrap();

    let hits_total = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {hits_total} = load i64, i64* {}", globals.hits).unwrap();
    let hit_rate_num = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {hit_rate_num} = mul i64 {hits_total}, 1000").unwrap();
    let hit_per_thousand = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {hit_per_thousand} = udiv i64 {hit_rate_num}, {total_calls}"
    )
    .unwrap();
    let strong_hit_rate = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {strong_hit_rate} = icmp uge i64 {hit_per_thousand}, {}",
        agam_profile::ADAPTIVE_ADMISSION_STRONG_HIT_RATE_PER_THOUSAND
    )
    .unwrap();
    let growing_hit_rate = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {growing_hit_rate} = icmp uge i64 {hit_per_thousand}, {}",
        agam_profile::ADAPTIVE_ADMISSION_GROWING_HIT_RATE_PER_THOUSAND
    )
    .unwrap();
    let growing_hit_rate_bonus = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {growing_hit_rate_bonus} = select i1 {growing_hit_rate}, i32 {}, i32 0",
        agam_profile::ADAPTIVE_ADMISSION_GROWING_HIT_RATE_SCORE
    )
    .unwrap();
    let hit_rate_bonus = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {hit_rate_bonus} = select i1 {strong_hit_rate}, i32 {}, i32 {growing_hit_rate_bonus}",
        agam_profile::ADAPTIVE_ADMISSION_STRONG_HIT_RATE_SCORE
    )
    .unwrap();
    let score_after_hit_rate = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {score_after_hit_rate} = add i32 {score_after_reuse}, {hit_rate_bonus}"
    )
    .unwrap();

    let hottest_key_hits = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {hottest_key_hits} = load i64, i64* {}",
        globals.profile_hottest_key_hits
    )
    .unwrap();
    let dominant_hits = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {dominant_hits} = mul i64 {hottest_key_hits}, 2").unwrap();
    let dominant_hot_key = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {dominant_hot_key} = icmp uge i64 {dominant_hits}, {total_calls}"
    )
    .unwrap();
    let enough_calls_for_hot_key = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {enough_calls_for_hot_key} = icmp uge i64 {total_calls}, {}",
        agam_profile::ADAPTIVE_ADMISSION_MIN_CALLS_FOR_DOMINANT_HOT_KEY
    )
    .unwrap();
    let dominant_hot_key_with_calls = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {dominant_hot_key_with_calls} = and i1 {enough_calls_for_hot_key}, {dominant_hot_key}"
    )
    .unwrap();
    let dominant_hot_key_bonus = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {dominant_hot_key_bonus} = select i1 {dominant_hot_key_with_calls}, i32 {}, i32 0",
        agam_profile::ADAPTIVE_ADMISSION_DOMINANT_HOT_KEY_SCORE
    )
    .unwrap();
    let score_after_hot_key = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {score_after_hot_key} = add i32 {score_after_hit_rate}, {dominant_hot_key_bonus}"
    )
    .unwrap();

    let stable_slots = emit_adaptive_stable_argument_slot_count(
        out,
        next_temp,
        globals,
        layout.params.len(),
        total_calls,
    );
    let stable_slots_over_cap = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {stable_slots_over_cap} = icmp ugt i32 {stable_slots}, 2"
    )
    .unwrap();
    let stable_slots_capped = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {stable_slots_capped} = select i1 {stable_slots_over_cap}, i32 2, i32 {stable_slots}"
    )
    .unwrap();
    let stable_slots_bonus = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {stable_slots_bonus} = mul i32 {stable_slots_capped}, {}",
        agam_profile::ADAPTIVE_ADMISSION_STABLE_ARGUMENT_SLOT_SCORE
    )
    .unwrap();
    let score_after_stable = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {score_after_stable} = add i32 {score_after_hot_key}, {stable_slots_bonus}"
    )
    .unwrap();

    let unique_keys = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {unique_keys} = load i32, i32* {}",
        globals.profile_unique_keys
    )
    .unwrap();
    let broad_unique_spread = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {broad_unique_spread} = icmp ugt i32 {unique_keys}, {}",
        capacity.saturating_mul(2) as u32
    )
    .unwrap();
    let spread_penalty_applies = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {spread_penalty_applies} = icmp uge i32 {score_after_stable}, {}",
        agam_profile::ADAPTIVE_ADMISSION_BROAD_KEY_SPREAD_PENALTY
    )
    .unwrap();
    let spread_penalty_sub = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {spread_penalty_sub} = sub i32 {score_after_stable}, {}",
        agam_profile::ADAPTIVE_ADMISSION_BROAD_KEY_SPREAD_PENALTY
    )
    .unwrap();
    let spread_penalized = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {spread_penalized} = select i1 {spread_penalty_applies}, i32 {spread_penalty_sub}, i32 0"
    )
    .unwrap();
    let score_after_spread = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {score_after_spread} = select i1 {broad_unique_spread}, i32 {spread_penalized}, i32 {score_after_stable}"
    )
    .unwrap();

    let score_after_capacity = if capacity > 0 {
        let cache_full = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {cache_full} = icmp uge i32 {cached_entries}, {}",
            capacity as u32
        )
        .unwrap();
        let full_penalty_applies = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {full_penalty_applies} = icmp uge i32 {score_after_spread}, {}",
            agam_profile::ADAPTIVE_ADMISSION_CACHE_FULL_PENALTY
        )
        .unwrap();
        let full_penalty_sub = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {full_penalty_sub} = sub i32 {score_after_spread}, {}",
            agam_profile::ADAPTIVE_ADMISSION_CACHE_FULL_PENALTY
        )
        .unwrap();
        let full_penalized = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {full_penalized} = select i1 {full_penalty_applies}, i32 {full_penalty_sub}, i32 0"
        )
        .unwrap();
        let score_after_capacity = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {score_after_capacity} = select i1 {cache_full}, i32 {full_penalized}, i32 {score_after_spread}"
        )
        .unwrap();
        score_after_capacity
    } else {
        score_after_spread
    };

    emit_specialization_feedback_adjusted_score(out, next_temp, globals, &score_after_capacity)
}

fn emit_specialized_call_or_fallback(
    out: &mut String,
    next_temp: &mut usize,
    layout: &FunctionLayout,
    original: &str,
    call_args: &str,
    encoded_args: &[String],
    globals: &CallCacheGlobalNames,
    specializations: &[LlvmFunctionSpecialization],
) -> Result<String, String> {
    if specializations.is_empty() {
        let miss_value = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {miss_value} = call noundef {} @{}({})",
            layout.return_ty.ir(),
            original,
            call_args
        )
        .unwrap();
        return Ok(miss_value);
    }

    let applicable_specializations: Vec<_> = specializations
        .iter()
        .filter(|specialization| {
            !specialization.stable_values.is_empty()
                && specialization
                    .stable_values
                    .iter()
                    .all(|value| encoded_args.get(value.index).is_some())
        })
        .collect();
    if applicable_specializations.is_empty() {
        let miss_value = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {miss_value} = call noundef {} @{}({})",
            layout.return_ty.ir(),
            original,
            call_args
        )
        .unwrap();
        return Ok(miss_value);
    }

    let generic_label = fresh_call_cache_label(next_temp, "generic_call");
    let cont_label = fresh_call_cache_label(next_temp, "specialized_cont");
    let mut incoming_values = Vec::new();

    for (index, specialization) in applicable_specializations.iter().enumerate() {
        let mut guard = String::new();
        for stable in &specialization.stable_values {
            let expected = llvm_i64_literal(stable.raw_bits);
            let arg_eq = fresh_call_cache_temp(next_temp);
            writeln!(
                out,
                "  {arg_eq} = icmp eq i64 {}, {}",
                encoded_args[stable.index], expected
            )
            .unwrap();
            if guard.is_empty() {
                guard = arg_eq;
            } else {
                let combined = fresh_call_cache_temp(next_temp);
                writeln!(out, "  {combined} = and i1 {guard}, {arg_eq}").unwrap();
                guard = combined;
            }
        }

        let specialized_label = fresh_call_cache_label(next_temp, "spec_call");
        let fail_label = if index + 1 < applicable_specializations.len() {
            fresh_call_cache_label(next_temp, "spec_next")
        } else {
            generic_label.clone()
        };
        writeln!(
            out,
            "  br i1 {guard}, label %{specialized_label}, label %{fail_label}"
        )
        .unwrap();

        writeln!(out, "{specialized_label}:").unwrap();
        emit_increment_i64_global(out, next_temp, &globals.profile_specialization_hits);
        let specialized_value = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {specialized_value} = call noundef {} @{}({})",
            layout.return_ty.ir(),
            specialization.clone_name,
            call_args
        )
        .unwrap();
        writeln!(out, "  br label %{cont_label}").unwrap();
        incoming_values.push((specialized_value, specialized_label));

        if fail_label != generic_label {
            writeln!(out, "{fail_label}:").unwrap();
        }
    }

    writeln!(out, "{generic_label}:").unwrap();
    emit_increment_i64_global(out, next_temp, &globals.profile_specialization_fallbacks);
    let generic_value = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {generic_value} = call noundef {} @{}({})",
        layout.return_ty.ir(),
        original,
        call_args
    )
    .unwrap();
    writeln!(out, "  br label %{cont_label}").unwrap();

    writeln!(out, "{cont_label}:").unwrap();
    let miss_value = fresh_call_cache_temp(next_temp);
    let mut phi_incoming = incoming_values
        .into_iter()
        .map(|(value, label)| format!("[ {value}, %{label} ]"))
        .collect::<Vec<_>>();
    phi_incoming.push(format!("[ {generic_value}, %{generic_label} ]"));
    writeln!(
        out,
        "  {miss_value} = phi {} {}",
        layout.return_ty.ir(),
        phi_incoming.join(", ")
    )
    .unwrap();
    Ok(miss_value)
}

fn emit_call_cache_stable_value_profile_update(
    out: &mut String,
    next_temp: &mut usize,
    globals: &CallCacheGlobalNames,
    encoded_args: &[String],
) -> String {
    let mut current_label = "entry".to_string();
    for (arg_index, arg_bits) in encoded_args.iter().enumerate() {
        let seed_label = fresh_call_cache_label(next_temp, "stable_seed");
        let compare_label = fresh_call_cache_label(next_temp, "stable_compare");
        let hit_label = fresh_call_cache_label(next_temp, "stable_hit");
        let miss_label = fresh_call_cache_label(next_temp, "stable_miss");
        let done_label = fresh_call_cache_label(next_temp, "stable_done");

        let value_ptr = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {value_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {}",
            MAX_CALL_CACHE_ARGS, MAX_CALL_CACHE_ARGS, globals.profile_stable_values, arg_index
        )
        .unwrap();
        let score_ptr = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {score_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {}",
            MAX_CALL_CACHE_ARGS, MAX_CALL_CACHE_ARGS, globals.profile_stable_scores, arg_index
        )
        .unwrap();
        let matches_ptr = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {matches_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {}",
            MAX_CALL_CACHE_ARGS, MAX_CALL_CACHE_ARGS, globals.profile_stable_matches, arg_index
        )
        .unwrap();
        let score_old = fresh_call_cache_temp(next_temp);
        writeln!(out, "  {score_old} = load i64, i64* {score_ptr}").unwrap();
        let score_is_zero = fresh_call_cache_temp(next_temp);
        writeln!(out, "  {score_is_zero} = icmp eq i64 {score_old}, 0").unwrap();
        writeln!(
            out,
            "  br i1 {score_is_zero}, label %{seed_label}, label %{compare_label}"
        )
        .unwrap();

        writeln!(out, "{seed_label}:").unwrap();
        writeln!(out, "  store i64 {arg_bits}, i64* {value_ptr}").unwrap();
        writeln!(out, "  store i64 1, i64* {score_ptr}").unwrap();
        writeln!(out, "  store i64 1, i64* {matches_ptr}").unwrap();
        writeln!(out, "  br label %{done_label}").unwrap();

        writeln!(out, "{compare_label}:").unwrap();
        let value_old = fresh_call_cache_temp(next_temp);
        writeln!(out, "  {value_old} = load i64, i64* {value_ptr}").unwrap();
        let matches = fresh_call_cache_temp(next_temp);
        writeln!(out, "  {matches} = icmp eq i64 {value_old}, {arg_bits}").unwrap();
        writeln!(
            out,
            "  br i1 {matches}, label %{hit_label}, label %{miss_label}"
        )
        .unwrap();

        writeln!(out, "{hit_label}:").unwrap();
        let score_hit = fresh_call_cache_temp(next_temp);
        writeln!(out, "  {score_hit} = add i64 {score_old}, 1").unwrap();
        writeln!(out, "  store i64 {score_hit}, i64* {score_ptr}").unwrap();
        let matches_old = fresh_call_cache_temp(next_temp);
        writeln!(out, "  {matches_old} = load i64, i64* {matches_ptr}").unwrap();
        let matches_hit = fresh_call_cache_temp(next_temp);
        writeln!(out, "  {matches_hit} = add i64 {matches_old}, 1").unwrap();
        writeln!(out, "  store i64 {matches_hit}, i64* {matches_ptr}").unwrap();
        writeln!(out, "  br label %{done_label}").unwrap();

        writeln!(out, "{miss_label}:").unwrap();
        let score_miss = fresh_call_cache_temp(next_temp);
        writeln!(out, "  {score_miss} = sub i64 {score_old}, 1").unwrap();
        writeln!(out, "  store i64 {score_miss}, i64* {score_ptr}").unwrap();
        let matches_old = fresh_call_cache_temp(next_temp);
        writeln!(out, "  {matches_old} = load i64, i64* {matches_ptr}").unwrap();
        let candidate_extinguished = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {candidate_extinguished} = icmp eq i64 {score_miss}, 0"
        )
        .unwrap();
        let matches_next = fresh_call_cache_temp(next_temp);
        writeln!(
            out,
            "  {matches_next} = select i1 {candidate_extinguished}, i64 0, i64 {matches_old}"
        )
        .unwrap();
        writeln!(out, "  store i64 {matches_next}, i64* {matches_ptr}").unwrap();
        writeln!(out, "  br label %{done_label}").unwrap();

        writeln!(out, "{done_label}:").unwrap();
        current_label = done_label;
    }

    current_label
}

fn emit_call_cache_reuse_profile_update(
    out: &mut String,
    next_temp: &mut usize,
    globals: &CallCacheGlobalNames,
    reuse: &str,
) {
    let reuse_total_old = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {reuse_total_old} = load i64, i64* {}",
        globals.profile_reuse_total
    )
    .unwrap();
    let reuse_total_new = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {reuse_total_new} = add i64 {reuse_total_old}, {reuse}"
    )
    .unwrap();
    writeln!(
        out,
        "  store i64 {reuse_total_new}, i64* {}",
        globals.profile_reuse_total
    )
    .unwrap();
    let reuse_samples_old = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {reuse_samples_old} = load i64, i64* {}",
        globals.profile_reuse_samples
    )
    .unwrap();
    let reuse_samples_new = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {reuse_samples_new} = add i64 {reuse_samples_old}, 1"
    )
    .unwrap();
    writeln!(
        out,
        "  store i64 {reuse_samples_new}, i64* {}",
        globals.profile_reuse_samples
    )
    .unwrap();
    let reuse_max_old = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {reuse_max_old} = load i64, i64* {}",
        globals.profile_reuse_max
    )
    .unwrap();
    let reuse_gt_max = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {reuse_gt_max} = icmp ugt i64 {reuse}, {reuse_max_old}"
    )
    .unwrap();
    let reuse_max_new = fresh_call_cache_temp(next_temp);
    writeln!(
        out,
        "  {reuse_max_new} = select i1 {reuse_gt_max}, i64 {reuse}, i64 {reuse_max_old}"
    )
    .unwrap();
    writeln!(
        out,
        "  store i64 {reuse_max_new}, i64* {}",
        globals.profile_reuse_max
    )
    .unwrap();
}

fn emit_call_cache_reuse_profile_update_from_age(
    out: &mut String,
    next_temp: &mut usize,
    globals: &CallCacheGlobalNames,
    age_ptr: &str,
    calls_new: &str,
) {
    let age_old = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {age_old} = load i64, i64* {age_ptr}").unwrap();
    let reuse = fresh_call_cache_temp(next_temp);
    writeln!(out, "  {reuse} = sub i64 {calls_new}, {age_old}").unwrap();
    emit_call_cache_reuse_profile_update(out, next_temp, globals, &reuse);
}

fn emit_basic_call_cache_wrapper_ir(
    out: &mut String,
    name: &str,
    layout: &FunctionLayout,
    capacity: usize,
    warmup: u64,
    specializations: &[LlvmFunctionSpecialization],
) -> Result<(), String> {
    let wrapper = call_cache_wrapper_name(name);
    let original = mangle_name(name);
    let globals = call_cache_global_names(name);
    let params = layout
        .params
        .iter()
        .enumerate()
        .map(|(index, ty)| format!("{} noundef %p{index}", ty.ir()))
        .collect::<Vec<_>>()
        .join(", ");
    let call_args = layout
        .params
        .iter()
        .enumerate()
        .map(|(index, ty)| format!("{} %p{index}", ty.ir()))
        .collect::<Vec<_>>()
        .join(", ");

    let mut next_temp = 0usize;
    writeln!(
        out,
        "define noundef {} @{}({}) local_unnamed_addr {{",
        layout.return_ty.ir(),
        wrapper,
        params
    )
    .unwrap();
    writeln!(out, "entry:").unwrap();

    let calls_old = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {calls_old} = load i64, i64* {}", globals.calls).unwrap();
    let calls_new = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {calls_new} = add i64 {calls_old}, 1").unwrap();
    writeln!(out, "  store i64 {calls_new}, i64* {}", globals.calls).unwrap();

    let mut encoded_args = Vec::with_capacity(layout.params.len());
    for (index, ty) in layout.params.iter().enumerate() {
        encoded_args.push(emit_call_cache_encode_value(
            out,
            &mut next_temp,
            *ty,
            &format!("%p{index}"),
        )?);
    }
    let observe_predecessor =
        emit_call_cache_stable_value_profile_update(out, &mut next_temp, &globals, &encoded_args);
    let _ = emit_call_cache_observe_key(
        out,
        &mut next_temp,
        &globals,
        &observe_predecessor,
        &encoded_args,
        &calls_new,
    );

    let use_cache = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {use_cache} = icmp ule i64 {calls_new}, {warmup}").unwrap();
    writeln!(out, "  br i1 {use_cache}, label %miss, label %lookup").unwrap();

    writeln!(out, "lookup:").unwrap();
    let len_loaded = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {len_loaded} = load i32, i32* {}", globals.len).unwrap();
    writeln!(out, "  br label %scan").unwrap();

    let next_index = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "scan:").unwrap();
    let scan_index = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {scan_index} = phi i32 [ 0, %lookup ], [ {next_index}, %next ]"
    )
    .unwrap();
    let scan_done = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {scan_done} = icmp eq i32 {scan_index}, {len_loaded}"
    )
    .unwrap();
    writeln!(out, "  br i1 {scan_done}, label %miss, label %check").unwrap();

    writeln!(out, "check:").unwrap();
    let scan_index_i64 = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {scan_index_i64} = sext i32 {scan_index} to i64").unwrap();
    if encoded_args.is_empty() {
        writeln!(out, "  br label %hit").unwrap();
    } else {
        let mut combined_match = String::new();
        for (arg_index, arg_bits) in encoded_args.iter().enumerate() {
            let arg_ptr = fresh_call_cache_temp(&mut next_temp);
            writeln!(
                out,
                "  {arg_ptr} = getelementptr inbounds [{} x [{} x i64]], [{} x [{} x i64]]* {}, i64 0, i64 {scan_index_i64}, i64 {}",
                capacity,
                MAX_CALL_CACHE_ARGS,
                capacity,
                MAX_CALL_CACHE_ARGS,
                globals.keys,
                arg_index
            )
            .unwrap();
            let arg_loaded = fresh_call_cache_temp(&mut next_temp);
            writeln!(out, "  {arg_loaded} = load i64, i64* {arg_ptr}").unwrap();
            let arg_eq = fresh_call_cache_temp(&mut next_temp);
            writeln!(out, "  {arg_eq} = icmp eq i64 {arg_loaded}, {arg_bits}").unwrap();
            if combined_match.is_empty() {
                combined_match = arg_eq;
            } else {
                let new_match = fresh_call_cache_temp(&mut next_temp);
                writeln!(out, "  {new_match} = and i1 {combined_match}, {arg_eq}").unwrap();
                combined_match = new_match;
            }
        }
        writeln!(out, "  br i1 {combined_match}, label %hit, label %next").unwrap();
    }

    writeln!(out, "next:").unwrap();
    writeln!(out, "  {next_index} = add nuw i32 {scan_index}, 1").unwrap();
    writeln!(out, "  br label %scan").unwrap();

    writeln!(out, "hit:").unwrap();
    let cached_value_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {cached_value_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {scan_index_i64}",
        capacity, capacity, globals.values
    )
    .unwrap();
    let cached_bits = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {cached_bits} = load i64, i64* {cached_value_ptr}").unwrap();
    let hit_age_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {hit_age_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {scan_index_i64}",
        capacity, capacity, globals.ages
    )
    .unwrap();
    emit_call_cache_reuse_profile_update_from_age(
        out,
        &mut next_temp,
        &globals,
        &hit_age_ptr,
        &calls_new,
    );
    writeln!(out, "  store i64 {calls_new}, i64* {hit_age_ptr}").unwrap();
    emit_increment_call_cache_entry_hits(out, &mut next_temp, &globals, capacity, &scan_index_i64);
    let hit_count_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {hit_count_ptr} = load i64, i64* {}", globals.hits).unwrap();
    let hit_count_new = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {hit_count_new} = add i64 {hit_count_ptr}, 1").unwrap();
    writeln!(out, "  store i64 {hit_count_new}, i64* {}", globals.hits).unwrap();
    let cached_value =
        emit_call_cache_decode_value(out, &mut next_temp, layout.return_ty, &cached_bits)?;
    writeln!(out, "  ret {} {cached_value}", layout.return_ty.ir()).unwrap();

    writeln!(out, "miss:").unwrap();
    let miss_value = emit_specialized_call_or_fallback(
        out,
        &mut next_temp,
        layout,
        &original,
        &call_args,
        &encoded_args,
        &globals,
        specializations,
    )?;
    let store_enabled = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {store_enabled} = icmp ugt i64 {calls_new}, {warmup}"
    )
    .unwrap();
    writeln!(
        out,
        "  br i1 {store_enabled}, label %store_check, label %ret_miss"
    )
    .unwrap();

    writeln!(out, "store_check:").unwrap();
    let store_len = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {store_len} = load i32, i32* {}", globals.len).unwrap();
    let has_room = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {has_room} = icmp ult i32 {store_len}, {}",
        capacity as u32
    )
    .unwrap();
    writeln!(out, "  br i1 {has_room}, label %store, label %ret_miss").unwrap();

    writeln!(out, "store:").unwrap();
    let store_index_i64 = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {store_index_i64} = zext i32 {store_len} to i64").unwrap();
    for (arg_index, arg_bits) in encoded_args.iter().enumerate() {
        let arg_ptr = fresh_call_cache_temp(&mut next_temp);
        writeln!(
            out,
            "  {arg_ptr} = getelementptr inbounds [{} x [{} x i64]], [{} x [{} x i64]]* {}, i64 0, i64 {store_index_i64}, i64 {}",
            capacity,
            MAX_CALL_CACHE_ARGS,
            capacity,
            MAX_CALL_CACHE_ARGS,
            globals.keys,
            arg_index
        )
        .unwrap();
        writeln!(out, "  store i64 {arg_bits}, i64* {arg_ptr}").unwrap();
    }
    let result_bits =
        emit_call_cache_encode_value(out, &mut next_temp, layout.return_ty, &miss_value)?;
    let store_value_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {store_value_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {store_index_i64}",
        capacity, capacity, globals.values
    )
    .unwrap();
    writeln!(out, "  store i64 {result_bits}, i64* {store_value_ptr}").unwrap();
    let store_age_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {store_age_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {store_index_i64}",
        capacity, capacity, globals.ages
    )
    .unwrap();
    writeln!(out, "  store i64 {calls_new}, i64* {store_age_ptr}").unwrap();
    emit_store_call_cache_entry_hits(
        out,
        &mut next_temp,
        &globals,
        capacity,
        &store_index_i64,
        "1",
    );
    let store_count_old = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {store_count_old} = load i64, i64* {}",
        globals.stores
    )
    .unwrap();
    let store_count_new = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {store_count_new} = add i64 {store_count_old}, 1").unwrap();
    writeln!(
        out,
        "  store i64 {store_count_new}, i64* {}",
        globals.stores
    )
    .unwrap();
    let next_len = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {next_len} = add nuw i32 {store_len}, 1").unwrap();
    writeln!(out, "  store i32 {next_len}, i32* {}", globals.len).unwrap();
    writeln!(out, "  br label %ret_miss").unwrap();

    writeln!(out, "ret_miss:").unwrap();
    writeln!(out, "  ret {} {miss_value}", layout.return_ty.ir()).unwrap();
    writeln!(out, "}}\n").unwrap();

    Ok(())
}

fn emit_optimized_call_cache_wrapper_ir(
    out: &mut String,
    name: &str,
    layout: &FunctionLayout,
    capacity: usize,
    warmup: u64,
    specializations: &[LlvmFunctionSpecialization],
) -> Result<(), String> {
    let wrapper = call_cache_wrapper_name(name);
    let original = mangle_name(name);
    let globals = call_cache_global_names(name);
    let params = layout
        .params
        .iter()
        .enumerate()
        .map(|(index, ty)| format!("{} noundef %p{index}", ty.ir()))
        .collect::<Vec<_>>()
        .join(", ");
    let call_args = layout
        .params
        .iter()
        .enumerate()
        .map(|(index, ty)| format!("{} %p{index}", ty.ir()))
        .collect::<Vec<_>>()
        .join(", ");

    let mut next_temp = 0usize;
    writeln!(
        out,
        "define noundef {} @{}({}) local_unnamed_addr {{",
        layout.return_ty.ir(),
        wrapper,
        params
    )
    .unwrap();
    writeln!(out, "entry:").unwrap();

    let calls_old = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {calls_old} = load i64, i64* {}", globals.calls).unwrap();
    let calls_new = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {calls_new} = add i64 {calls_old}, 1").unwrap();
    writeln!(out, "  store i64 {calls_new}, i64* {}", globals.calls).unwrap();

    let victim_index_slot = fresh_call_cache_temp(&mut next_temp);
    let best_score_slot = fresh_call_cache_temp(&mut next_temp);
    let best_age_slot = fresh_call_cache_temp(&mut next_temp);
    let loop_index_slot = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {victim_index_slot} = alloca i32").unwrap();
    writeln!(out, "  {best_score_slot} = alloca i32").unwrap();
    writeln!(out, "  {best_age_slot} = alloca i64").unwrap();
    writeln!(out, "  {loop_index_slot} = alloca i32").unwrap();

    let mut encoded_args = Vec::with_capacity(layout.params.len());
    for (index, ty) in layout.params.iter().enumerate() {
        encoded_args.push(emit_call_cache_encode_value(
            out,
            &mut next_temp,
            *ty,
            &format!("%p{index}"),
        )?);
    }
    let observe_predecessor =
        emit_call_cache_stable_value_profile_update(out, &mut next_temp, &globals, &encoded_args);
    let (observed_candidate_hits, observed_candidate_reuse) = emit_call_cache_observe_key(
        out,
        &mut next_temp,
        &globals,
        &observe_predecessor,
        &encoded_args,
        &calls_new,
    );

    let use_cache = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {use_cache} = icmp ule i64 {calls_new}, {warmup}").unwrap();
    writeln!(out, "  br i1 {use_cache}, label %miss, label %lookup").unwrap();

    writeln!(out, "lookup:").unwrap();
    let len_loaded = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {len_loaded} = load i32, i32* {}", globals.len).unwrap();
    writeln!(out, "  br label %scan").unwrap();

    let next_index = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "scan:").unwrap();
    let scan_index = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {scan_index} = phi i32 [ 0, %lookup ], [ {next_index}, %next ]"
    )
    .unwrap();
    let scan_done = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {scan_done} = icmp eq i32 {scan_index}, {len_loaded}"
    )
    .unwrap();
    writeln!(out, "  br i1 {scan_done}, label %miss, label %check").unwrap();

    writeln!(out, "check:").unwrap();
    let scan_index_i64 = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {scan_index_i64} = sext i32 {scan_index} to i64").unwrap();
    if encoded_args.is_empty() {
        writeln!(out, "  br label %hit").unwrap();
    } else {
        let mut combined_match = String::new();
        for (arg_index, arg_bits) in encoded_args.iter().enumerate() {
            let arg_ptr = fresh_call_cache_temp(&mut next_temp);
            writeln!(
                out,
                "  {arg_ptr} = getelementptr inbounds [{} x [{} x i64]], [{} x [{} x i64]]* {}, i64 0, i64 {scan_index_i64}, i64 {}",
                capacity,
                MAX_CALL_CACHE_ARGS,
                capacity,
                MAX_CALL_CACHE_ARGS,
                globals.keys,
                arg_index
            )
            .unwrap();
            let arg_loaded = fresh_call_cache_temp(&mut next_temp);
            writeln!(out, "  {arg_loaded} = load i64, i64* {arg_ptr}").unwrap();
            let arg_eq = fresh_call_cache_temp(&mut next_temp);
            writeln!(out, "  {arg_eq} = icmp eq i64 {arg_loaded}, {arg_bits}").unwrap();
            if combined_match.is_empty() {
                combined_match = arg_eq;
            } else {
                let new_match = fresh_call_cache_temp(&mut next_temp);
                writeln!(out, "  {new_match} = and i1 {combined_match}, {arg_eq}").unwrap();
                combined_match = new_match;
            }
        }
        writeln!(out, "  br i1 {combined_match}, label %hit, label %next").unwrap();
    }

    writeln!(out, "next:").unwrap();
    writeln!(out, "  {next_index} = add nuw i32 {scan_index}, 1").unwrap();
    writeln!(out, "  br label %scan").unwrap();

    writeln!(out, "hit:").unwrap();
    let cached_value_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {cached_value_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {scan_index_i64}",
        capacity, capacity, globals.values
    )
    .unwrap();
    let cached_bits = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {cached_bits} = load i64, i64* {cached_value_ptr}").unwrap();
    let hit_score_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {hit_score_ptr} = getelementptr inbounds [{} x i32], [{} x i32]* {}, i64 0, i64 {scan_index_i64}",
        capacity, capacity, globals.scores
    )
    .unwrap();
    let hit_score_old = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {hit_score_old} = load i32, i32* {hit_score_ptr}").unwrap();
    let hit_score_base = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {hit_score_base} = add i32 {hit_score_old}, 1").unwrap();
    let hit_score_new =
        emit_specialization_feedback_adjusted_score(out, &mut next_temp, &globals, &hit_score_base);
    writeln!(out, "  store i32 {hit_score_new}, i32* {hit_score_ptr}").unwrap();
    let hit_age_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {hit_age_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {scan_index_i64}",
        capacity, capacity, globals.ages
    )
    .unwrap();
    emit_call_cache_reuse_profile_update_from_age(
        out,
        &mut next_temp,
        &globals,
        &hit_age_ptr,
        &calls_new,
    );
    writeln!(out, "  store i64 {calls_new}, i64* {hit_age_ptr}").unwrap();
    emit_increment_call_cache_entry_hits(out, &mut next_temp, &globals, capacity, &scan_index_i64);
    let hit_count_old = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {hit_count_old} = load i64, i64* {}", globals.hits).unwrap();
    let hit_count_new = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {hit_count_new} = add i64 {hit_count_old}, 1").unwrap();
    writeln!(out, "  store i64 {hit_count_new}, i64* {}", globals.hits).unwrap();
    let cached_value =
        emit_call_cache_decode_value(out, &mut next_temp, layout.return_ty, &cached_bits)?;
    writeln!(out, "  ret {} {cached_value}", layout.return_ty.ir()).unwrap();

    writeln!(out, "miss:").unwrap();
    let miss_value = emit_specialized_call_or_fallback(
        out,
        &mut next_temp,
        layout,
        &original,
        &call_args,
        &encoded_args,
        &globals,
        specializations,
    )?;
    let store_enabled = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {store_enabled} = icmp ugt i64 {calls_new}, {warmup}"
    )
    .unwrap();
    writeln!(
        out,
        "  br i1 {store_enabled}, label %admit_check, label %ret_miss"
    )
    .unwrap();

    writeln!(out, "admit_check:").unwrap();
    let pending_valid = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {pending_valid} = load i1, i1* {}",
        globals.pending_valid
    )
    .unwrap();
    if encoded_args.is_empty() {
        writeln!(
            out,
            "  br i1 {pending_valid}, label %admit_hit, label %admit_reset"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "  br i1 {pending_valid}, label %admit_compare, label %admit_reset"
        )
        .unwrap();

        writeln!(out, "admit_compare:").unwrap();
        let mut pending_match = String::new();
        for (arg_index, arg_bits) in encoded_args.iter().enumerate() {
            let arg_ptr = fresh_call_cache_temp(&mut next_temp);
            writeln!(
                out,
                "  {arg_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {}",
                MAX_CALL_CACHE_ARGS, MAX_CALL_CACHE_ARGS, globals.pending_keys, arg_index
            )
            .unwrap();
            let arg_loaded = fresh_call_cache_temp(&mut next_temp);
            writeln!(out, "  {arg_loaded} = load i64, i64* {arg_ptr}").unwrap();
            let arg_eq = fresh_call_cache_temp(&mut next_temp);
            writeln!(out, "  {arg_eq} = icmp eq i64 {arg_loaded}, {arg_bits}").unwrap();
            if pending_match.is_empty() {
                pending_match = arg_eq;
            } else {
                let new_match = fresh_call_cache_temp(&mut next_temp);
                writeln!(out, "  {new_match} = and i1 {pending_match}, {arg_eq}").unwrap();
                pending_match = new_match;
            }
        }
        writeln!(
            out,
            "  br i1 {pending_match}, label %admit_hit, label %admit_reset"
        )
        .unwrap();
    }

    writeln!(out, "admit_reset:").unwrap();
    for (arg_index, arg_bits) in encoded_args.iter().enumerate() {
        let arg_ptr = fresh_call_cache_temp(&mut next_temp);
        writeln!(
            out,
            "  {arg_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {}",
            MAX_CALL_CACHE_ARGS, MAX_CALL_CACHE_ARGS, globals.pending_keys, arg_index
        )
        .unwrap();
        writeln!(out, "  store i64 {arg_bits}, i64* {arg_ptr}").unwrap();
    }
    writeln!(out, "  store i1 true, i1* {}", globals.pending_valid).unwrap();
    writeln!(out, "  store i32 1, i32* {}", globals.pending_count).unwrap();
    writeln!(
        out,
        "  store i64 {calls_new}, i64* {}",
        globals.pending_last_seen
    )
    .unwrap();
    writeln!(out, "  br label %ret_miss").unwrap();

    writeln!(out, "admit_hit:").unwrap();
    writeln!(
        out,
        "  store i64 {calls_new}, i64* {}",
        globals.pending_last_seen
    )
    .unwrap();
    let pending_count_old = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {pending_count_old} = load i32, i32* {}",
        globals.pending_count
    )
    .unwrap();
    let pending_count_new = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {pending_count_new} = add i32 {pending_count_old}, 1"
    )
    .unwrap();
    writeln!(
        out,
        "  store i32 {pending_count_new}, i32* {}",
        globals.pending_count
    )
    .unwrap();
    let pending_count_new_i64 = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {pending_count_new_i64} = zext i32 {pending_count_new} to i64"
    )
    .unwrap();
    emit_update_i64_global_max(
        out,
        &mut next_temp,
        &globals.profile_hottest_key_hits,
        &pending_count_new_i64,
    );
    let store_len = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {store_len} = load i32, i32* {}", globals.len).unwrap();
    let candidate_score = emit_optimized_admission_score(
        out,
        &mut next_temp,
        layout,
        &globals,
        capacity,
        &calls_new,
        &store_len,
        &observed_candidate_hits,
        &observed_candidate_reuse,
    );
    let repeated_enough = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {repeated_enough} = icmp uge i32 {observed_candidate_hits}, {}",
        agam_profile::ADAPTIVE_ADMISSION_MIN_CANDIDATE_HITS as u32
    )
    .unwrap();
    let threshold_met = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {threshold_met} = icmp uge i32 {candidate_score}, {}",
        agam_profile::ADAPTIVE_ADMISSION_OPTIMIZE_THRESHOLD
    )
    .unwrap();
    let admit_candidate = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {admit_candidate} = and i1 {repeated_enough}, {threshold_met}"
    )
    .unwrap();
    writeln!(
        out,
        "  br i1 {admit_candidate}, label %store_check, label %ret_miss"
    )
    .unwrap();

    writeln!(out, "store_check:").unwrap();
    let has_room = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {has_room} = icmp ult i32 {store_len}, {}",
        capacity as u32
    )
    .unwrap();
    writeln!(
        out,
        "  br i1 {has_room}, label %store_new, label %victim_init"
    )
    .unwrap();

    writeln!(out, "store_new:").unwrap();
    let store_index_i64 = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {store_index_i64} = zext i32 {store_len} to i64").unwrap();
    for (arg_index, arg_bits) in encoded_args.iter().enumerate() {
        let arg_ptr = fresh_call_cache_temp(&mut next_temp);
        writeln!(
            out,
            "  {arg_ptr} = getelementptr inbounds [{} x [{} x i64]], [{} x [{} x i64]]* {}, i64 0, i64 {store_index_i64}, i64 {}",
            capacity,
            MAX_CALL_CACHE_ARGS,
            capacity,
            MAX_CALL_CACHE_ARGS,
            globals.keys,
            arg_index
        )
        .unwrap();
        writeln!(out, "  store i64 {arg_bits}, i64* {arg_ptr}").unwrap();
    }
    let result_bits =
        emit_call_cache_encode_value(out, &mut next_temp, layout.return_ty, &miss_value)?;
    let store_value_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {store_value_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {store_index_i64}",
        capacity, capacity, globals.values
    )
    .unwrap();
    writeln!(out, "  store i64 {result_bits}, i64* {store_value_ptr}").unwrap();
    let store_score_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {store_score_ptr} = getelementptr inbounds [{} x i32], [{} x i32]* {}, i64 0, i64 {store_index_i64}",
        capacity, capacity, globals.scores
    )
    .unwrap();
    writeln!(out, "  store i32 {candidate_score}, i32* {store_score_ptr}").unwrap();
    let store_age_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {store_age_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {store_index_i64}",
        capacity, capacity, globals.ages
    )
    .unwrap();
    writeln!(out, "  store i64 {calls_new}, i64* {store_age_ptr}").unwrap();
    emit_store_call_cache_entry_hits(
        out,
        &mut next_temp,
        &globals,
        capacity,
        &store_index_i64,
        &pending_count_new_i64,
    );
    let store_count_old = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {store_count_old} = load i64, i64* {}",
        globals.stores
    )
    .unwrap();
    let store_count_new = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {store_count_new} = add i64 {store_count_old}, 1").unwrap();
    writeln!(
        out,
        "  store i64 {store_count_new}, i64* {}",
        globals.stores
    )
    .unwrap();
    let next_len = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {next_len} = add nuw i32 {store_len}, 1").unwrap();
    writeln!(out, "  store i32 {next_len}, i32* {}", globals.len).unwrap();
    writeln!(out, "  br label %store_done").unwrap();

    writeln!(out, "victim_init:").unwrap();
    writeln!(out, "  store i32 0, i32* {victim_index_slot}").unwrap();
    let zero_score_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {zero_score_ptr} = getelementptr inbounds [{} x i32], [{} x i32]* {}, i64 0, i64 0",
        capacity, capacity, globals.scores
    )
    .unwrap();
    let zero_score = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {zero_score} = load i32, i32* {zero_score_ptr}").unwrap();
    writeln!(out, "  store i32 {zero_score}, i32* {best_score_slot}").unwrap();
    let zero_age_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {zero_age_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 0",
        capacity, capacity, globals.ages
    )
    .unwrap();
    let zero_age = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {zero_age} = load i64, i64* {zero_age_ptr}").unwrap();
    writeln!(out, "  store i64 {zero_age}, i64* {best_age_slot}").unwrap();
    writeln!(out, "  store i32 1, i32* {loop_index_slot}").unwrap();
    writeln!(out, "  br label %victim_scan").unwrap();

    writeln!(out, "victim_scan:").unwrap();
    let loop_index = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {loop_index} = load i32, i32* {loop_index_slot}").unwrap();
    let loop_done = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {loop_done} = icmp eq i32 {loop_index}, {store_len}").unwrap();
    writeln!(
        out,
        "  br i1 {loop_done}, label %victim_decide, label %victim_check"
    )
    .unwrap();

    writeln!(out, "victim_check:").unwrap();
    let loop_index_i64 = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {loop_index_i64} = zext i32 {loop_index} to i64").unwrap();
    let loop_score_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {loop_score_ptr} = getelementptr inbounds [{} x i32], [{} x i32]* {}, i64 0, i64 {loop_index_i64}",
        capacity, capacity, globals.scores
    )
    .unwrap();
    let loop_score = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {loop_score} = load i32, i32* {loop_score_ptr}").unwrap();
    let best_score = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {best_score} = load i32, i32* {best_score_slot}").unwrap();
    let worse_score = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {worse_score} = icmp ult i32 {loop_score}, {best_score}"
    )
    .unwrap();
    let same_score = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {same_score} = icmp eq i32 {loop_score}, {best_score}"
    )
    .unwrap();
    let loop_age_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {loop_age_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {loop_index_i64}",
        capacity, capacity, globals.ages
    )
    .unwrap();
    let loop_age = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {loop_age} = load i64, i64* {loop_age_ptr}").unwrap();
    let best_age = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {best_age} = load i64, i64* {best_age_slot}").unwrap();
    let older_age = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {older_age} = icmp ult i64 {loop_age}, {best_age}").unwrap();
    let same_and_older = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {same_and_older} = and i1 {same_score}, {older_age}").unwrap();
    let replace_victim = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {replace_victim} = or i1 {worse_score}, {same_and_older}"
    )
    .unwrap();
    writeln!(
        out,
        "  br i1 {replace_victim}, label %victim_replace, label %victim_next"
    )
    .unwrap();

    writeln!(out, "victim_replace:").unwrap();
    writeln!(out, "  store i32 {loop_index}, i32* {victim_index_slot}").unwrap();
    writeln!(out, "  store i32 {loop_score}, i32* {best_score_slot}").unwrap();
    writeln!(out, "  store i64 {loop_age}, i64* {best_age_slot}").unwrap();
    writeln!(out, "  br label %victim_next").unwrap();

    writeln!(out, "victim_next:").unwrap();
    let loop_index_next = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {loop_index_next} = add nuw i32 {loop_index}, 1").unwrap();
    writeln!(out, "  store i32 {loop_index_next}, i32* {loop_index_slot}").unwrap();
    writeln!(out, "  br label %victim_scan").unwrap();

    writeln!(out, "victim_decide:").unwrap();
    let best_score_final = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {best_score_final} = load i32, i32* {best_score_slot}"
    )
    .unwrap();
    let better_score = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {better_score} = icmp ugt i32 {candidate_score}, {best_score_final}"
    )
    .unwrap();
    let equal_score = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {equal_score} = icmp eq i32 {candidate_score}, {best_score_final}"
    )
    .unwrap();
    let best_age_final = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {best_age_final} = load i64, i64* {best_age_slot}").unwrap();
    let newer_age = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {newer_age} = icmp ugt i64 {calls_new}, {best_age_final}"
    )
    .unwrap();
    let equal_and_newer = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {equal_and_newer} = and i1 {equal_score}, {newer_age}"
    )
    .unwrap();
    let admit_replace = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {admit_replace} = or i1 {better_score}, {equal_and_newer}"
    )
    .unwrap();
    writeln!(
        out,
        "  br i1 {admit_replace}, label %store_replace, label %ret_miss"
    )
    .unwrap();

    writeln!(out, "store_replace:").unwrap();
    let victim_index = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {victim_index} = load i32, i32* {victim_index_slot}").unwrap();
    let victim_index_i64 = fresh_call_cache_temp(&mut next_temp);
    writeln!(out, "  {victim_index_i64} = zext i32 {victim_index} to i64").unwrap();
    for (arg_index, arg_bits) in encoded_args.iter().enumerate() {
        let arg_ptr = fresh_call_cache_temp(&mut next_temp);
        writeln!(
            out,
            "  {arg_ptr} = getelementptr inbounds [{} x [{} x i64]], [{} x [{} x i64]]* {}, i64 0, i64 {victim_index_i64}, i64 {}",
            capacity,
            MAX_CALL_CACHE_ARGS,
            capacity,
            MAX_CALL_CACHE_ARGS,
            globals.keys,
            arg_index
        )
        .unwrap();
        writeln!(out, "  store i64 {arg_bits}, i64* {arg_ptr}").unwrap();
    }
    let replace_bits =
        emit_call_cache_encode_value(out, &mut next_temp, layout.return_ty, &miss_value)?;
    let replace_value_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {replace_value_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {victim_index_i64}",
        capacity, capacity, globals.values
    )
    .unwrap();
    writeln!(out, "  store i64 {replace_bits}, i64* {replace_value_ptr}").unwrap();
    let replace_score_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {replace_score_ptr} = getelementptr inbounds [{} x i32], [{} x i32]* {}, i64 0, i64 {victim_index_i64}",
        capacity, capacity, globals.scores
    )
    .unwrap();
    writeln!(
        out,
        "  store i32 {candidate_score}, i32* {replace_score_ptr}"
    )
    .unwrap();
    let replace_age_ptr = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {replace_age_ptr} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {victim_index_i64}",
        capacity, capacity, globals.ages
    )
    .unwrap();
    writeln!(out, "  store i64 {calls_new}, i64* {replace_age_ptr}").unwrap();
    emit_store_call_cache_entry_hits(
        out,
        &mut next_temp,
        &globals,
        capacity,
        &victim_index_i64,
        &pending_count_new_i64,
    );
    let replace_store_old = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {replace_store_old} = load i64, i64* {}",
        globals.stores
    )
    .unwrap();
    let replace_store_new = fresh_call_cache_temp(&mut next_temp);
    writeln!(
        out,
        "  {replace_store_new} = add i64 {replace_store_old}, 1"
    )
    .unwrap();
    writeln!(
        out,
        "  store i64 {replace_store_new}, i64* {}",
        globals.stores
    )
    .unwrap();
    writeln!(out, "  br label %store_done").unwrap();

    writeln!(out, "store_done:").unwrap();
    writeln!(out, "  store i1 false, i1* {}", globals.pending_valid).unwrap();
    writeln!(out, "  store i32 0, i32* {}", globals.pending_count).unwrap();
    writeln!(out, "  store i64 0, i64* {}", globals.pending_last_seen).unwrap();
    writeln!(out, "  br label %ret_miss").unwrap();

    writeln!(out, "ret_miss:").unwrap();
    writeln!(out, "  ret {} {miss_value}", layout.return_ty.ir()).unwrap();
    writeln!(out, "}}\n").unwrap();

    Ok(())
}

fn fresh_call_cache_temp(next_temp: &mut usize) -> String {
    let value = format!("%cc{}", *next_temp);
    *next_temp += 1;
    value
}

fn emit_call_cache_encode_value(
    out: &mut String,
    next_temp: &mut usize,
    ty: LlvmType,
    value: &str,
) -> Result<String, String> {
    match ty {
        LlvmType::Int(int_ty) if int_ty.bits == 64 => Ok(value.to_string()),
        LlvmType::Int(int_ty) => {
            let widened = fresh_call_cache_temp(next_temp);
            let opcode = if int_ty.signed { "sext" } else { "zext" };
            writeln!(out, "  {widened} = {opcode} {} {value} to i64", ty.ir()).unwrap();
            Ok(widened)
        }
        LlvmType::Bool => {
            let widened = fresh_call_cache_temp(next_temp);
            writeln!(out, "  {widened} = zext i1 {value} to i64").unwrap();
            Ok(widened)
        }
        LlvmType::Float(LlvmFloatType::F64) => {
            let bits = fresh_call_cache_temp(next_temp);
            writeln!(out, "  {bits} = bitcast double {value} to i64").unwrap();
            Ok(bits)
        }
        LlvmType::Float(LlvmFloatType::F32) => {
            let bits32 = fresh_call_cache_temp(next_temp);
            writeln!(out, "  {bits32} = bitcast float {value} to i32").unwrap();
            let bits64 = fresh_call_cache_temp(next_temp);
            writeln!(out, "  {bits64} = zext i32 {bits32} to i64").unwrap();
            Ok(bits64)
        }
        _ => Err(format!("unsupported LLVM call-cache type {ty:?}")),
    }
}

fn emit_call_cache_decode_value(
    out: &mut String,
    next_temp: &mut usize,
    ty: LlvmType,
    bits: &str,
) -> Result<String, String> {
    match ty {
        LlvmType::Int(int_ty) if int_ty.bits == 64 => Ok(bits.to_string()),
        LlvmType::Int(_) => {
            let truncated = fresh_call_cache_temp(next_temp);
            writeln!(out, "  {truncated} = trunc i64 {bits} to {}", ty.ir()).unwrap();
            Ok(truncated)
        }
        LlvmType::Bool => {
            let truncated = fresh_call_cache_temp(next_temp);
            writeln!(out, "  {truncated} = trunc i64 {bits} to i1").unwrap();
            Ok(truncated)
        }
        LlvmType::Float(LlvmFloatType::F64) => {
            let decoded = fresh_call_cache_temp(next_temp);
            writeln!(out, "  {decoded} = bitcast i64 {bits} to double").unwrap();
            Ok(decoded)
        }
        LlvmType::Float(LlvmFloatType::F32) => {
            let bits32 = fresh_call_cache_temp(next_temp);
            writeln!(out, "  {bits32} = trunc i64 {bits} to i32").unwrap();
            let decoded = fresh_call_cache_temp(next_temp);
            writeln!(out, "  {decoded} = bitcast i32 {bits32} to float").unwrap();
            Ok(decoded)
        }
        _ => Err(format!("unsupported LLVM call-cache type {ty:?}")),
    }
}

fn llvm_i64_literal(raw_bits: u64) -> String {
    (raw_bits as i64).to_string()
}

fn build_specialization_registry(
    module: &MirModule,
    layouts: &HashMap<String, FunctionLayout>,
    call_cache_plan: &CallCachePlan,
    plans: &[CallCacheSpecializationPlan],
) -> SpecializationRegistry {
    let function_names: HashSet<&str> = module
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    let mut by_function = HashMap::new();

    for plan in plans {
        if !function_names.contains(plan.name.as_str())
            || !call_cache_plan.contains(&plan.name)
            || !call_cache_plan.is_optimized(&plan.name)
        {
            continue;
        }
        let Some(layout) = layouts.get(&plan.name) else {
            continue;
        };

        let mut stable_values: BTreeMap<usize, StableScalarValueProfile> = BTreeMap::new();
        for value in &plan.stable_values {
            let Some(param_ty) = layout.params.get(value.index).copied() else {
                continue;
            };
            if !supports_call_cache_type(param_ty) {
                continue;
            }
            let replace = stable_values
                .get(&value.index)
                .map(|current| value.matches > current.matches)
                .unwrap_or(true);
            if replace {
                stable_values.insert(value.index, value.clone());
            }
        }

        if stable_values.is_empty() {
            continue;
        }

        let stable_values: Vec<_> = stable_values.into_values().collect();
        by_function
            .entry(plan.name.clone())
            .or_insert_with(Vec::new)
            .push(LlvmFunctionSpecialization {
                clone_name: llvm_specialization_clone_name(&plan.name, &stable_values),
                stable_values,
            });
    }

    for specializations in by_function.values_mut() {
        specializations.sort_by(|left, right| {
            right
                .stable_values
                .len()
                .cmp(&left.stable_values.len())
                .then_with(|| {
                    right
                        .stable_values
                        .iter()
                        .map(|value| value.matches)
                        .sum::<u64>()
                        .cmp(
                            &left
                                .stable_values
                                .iter()
                                .map(|value| value.matches)
                                .sum::<u64>(),
                        )
                })
                .then_with(|| left.clone_name.cmp(&right.clone_name))
        });
        specializations.dedup_by(|left, right| left.stable_values == right.stable_values);
    }

    SpecializationRegistry { by_function }
}

fn llvm_specialization_clone_name(
    function: &str,
    stable_values: &[StableScalarValueProfile],
) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    function.hash(&mut hasher);
    for value in stable_values {
        value.index.hash(&mut hasher);
        value.raw_bits.hash(&mut hasher);
    }
    format!(
        "__agam_spec_{}_{}",
        sanitize_name(function),
        hasher.finish()
    )
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
    fn test_emit_call_cache_wrapper_for_pure_function() {
        let llvm = compile_to_llvm_with_options(
            r#"
fn hot(n: i64) -> i64:
    let total: i64 = 0
    let i: i64 = 0
    while i < 16:
        total = total + n + i
        i = i + 1
    return total

fn main() -> i32:
    if hot(argc()) >= 0:
        return 0
    return 1
"#,
            LlvmEmitOptions {
                call_cache: true,
                ..LlvmEmitOptions::default()
            },
        );
        assert!(llvm.contains("@agam_call_cache_hot("));
        assert!(llvm.contains("@agam_call_cache_hot_calls = internal global i64 0"));
        assert!(llvm.contains("@agam_call_cache_hot_hits = internal global i64 0"));
        assert!(llvm.contains("@agam_call_cache_hot_stores = internal global i64 0"));
        assert!(llvm.contains("call noundef i64 @agam_call_cache_hot"));
        assert!(llvm.contains("define void @agam_call_cache_dump_profiles()"));
        assert!(llvm.contains("AGAM_LLVM_CALL_CACHE_PROFILE_OUT"));
    }

    #[test]
    fn test_emit_llvm_call_cache_stable_value_profile_globals_and_export() {
        let llvm = compile_to_llvm_with_options(
            r#"
fn hot(n: i64) -> i64:
    return n + 1

fn main() -> i32:
    if hot(argc()) >= 0:
        return 0
    return 1
"#,
            LlvmEmitOptions {
                call_cache: true,
                ..LlvmEmitOptions::default()
            },
        );
        assert!(llvm.contains(
            "@agam_call_cache_hot_profile_stable_values = internal global [4 x i64] zeroinitializer"
        ));
        assert!(llvm.contains(
            "@agam_call_cache_hot_profile_stable_scores = internal global [4 x i64] zeroinitializer"
        ));
        assert!(llvm.contains(
            "@agam_call_cache_hot_profile_stable_matches = internal global [4 x i64] zeroinitializer"
        ));
        assert!(llvm.contains("@agam_call_cache_hot_profile_reuse_total = internal global i64 0"));
        assert!(
            llvm.contains("@agam_call_cache_hot_profile_reuse_samples = internal global i64 0")
        );
        assert!(llvm.contains("@agam_call_cache_hot_profile_reuse_max = internal global i64 0"));
        assert!(
            llvm.contains(
                "@agam_call_cache_hot_profile_specialization_hits = internal global i64 0"
            )
        );
        assert!(llvm.contains("@agam_call_cache_hot_profile_unique_keys = internal global i32 0"));
        assert!(
            llvm.contains("@agam_call_cache_hot_profile_hottest_key_hits = internal global i64 0")
        );
        assert!(llvm.contains(
            "@agam_call_cache_hot_profile_observed_keys = internal global [64 x [4 x i64]] zeroinitializer"
        ));
        assert!(llvm.contains(
            "@agam_call_cache_hot_profile_observed_hits = internal global [64 x i64] zeroinitializer"
        ));
        assert!(llvm.contains(
            "@agam_call_cache_hot_profile_observed_last_seen = internal global [64 x i64] zeroinitializer"
        ));
        assert!(llvm.contains(
            "@agam_call_cache_hot_profile_observed_reuse = internal global [64 x i64] zeroinitializer"
        ));
        assert!(llvm.contains(
            "@agam_call_cache_hot_profile_specialization_fallbacks = internal global i64 0"
        ));
        assert!(llvm.contains("AGAM_LLVM_CALL_CACHE_PROFILE_V5"));
        assert!(llvm.contains(
            "getelementptr inbounds [4 x i64], [4 x i64]* @agam_call_cache_hot_profile_stable_values"
        ));
        assert!(llvm.contains(
            "getelementptr inbounds [4 x i64], [4 x i64]* @agam_call_cache_hot_profile_stable_matches"
        ));
    }

    #[test]
    fn test_do_not_emit_call_cache_for_impure_function() {
        let llvm = compile_to_llvm_with_options(
            r#"
fn nowish() -> f64:
    return clock()

fn main() -> i32:
    let x: f64 = nowish()
    if x >= 0.0:
        return 0
    return 1
"#,
            LlvmEmitOptions {
                call_cache: true,
                ..LlvmEmitOptions::default()
            },
        );
        assert!(!llvm.contains("@agam_call_cache_nowish("));
        assert!(!llvm.contains("@agam_call_cache_nowish_calls"));
        assert!(llvm.contains("call noundef double @agam_nowish()"));
    }

    #[test]
    fn test_emit_call_cache_wrapper_for_stable_process_arg_reader() {
        let llvm = compile_to_llvm_with_options(
            r#"
fn arg_count() -> i32:
    return argc()

fn main() -> i32:
    if arg_count() > 0:
        return 0
    return 1
"#,
            LlvmEmitOptions {
                call_cache: true,
                ..LlvmEmitOptions::default()
            },
        );
        assert!(llvm.contains("@agam_call_cache_arg_count("));
        assert!(llvm.contains("call noundef i32 @agam_call_cache_arg_count"));
    }

    #[test]
    fn test_emit_call_cache_wrapper_for_selected_pure_caller_without_selected_callee() {
        let llvm = compile_to_llvm_with_options(
            r#"
fn inner(n: i64) -> i64:
    return n + 1

fn outer(n: i64) -> i64:
    return inner(n) + 1

fn main() -> i32:
    if outer(argc()) >= 0:
        return 0
    return 1
"#,
            LlvmEmitOptions {
                call_cache_only: vec!["outer".into()],
                ..LlvmEmitOptions::default()
            },
        );
        assert!(llvm.contains("@agam_call_cache_outer("));
        assert!(!llvm.contains("@agam_call_cache_inner("));
    }

    #[test]
    fn test_emit_optimized_call_cache_wrapper_for_pure_function() {
        let llvm = compile_to_llvm_with_options(
            r#"
fn hot(n: i64) -> i64:
    return n + 1

fn main() -> i32:
    if hot(argc()) >= 0:
        return 0
    return 1
"#,
            LlvmEmitOptions {
                call_cache_optimize: true,
                ..LlvmEmitOptions::default()
            },
        );
        assert!(llvm.contains("@agam_call_cache_hot_scores = internal global"));
        assert!(llvm.contains("@agam_call_cache_hot_pending_count = internal global i32 0"));
        assert!(llvm.contains("@agam_call_cache_hot_pending_last_seen = internal global i64 0"));
        assert!(llvm.contains("label %admit_check"));
        assert!(llvm.contains("label %victim_init"));
    }

    #[test]
    fn test_emit_optimized_call_cache_wrapper_uses_adaptive_admission_signals() {
        let llvm = compile_to_llvm_with_options(
            r#"
fn hot(n: i64) -> i64:
    return n + 1

fn main() -> i32:
    if hot(argc()) >= 0:
        return 0
    return 1
"#,
            LlvmEmitOptions {
                call_cache_optimize: true,
                ..LlvmEmitOptions::default()
            },
        );
        assert!(llvm.contains(
            "getelementptr inbounds [64 x i64], [64 x i64]* @agam_call_cache_hot_profile_observed_last_seen"
        ));
        assert!(llvm.contains("phi i32 [ 0, %stable_done_"));
        assert!(llvm.contains("observe_victim_init"));
        assert!(llvm.contains("load i64, i64* @agam_call_cache_hot_hits"));
        assert!(llvm.contains(
            "getelementptr inbounds [4 x i64], [4 x i64]* @agam_call_cache_hot_profile_stable_scores"
        ));
        assert!(llvm.contains(
            "getelementptr inbounds [4 x i64], [4 x i64]* @agam_call_cache_hot_profile_stable_matches"
        ));
        assert!(llvm.contains("load i32, i32* @agam_call_cache_hot_profile_unique_keys"));
        assert!(llvm.contains("load i64, i64* @agam_call_cache_hot_profile_hottest_key_hits"));
        assert!(llvm.contains(
            "getelementptr inbounds [64 x [4 x i64]], [64 x [4 x i64]]* @agam_call_cache_hot_profile_observed_keys"
        ));
        assert!(llvm.contains(
            "getelementptr inbounds [64 x i64], [64 x i64]* @agam_call_cache_hot_profile_observed_hits"
        ));
        assert!(llvm.contains(
            "getelementptr inbounds [64 x i64], [64 x i64]* @agam_call_cache_hot_profile_observed_reuse"
        ));
        assert!(llvm.contains("load i64, i64* @agam_call_cache_hot_profile_specialization_hits"));
        assert!(
            llvm.contains("load i64, i64* @agam_call_cache_hot_profile_specialization_fallbacks")
        );
        assert!(
            llvm.matches("load i64, i64* @agam_call_cache_hot_profile_reuse_total")
                .count()
                >= 2
        );
        assert!(llvm.contains("icmp uge i64"));
        assert!(llvm.contains("select i1"));
    }

    #[test]
    fn test_emit_guarded_specialization_clone_for_optimized_llvm_call_cache() {
        let llvm = compile_to_llvm_with_options(
            r#"
fn hot(n: i64) -> i64:
    return n + 1

fn main() -> i32:
    if hot(argc()) >= 0:
        return 0
    return 1
"#,
            LlvmEmitOptions {
                call_cache_optimize_only: vec!["hot".into()],
                call_cache_specializations: vec![CallCacheSpecializationPlan {
                    name: "hot".into(),
                    stable_values: vec![StableScalarValueProfile {
                        index: 0,
                        raw_bits: 7,
                        matches: 16,
                    }],
                }],
                ..LlvmEmitOptions::default()
            },
        );
        assert!(llvm.contains("define noundef i64 @__agam_spec_hot_"));
        assert!(llvm.contains("store i64 7, i64* %local_n"));
        assert!(llvm.contains("call noundef i64 @__agam_spec_hot_"));
        assert!(llvm.contains("label %spec_call_"));
        assert!(llvm.contains("phi i64"));
        assert!(llvm.contains("load i64, i64* @agam_call_cache_hot_profile_specialization_hits"));
        assert!(
            llvm.contains("load i64, i64* @agam_call_cache_hot_profile_specialization_fallbacks")
        );
    }

    #[test]
    fn test_emit_multiple_guarded_specialization_clones_for_same_function() {
        let specific_values = vec![
            StableScalarValueProfile {
                index: 0,
                raw_bits: 7,
                matches: 24,
            },
            StableScalarValueProfile {
                index: 1,
                raw_bits: 9,
                matches: 20,
            },
        ];
        let broad_values = vec![StableScalarValueProfile {
            index: 0,
            raw_bits: 7,
            matches: 16,
        }];
        let specific_clone = llvm_specialization_clone_name("hot", &specific_values);
        let broad_clone = llvm_specialization_clone_name("hot", &broad_values);
        let llvm = compile_to_llvm_with_options(
            r#"
fn hot(a: i64, b: i64) -> i64:
    return a + b

fn main() -> i32:
    if hot(7, 9) > 0:
        return 0
    return 1
"#,
            LlvmEmitOptions {
                call_cache_optimize_only: vec!["hot".into()],
                call_cache_specializations: vec![
                    CallCacheSpecializationPlan {
                        name: "hot".into(),
                        stable_values: broad_values.clone(),
                    },
                    CallCacheSpecializationPlan {
                        name: "hot".into(),
                        stable_values: specific_values.clone(),
                    },
                ],
                ..LlvmEmitOptions::default()
            },
        );
        assert_eq!(
            llvm.match_indices("define noundef i64 @__agam_spec_hot_")
                .count(),
            2
        );
        assert!(llvm.contains(&format!("call noundef i64 @{specific_clone}(")));
        assert!(llvm.contains(&format!("call noundef i64 @{broad_clone}(")));
        let specific_pos = llvm
            .find(&format!("call noundef i64 @{specific_clone}("))
            .expect("missing specific clone call");
        let broad_pos = llvm
            .find(&format!("call noundef i64 @{broad_clone}("))
            .expect("missing broad clone call");
        assert!(
            specific_pos < broad_pos,
            "expected the more specific clone to be checked before the broader clone"
        );
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
                ..LlvmEmitOptions::default()
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
