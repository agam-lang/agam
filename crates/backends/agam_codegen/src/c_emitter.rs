//! C code emitter — translates MIR into C source code.
//!
//! The generated C code can be compiled with any standard C compiler
//! to produce native binaries. This is the simplest path to running
//! Agam programs natively without requiring LLVM bindings.

use agam_mir::ir::*;
use agam_sema::types::{FloatSize, Type, builtin_type_by_id};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CType {
    Int,
    Float,
    Bool,
    Str,
    OpaquePtr,
    Tensor,
    DataFrame,
}

impl CType {
    fn name(self) -> &'static str {
        match self {
            CType::Int => "agam_int",
            CType::Float => "agam_float",
            CType::Bool => "agam_bool",
            CType::Str => "agam_str",
            CType::OpaquePtr => "void*",
            CType::Tensor => "AgamTensor*",
            CType::DataFrame => "AgamDataFrame*",
        }
    }

    fn default_value(self) -> &'static str {
        match self {
            CType::Int => "0",
            CType::Float => "0.0",
            CType::Bool => "0",
            CType::Str => "NULL",
            CType::OpaquePtr | CType::Tensor | CType::DataFrame => "NULL",
        }
    }
}

#[derive(Clone, Copy)]
struct BuiltinSig {
    return_ty: CType,
}

struct FunctionLayout {
    params: Vec<CType>,
    return_ty: CType,
    value_types: HashMap<ValueId, CType>,
    local_types: HashMap<String, CType>,
}

fn analyze_module(module: &MirModule) -> HashMap<String, FunctionLayout> {
    let mut return_types: HashMap<String, CType> = module
        .functions
        .iter()
        .map(|func| {
            (
                func.name.clone(),
                infer_ctype_from_type_id(func.return_ty).unwrap_or(CType::Int),
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

fn analyze_function(func: &MirFunction, return_types: &HashMap<String, CType>) -> FunctionLayout {
    let mut layout = FunctionLayout {
        params: func
            .params
            .iter()
            .map(|param| infer_ctype_from_type_id(param.ty).unwrap_or(CType::Int))
            .collect(),
        return_ty: infer_ctype_from_type_id(func.return_ty).unwrap_or(CType::Int),
        value_types: HashMap::new(),
        local_types: HashMap::new(),
    };

    for (index, param) in func.params.iter().enumerate() {
        let ty = layout.params.get(index).copied().unwrap_or(CType::Int);
        layout.value_types.insert(param.value, ty);
        layout.local_types.insert(param.name.clone(), ty);
    }

    for block in &func.blocks {
        for instr in &block.instructions {
            let inferred = match &instr.op {
                Op::ConstInt(_) => CType::Int,
                Op::ConstFloat(_) => CType::Float,
                Op::ConstBool(_) => CType::Bool,
                Op::ConstString(_) => CType::Str,
                Op::Unit => CType::Int,
                Op::Copy(value) => value_type(&layout, *value),
                Op::BinOp { op, left, right } => {
                    infer_binop_type(*op, value_type(&layout, *left), value_type(&layout, *right))
                }
                Op::UnOp { op, operand } => infer_unop_type(*op, value_type(&layout, *operand)),
                Op::Call { callee, .. } => {
                    if is_print_builtin(callee) {
                        CType::Int
                    } else if let Some(sig) = builtin_signature(callee) {
                        sig.return_ty
                    } else {
                        return_types.get(callee).copied().unwrap_or(CType::Int)
                    }
                }
                Op::LoadLocal(name) => layout
                    .local_types
                    .get(name)
                    .copied()
                    .or_else(|| infer_ctype_from_type_id(instr.ty))
                    .unwrap_or(CType::Int),
                Op::StoreLocal { name, value } => {
                    let ty = layout
                        .local_types
                        .get(name)
                        .copied()
                        .or_else(|| infer_ctype_from_type_id(instr.ty))
                        .unwrap_or_else(|| value_type(&layout, *value));
                    layout.local_types.insert(name.clone(), ty);
                    ty
                }
                Op::StoreIndex { value, .. } => value_type(&layout, *value),
                Op::Alloca { name, ty } => {
                    let ty = infer_ctype_from_type_id(*ty)
                        .or_else(|| layout.local_types.get(name).copied())
                        .unwrap_or(CType::Int);
                    layout.local_types.entry(name.clone()).or_insert(ty);
                    ty
                }
                Op::GetField { object, .. } => value_type(&layout, *object),
                Op::GetIndex { object, .. } => value_type(&layout, *object),
                Op::Phi(entries) => entries
                    .iter()
                    .map(|(_, value)| value_type(&layout, *value))
                    .reduce(merge_type)
                    .unwrap_or(CType::Int),
                Op::Cast { target_ty, value } => infer_ctype_from_type_id(*target_ty)
                    .unwrap_or_else(|| value_type(&layout, *value)),
                Op::EffectPerform {
                    effect, operation, ..
                } => infer_effect_return_type(effect, operation),
                Op::HandleWith { .. } => CType::Int,
                Op::GpuKernelLaunch { .. } => CType::Int,
                Op::GpuSharedAlloc { .. } => CType::Int,
                Op::GpuIntrinsic { .. } => CType::Int,
                Op::InlineAsm { .. } => CType::Int,
                Op::EnumConstruct { .. } | Op::EnumPayload { .. } => {
                    infer_ctype_from_type_id(instr.ty).unwrap_or(CType::Int)
                }
                Op::EnumTag(_) => CType::Int,
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

    layout
}

fn infer_ctype_from_type_id(type_id: agam_sema::symbol::TypeId) -> Option<CType> {
    match builtin_type_by_id(type_id)? {
        Type::Bool => Some(CType::Bool),
        Type::Str => Some(CType::Str),
        Type::Float(FloatSize::F32 | FloatSize::F64) => Some(CType::Float),
        Type::Int(_) | Type::UInt(_) => Some(CType::Int),
        Type::Ref { .. } | Type::Ptr { .. } | Type::Slice(_) | Type::Array { .. } => {
            Some(CType::OpaquePtr)
        }
        _ => None,
    }
}

fn builtin_signature(name: &str) -> Option<BuiltinSig> {
    match name {
        "argc" => Some(BuiltinSig {
            return_ty: CType::Int,
        }),
        "argv" => Some(BuiltinSig {
            return_ty: CType::Str,
        }),
        "parse_int" => Some(BuiltinSig {
            return_ty: CType::Int,
        }),
        "clock" => Some(BuiltinSig {
            return_ty: CType::Float,
        }),
        "gpu_malloc" => Some(BuiltinSig {
            return_ty: CType::OpaquePtr,
        }),
        "gpu_free" | "gpu_memcpy_to_device" | "gpu_memcpy_to_host" => Some(BuiltinSig {
            return_ty: CType::Int,
        }),
        "adam" => Some(BuiltinSig {
            return_ty: CType::Float,
        }),
        "dataframe_build_sin" => Some(BuiltinSig {
            return_ty: CType::DataFrame,
        }),
        "dataframe_filter_gt" => Some(BuiltinSig {
            return_ty: CType::DataFrame,
        }),
        "dataframe_sort" => Some(BuiltinSig {
            return_ty: CType::DataFrame,
        }),
        "dataframe_group_by" => Some(BuiltinSig {
            return_ty: CType::DataFrame,
        }),
        "dataframe_mean" => Some(BuiltinSig {
            return_ty: CType::Float,
        }),
        "dataframe_free" => Some(BuiltinSig {
            return_ty: CType::Int,
        }),
        "tensor_fill_rand" => Some(BuiltinSig {
            return_ty: CType::Tensor,
        }),
        "dense_layer" => Some(BuiltinSig {
            return_ty: CType::Tensor,
        }),
        "conv2d" => Some(BuiltinSig {
            return_ty: CType::Tensor,
        }),
        "tensor_checksum" => Some(BuiltinSig {
            return_ty: CType::Float,
        }),
        "tensor_free" => Some(BuiltinSig {
            return_ty: CType::Int,
        }),
        _ => None,
    }
}

fn is_print_builtin(name: &str) -> bool {
    matches!(name, "print" | "println" | "print_int" | "print_str")
}

fn has_runtime_prelude_definition(name: &str) -> bool {
    is_print_builtin(name) || builtin_signature(name).is_some() || is_effect_prelude_function(name)
}

/// Return true if `name` is an effect dispatch function emitted in the prelude.
fn is_effect_prelude_function(name: &str) -> bool {
    name.starts_with("agam_effect_")
}

/// Infer the C return type for a specific effect operation.
fn infer_effect_return_type(effect: &str, operation: &str) -> CType {
    match (effect, operation) {
        // FileSystem returns
        ("FileSystem", "exists" | "is_file" | "is_dir") => CType::Bool,
        ("FileSystem", "read_to_string") => CType::Str,
        ("FileSystem", "read_lines") => CType::Str, // simplified: returns joined string
        ("FileSystem", "list_dir") => CType::Str,   // simplified: returns joined string
        ("FileSystem", "create_dir_all" | "write_string" | "append_string") => CType::Int,
        // Console returns
        ("Console", "read_line") => CType::Str,
        ("Console", "print" | "println" | "eprint" | "eprintln") => CType::Int,
        _ => CType::Int,
    }
}

/// Map an effect+operation pair to its emitted C function name.
fn effect_c_function_name(effect: &str, operation: &str) -> String {
    format!("agam_effect_{}_{}", effect, operation)
}

fn infer_binop_type(op: MirBinOp, left: CType, right: CType) -> CType {
    match op {
        MirBinOp::Eq
        | MirBinOp::NotEq
        | MirBinOp::Lt
        | MirBinOp::LtEq
        | MirBinOp::Gt
        | MirBinOp::GtEq
        | MirBinOp::And
        | MirBinOp::Or => CType::Bool,
        MirBinOp::Add if left == CType::Str || right == CType::Str => CType::Str,
        _ if left == CType::OpaquePtr || right == CType::OpaquePtr => CType::OpaquePtr,
        _ if left == CType::DataFrame || right == CType::DataFrame => CType::DataFrame,
        _ if left == CType::Tensor || right == CType::Tensor => CType::Tensor,
        _ if left == CType::Float || right == CType::Float => CType::Float,
        _ => CType::Int,
    }
}

fn infer_unop_type(op: MirUnOp, operand: CType) -> CType {
    match op {
        MirUnOp::Not => CType::Bool,
        MirUnOp::Neg if operand == CType::Float => CType::Float,
        _ => CType::Int,
    }
}

fn merge_type(left: CType, right: CType) -> CType {
    if left == right {
        left
    } else if left == CType::OpaquePtr || right == CType::OpaquePtr {
        CType::OpaquePtr
    } else if left == CType::DataFrame || right == CType::DataFrame {
        CType::DataFrame
    } else if left == CType::Tensor || right == CType::Tensor {
        CType::Tensor
    } else if left == CType::Str || right == CType::Str {
        CType::Str
    } else if left == CType::Float || right == CType::Float {
        CType::Float
    } else if left == CType::Bool || right == CType::Bool {
        CType::Bool
    } else {
        CType::Int
    }
}

fn value_type(layout: &FunctionLayout, value: ValueId) -> CType {
    layout
        .value_types
        .get(&value)
        .copied()
        .unwrap_or(CType::Int)
}

fn emit_common_prelude(out: &mut String) {
    out.push_str(include_str!("../runtime/common_prelude.c"));
}

fn emit_dataframe_prelude(out: &mut String) {
    out.push_str(include_str!("../runtime/dataframe_prelude.c"));
}

fn emit_tensor_prelude(out: &mut String) {
    out.push_str(include_str!("../runtime/tensor_prelude.c"));
}

fn emit_gpu_host_prelude(out: &mut String) {
    out.push_str(include_str!("../runtime/gpu_host_prelude.c"));
}

fn emit_effect_prelude(out: &mut String) {
    out.push_str(include_str!("../runtime/effect_prelude.c"));
}

fn emit_runtime_prelude(out: &mut String, module: &MirModule) {
    use agam_sema::target::TargetProfile;

    // Determine if any function targets IoT (most restrictive)
    let has_iot = module
        .functions
        .iter()
        .any(|f| f.target == TargetProfile::Iot);
    let has_hpc = module
        .functions
        .iter()
        .any(|f| f.target == TargetProfile::Hpc);

    emit_common_prelude(out);

    if has_hpc {
        out.push_str("/* ── HPC Target: aggressive vectorization preferred ── */\n");
        out.push_str("#ifndef AGAM_HPC\n#define AGAM_HPC 1\n#endif\n\n");
    }

    if has_iot {
        out.push_str("/* ── IoT Target: no heap, no stdlib, no effects ── */\n");
        out.push_str("#ifndef AGAM_NO_HEAP\n#define AGAM_NO_HEAP 1\n#endif\n\n");
    } else {
        // Only emit heavy preludes when not targeting IoT
        emit_gpu_host_prelude(out);
        emit_dataframe_prelude(out);
        emit_tensor_prelude(out);
        emit_effect_prelude(out);
    }
    out.push('\n');
}

/// Emit a complete MIR module as C source code.
pub fn emit_c(module: &MirModule) -> String {
    let layouts = analyze_module(module);
    let mut output = String::new();

    // Header
    writeln!(output, "/* Generated by agamc — Agam Compiler */").unwrap();
    writeln!(output, "#include <stdio.h>").unwrap();
    writeln!(output, "#include <stdlib.h>").unwrap();
    writeln!(output, "#include <string.h>").unwrap();
    writeln!(output, "#include <stdint.h>").unwrap();
    writeln!(output, "#include <math.h>").unwrap();
    writeln!(output, "#include <time.h>").unwrap();
    writeln!(output).unwrap();

    // Type aliases
    writeln!(output, "typedef int64_t agam_int;").unwrap();
    writeln!(output, "typedef double agam_float;").unwrap();
    writeln!(output, "typedef int agam_bool;").unwrap();
    writeln!(output, "typedef const char* agam_str;").unwrap();
    writeln!(output).unwrap();

    emit_runtime_prelude(&mut output, module);

    // Collect all unknown function calls and generate stub declarations
    let mut unknown_funcs: HashSet<String> = HashSet::new();
    for func in &module.functions {
        let layout = layouts.get(&func.name).expect("missing function layout");
        for block in &func.blocks {
            for instr in &block.instructions {
                if let Op::Call { callee, args } = &instr.op {
                    let mangled = mangle_name(callee);
                    if !has_runtime_prelude_definition(callee)
                        && !module
                            .functions
                            .iter()
                            .any(|f| mangle_name(&f.name) == mangled)
                    {
                        let ret_ty = value_type(layout, instr.result).name();
                        unknown_funcs.insert(format!(
                            "{} {}({});",
                            ret_ty,
                            mangled,
                            args.iter()
                                .enumerate()
                                .map(|(i, arg)| format!(
                                    "{} __a{}",
                                    value_type(layout, *arg).name(),
                                    i
                                ))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                }
            }
        }
    }
    if !unknown_funcs.is_empty() {
        writeln!(output, "/* ── External function stubs ── */").unwrap();
        for stub in &unknown_funcs {
            writeln!(output, "{}", stub).unwrap();
        }
        writeln!(output).unwrap();
    }

    // Forward declarations
    for func in &module.functions {
        let layout = layouts.get(&func.name).expect("missing function layout");
        if func.name == "main" {
            writeln!(output, "int main(int argc, char** argv);").unwrap();
        } else {
            write!(
                output,
                "{} {}(",
                layout.return_ty.name(),
                mangle_name(&func.name)
            )
            .unwrap();
            for (i, _) in func.params.iter().enumerate() {
                if i > 0 {
                    write!(output, ", ").unwrap();
                }
                let param_ty = layout.params.get(i).copied().unwrap_or(CType::Int);
                write!(output, "{} __p{}", param_ty.name(), i).unwrap();
            }
            writeln!(output, ");").unwrap();
        }
    }
    writeln!(output).unwrap();

    // Function definitions
    for func in &module.functions {
        let layout = layouts.get(&func.name).expect("missing function layout");
        emit_function(&mut output, func, layout);
        writeln!(output).unwrap();
    }

    output
}

fn emit_function(out: &mut String, func: &MirFunction, layout: &FunctionLayout) {
    // Function signature
    if func.name == "main" {
        writeln!(out, "int main(int argc, char** argv) {{").unwrap();
    } else {
        write!(
            out,
            "{} {}(",
            layout.return_ty.name(),
            mangle_name(&func.name)
        )
        .unwrap();
        for (i, _param) in func.params.iter().enumerate() {
            if i > 0 {
                write!(out, ", ").unwrap();
            }
            let param_ty = layout.params.get(i).copied().unwrap_or(CType::Int);
            write!(out, "{} __p{}", param_ty.name(), i).unwrap();
        }
        writeln!(out, ") {{").unwrap();
    }

    // Emit parameter → local aliases
    if func.name == "main" {
        writeln!(out, "  agam_runtime_argc = (agam_int)argc;").unwrap();
        writeln!(out, "  agam_runtime_argv = argv;").unwrap();
    }
    for (i, param) in func.params.iter().enumerate() {
        let param_ty = layout.params.get(i).copied().unwrap_or(CType::Int);
        writeln!(
            out,
            "  {} {} = __p{};",
            param_ty.name(),
            mangle_local(&param.name),
            i
        )
        .unwrap();
    }

    // Emit all basic blocks
    for block in &func.blocks {
        emit_block(out, block, layout, func.name == "main");
    }

    writeln!(out, "}}").unwrap();
}

fn emit_block(out: &mut String, block: &BasicBlock, layout: &FunctionLayout, is_main: bool) {
    writeln!(out, "block_{}:", block.id.0).unwrap();

    for instr in &block.instructions {
        emit_instruction(out, instr, layout);
    }

    emit_terminator(out, &block.terminator, layout, is_main);
}

fn emit_instruction(out: &mut String, instr: &Instruction, layout: &FunctionLayout) {
    let v = format!("__v{}", instr.result.0);
    let result_ty = value_type(layout, instr.result);

    match &instr.op {
        Op::ConstInt(val) => {
            writeln!(out, "  {} {} = {};", result_ty.name(), v, val).unwrap();
        }
        Op::ConstFloat(val) => {
            writeln!(out, "  {} {} = {};", result_ty.name(), v, val).unwrap();
        }
        Op::ConstBool(val) => {
            writeln!(
                out,
                "  {} {} = {};",
                result_ty.name(),
                v,
                if *val { 1 } else { 0 }
            )
            .unwrap();
        }
        Op::ConstString(val) => {
            writeln!(
                out,
                "  {} {} = \"{}\";",
                result_ty.name(),
                v,
                escape_c_string(val)
            )
            .unwrap();
        }
        Op::Unit => {
            writeln!(
                out,
                "  {} {} = {}; /* unit */",
                result_ty.name(),
                v,
                result_ty.default_value()
            )
            .unwrap();
        }
        Op::BinOp { op, left, right } => {
            if *op == MirBinOp::Add && result_ty == CType::Str {
                writeln!(
                    out,
                    "  {} {} = agam_str_concat((agam_str)__v{}, (agam_str)__v{});",
                    result_ty.name(),
                    v,
                    left.0,
                    right.0
                )
                .unwrap();
            } else {
                let op_str = binop_to_c(*op);
                writeln!(
                    out,
                    "  {} {} = __v{} {} __v{};",
                    result_ty.name(),
                    v,
                    left.0,
                    op_str,
                    right.0
                )
                .unwrap();
            }
        }
        Op::UnOp { op, operand } => {
            let op_str = unop_to_c(*op);
            writeln!(
                out,
                "  {} {} = {}__v{};",
                result_ty.name(),
                v,
                op_str,
                operand.0
            )
            .unwrap();
        }
        Op::Call { callee, args } => {
            if is_print_builtin(callee) {
                if args.is_empty() {
                    writeln!(out, "  printf(\"\\n\");").unwrap();
                } else {
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            writeln!(out, "  printf(\" \");").unwrap();
                        }
                        emit_print_value(out, *arg, value_type(layout, *arg));
                    }
                    if callee == "println" || callee == "print_str" {
                        writeln!(out, "  printf(\"\\n\");").unwrap();
                    } else if callee == "print" || callee == "print_int" {
                        writeln!(out, "  printf(\"\\n\");").unwrap(); // always append newline for benchmarking clarify
                    }
                }
                writeln!(out, "  {} {} = 0;", result_ty.name(), v).unwrap();
            } else {
                let arg_strs: Vec<String> = args.iter().map(|a| format!("__v{}", a.0)).collect();
                writeln!(
                    out,
                    "  {} {} = {}({});",
                    result_ty.name(),
                    v,
                    mangle_name(callee),
                    arg_strs.join(", ")
                )
                .unwrap();
            }
        }
        Op::Copy(value) => {
            writeln!(out, "  {} {} = __v{};", result_ty.name(), v, value.0).unwrap();
        }
        Op::LoadLocal(name) => {
            writeln!(
                out,
                "  {} {} = {};",
                result_ty.name(),
                v,
                mangle_local(name)
            )
            .unwrap();
        }
        Op::StoreLocal { name, value } => {
            writeln!(out, "  {} = __v{};", mangle_local(name), value.0).unwrap();
            writeln!(out, "  {} {} = __v{};", result_ty.name(), v, value.0).unwrap();
        }
        Op::StoreIndex {
            object,
            index,
            value,
        } => {
            writeln!(
                out,
                "  /* __v{}[__v{}] = __v{} */",
                object.0, index.0, value.0
            )
            .unwrap();
            writeln!(out, "  {} {} = __v{};", result_ty.name(), v, value.0).unwrap();
        }
        Op::Alloca { name, .. } => {
            let local_ty = layout.local_types.get(name).copied().unwrap_or(CType::Int);
            writeln!(
                out,
                "  {} {} = {};",
                local_ty.name(),
                mangle_local(name),
                local_ty.default_value()
            )
            .unwrap();
            writeln!(
                out,
                "  {} {} = {};",
                result_ty.name(),
                v,
                result_ty.default_value()
            )
            .unwrap();
        }
        Op::GetField { object, field } => {
            writeln!(
                out,
                "  {} {} = __v{}; /* .{} */",
                result_ty.name(),
                v,
                object.0,
                field
            )
            .unwrap();
        }
        Op::GetIndex { object, index } => {
            writeln!(
                out,
                "  {} {} = __v{}; /* [__v{}] */",
                result_ty.name(),
                v,
                object.0,
                index.0
            )
            .unwrap();
        }
        Op::Phi(entries) => {
            writeln!(
                out,
                "  {} {} = {}; /* phi */",
                result_ty.name(),
                v,
                result_ty.default_value()
            )
            .unwrap();
            for (block, val) in entries {
                writeln!(out, "  /* phi: block_{} -> __v{} */", block.0, val.0).unwrap();
            }
        }
        Op::Cast { value, .. } => {
            writeln!(
                out,
                "  {} {} = ({})__v{};",
                result_ty.name(),
                v,
                result_ty.name(),
                value.0
            )
            .unwrap();
        }
        Op::EffectPerform {
            effect,
            operation,
            args,
        } => {
            let func_name = effect_c_function_name(effect, operation);
            let arg_strs: Vec<String> = args.iter().map(|a| format!("__v{}", a.0)).collect();
            writeln!(
                out,
                "  {} {} = {}({});",
                result_ty.name(),
                v,
                func_name,
                arg_strs.join(", ")
            )
            .unwrap();
        }
        Op::HandleWith {
            effect,
            handler,
            body,
        } => {
            writeln!(
                out,
                "  /* handle {} with {} — executing body block_{} */",
                effect, handler, body.0
            )
            .unwrap();
            writeln!(out, "  {} {} = 0;", result_ty.name(), v).unwrap();
        }
        Op::GpuKernelLaunch {
            kernel_name,
            grid,
            block,
            shared_memory_bytes,
            args,
        } => {
            let arg_strs: Vec<String> = args.iter().map(|a| format!("__v{}", a.0)).collect();
            writeln!(
                out,
                "  /* GPU kernel launch: {}<<<__v{}, __v{}>>>({}, shared={}) */",
                kernel_name,
                grid.0,
                block.0,
                arg_strs.join(", "),
                shared_memory_bytes
            )
            .unwrap();
            writeln!(
                out,
                "  {} {} = 0; /* kernel launch result */",
                result_ty.name(),
                v
            )
            .unwrap();
        }
        Op::GpuSharedAlloc { element_abi, count } => {
            writeln!(
                out,
                "  {} {} = 0; /* GPU shared alloc {:?}[__v{}] lowers only through the NVPTX emitter */",
                result_ty.name(),
                v,
                element_abi,
                count.0
            )
            .unwrap();
        }
        Op::GpuIntrinsic { kind, args } => {
            let arg_strs: Vec<String> = args.iter().map(|a| format!("__v{}", a.0)).collect();
            writeln!(
                out,
                "  {} {} = 0; /* GPU intrinsic {:?}({}) */",
                result_ty.name(),
                v,
                kind,
                arg_strs.join(", ")
            )
            .unwrap();
        }
        Op::InlineAsm {
            asm_string,
            constraints,
            args,
        } => {
            let arg_strs: Vec<String> = args.iter().map(|a| format!("__v{}", a.0)).collect();
            writeln!(
                out,
                "  /* inline asm: \"{}\" constraints=\"{}\" args=({}) */",
                asm_string,
                constraints,
                arg_strs.join(", ")
            )
            .unwrap();
            writeln!(out, "  {} {} = 0; /* asm result */", result_ty.name(), v).unwrap();
        }
        Op::EnumConstruct { tag, payload } => {
            let payload_desc = payload
                .iter()
                .map(|value| format!("__v{}", value.0))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(
                out,
                "  /* unsupported enum construct: tag={} payload=[{}] */",
                tag, payload_desc
            )
            .unwrap();
            writeln!(
                out,
                "  {} {} = {};",
                result_ty.name(),
                v,
                result_ty.default_value()
            )
            .unwrap();
        }
        Op::EnumTag(value) => {
            writeln!(
                out,
                "  /* unsupported enum tag extraction from __v{} */",
                value.0
            )
            .unwrap();
            writeln!(
                out,
                "  {} {} = {};",
                result_ty.name(),
                v,
                result_ty.default_value()
            )
            .unwrap();
        }
        Op::EnumPayload { value, field_index } => {
            writeln!(
                out,
                "  /* unsupported enum payload extraction: __v{} field {} */",
                value.0, field_index
            )
            .unwrap();
            writeln!(
                out,
                "  {} {} = {};",
                result_ty.name(),
                v,
                result_ty.default_value()
            )
            .unwrap();
        }
    }
}

fn emit_terminator(out: &mut String, term: &Terminator, _layout: &FunctionLayout, is_main: bool) {
    match term {
        Terminator::Return(val) => {
            if is_main {
                writeln!(out, "  return (int)__v{};", val.0).unwrap();
            } else {
                writeln!(out, "  return __v{};", val.0).unwrap();
            }
        }
        Terminator::ReturnVoid => {
            writeln!(out, "  return 0;").unwrap();
        }
        Terminator::Jump(block) => {
            writeln!(out, "  goto block_{};", block.0).unwrap();
        }
        Terminator::Branch {
            condition,
            then_block,
            else_block,
        } => {
            writeln!(
                out,
                "  if (__v{}) goto block_{}; else goto block_{};",
                condition.0, then_block.0, else_block.0
            )
            .unwrap();
        }
        Terminator::Switch {
            discriminant,
            cases,
            default,
        } => {
            writeln!(out, "  switch (__v{}) {{", discriminant.0).unwrap();
            for (value, target) in cases {
                writeln!(out, "    case {}: goto block_{};", value, target.0).unwrap();
            }
            writeln!(out, "    default: goto block_{};", default.0).unwrap();
            writeln!(out, "  }}").unwrap();
        }
        Terminator::Unreachable => {
            writeln!(out, "  __builtin_unreachable();").unwrap();
        }
    }
}

fn emit_print_value(out: &mut String, value: ValueId, ty: CType) {
    match ty {
        CType::Str => {
            writeln!(out, "  printf(\"%s\", (agam_str)__v{});", value.0).unwrap();
        }
        CType::OpaquePtr => {
            writeln!(out, "  printf(\"%p\", (void*)__v{});", value.0).unwrap();
        }
        CType::Float => {
            writeln!(out, "  printf(\"%.17g\", (double)__v{});", value.0).unwrap();
        }
        CType::Bool => {
            writeln!(
                out,
                "  printf(\"%s\", __v{} ? \"true\" : \"false\");",
                value.0
            )
            .unwrap();
        }
        CType::Int => {
            writeln!(out, "  printf(\"%lld\", (long long)__v{});", value.0).unwrap();
        }
        CType::Tensor => {
            writeln!(out, "  printf(\"<tensor:%p>\", (void*)__v{});", value.0).unwrap();
        }
        CType::DataFrame => {
            writeln!(out, "  printf(\"<dataframe:%p>\", (void*)__v{});", value.0).unwrap();
        }
    }
}

fn binop_to_c(op: MirBinOp) -> &'static str {
    match op {
        MirBinOp::Add => "+",
        MirBinOp::Sub => "-",
        MirBinOp::Mul => "*",
        MirBinOp::Div => "/",
        MirBinOp::Mod => "%",
        MirBinOp::Eq => "==",
        MirBinOp::NotEq => "!=",
        MirBinOp::Lt => "<",
        MirBinOp::LtEq => "<=",
        MirBinOp::Gt => ">",
        MirBinOp::GtEq => ">=",
        MirBinOp::And => "&&",
        MirBinOp::Or => "||",
        MirBinOp::BitAnd => "&",
        MirBinOp::BitOr => "|",
        MirBinOp::BitXor => "^",
        MirBinOp::Shl => "<<",
        MirBinOp::Shr => ">>",
    }
}

fn unop_to_c(op: MirUnOp) -> &'static str {
    match op {
        MirUnOp::Neg => "-",
        MirUnOp::Not => "!",
        MirUnOp::BitNot => "~",
    }
}

fn mangle_name(name: &str) -> String {
    format!("agam_{}", name.replace("::", "_").replace(".", "_"))
}

fn mangle_local(name: &str) -> String {
    format!("_local_{}", name.replace("::", "_").replace(".", "_"))
}

fn escape_c_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;
    use agam_mir::lower::MirLowering;

    fn compile_to_c(source: &str) -> String {
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

        emit_c(&mir)
    }

    #[test]
    fn test_emit_main_function() {
        let c = compile_to_c("fn main(): return 42");
        assert!(
            c.contains("int main("),
            "C output should have main function"
        );
        assert!(c.contains("42"), "C output should have the literal 42");
        assert!(c.contains("return"), "C output should have return");
    }

    #[test]
    fn test_emit_includes() {
        let c = compile_to_c("fn main(): return 0");
        assert!(c.contains("#include <stdio.h>"));
        assert!(c.contains("#include <stdint.h>"));
    }

    #[test]
    fn test_emit_binary_op() {
        let c = compile_to_c("fn main(): let x = 1 + 2");
        assert!(c.contains("+"), "C output should contain addition");
    }

    #[test]
    fn test_emit_print_call() {
        let c = compile_to_c("fn main(): print(42)");
        assert!(
            c.contains("printf"),
            "C output should emit printf for print()"
        );
    }

    #[test]
    fn test_emit_type_aliases() {
        let c = compile_to_c("fn main(): return 0");
        assert!(c.contains("typedef int64_t agam_int;"));
        assert!(c.contains("typedef double agam_float;"));
    }

    #[test]
    fn test_emit_string_local_type() {
        let c = compile_to_c("fn main() { let name = \"World\"; print(name); }");
        assert!(c.contains("agam_str __v"));
        assert!(c.contains("printf(\"%s\""));
    }

    #[test]
    fn test_emit_float_type() {
        let c = compile_to_c("fn main() { let x = 1.5 + 2.5; }");
        assert!(c.contains("agam_float"));
    }

    #[test]
    fn test_non_main_returns_do_not_truncate_to_int() {
        let c =
            compile_to_c("fn add(a: i32) -> i32 { return a + 1; } fn main() { return add(41); }");
        assert!(c.contains("agam_add("));
        assert!(c.contains("return __v"));
    }

    #[test]
    fn test_ml_runtime_handles_emit_without_external_stubs() {
        let c = compile_to_c(
            "fn main() { \
                let df = dataframe_build_sin(32); \
                let filtered = dataframe_filter_gt(df, 0.5); \
                let grouped = dataframe_group_by(filtered, 8); \
                let mean = dataframe_mean(grouped); \
                print(mean); \
                dataframe_free(grouped); \
                dataframe_free(filtered); \
                dataframe_free(df); \
            }",
        );
        assert!(c.contains("typedef struct AgamDataFrame"));
        assert!(c.contains("AgamDataFrame* __v"));
        assert!(!c.contains("/* ── External function stubs ── */"));
    }

    #[test]
    fn test_tensor_runtime_handles_emit() {
        let c = compile_to_c(
            "fn main() { \
                let input = tensor_fill_rand(8, 8, 0.25); \
                let dense = dense_layer(input, 4, 0.5); \
                let conv = conv2d(input, 3, 0.75); \
                let score = tensor_checksum(dense) + tensor_checksum(conv); \
                print(score); \
                tensor_free(conv); \
                tensor_free(dense); \
                tensor_free(input); \
            }",
        );
        assert!(c.contains("typedef struct AgamTensor"));
        assert!(c.contains("AgamTensor* __v"));
        assert!(c.contains("agam_dense_layer"));
        assert!(c.contains("agam_conv2d"));
    }

    #[test]
    fn test_adam_builtin_returns_float() {
        let c = compile_to_c(
            "fn main() { let loss = adam(-1.0, 2.0, 0.001, 64, 0.0001); print(loss); }",
        );
        assert!(c.contains("agam_adam"));
        assert!(c.contains("agam_float __v"));
    }

    #[test]
    fn test_full_pipeline() {
        let c = compile_to_c("fn main(): let x = 10 + 20");
        // Should produce valid-looking C code
        assert!(c.contains("int main("));
        assert!(c.contains("return"));
    }

    #[test]
    fn test_emit_effect_perform_filesystem() {
        let c = compile_to_c("fn main() { perform FileSystem.exists(\".\"); }");
        assert!(
            c.contains("agam_effect_FileSystem_exists"),
            "C output should call agam_effect_FileSystem_exists, not a TODO stub"
        );
        assert!(
            !c.contains("/* TODO: effect"),
            "C output should not contain TODO effect stubs"
        );
    }

    #[test]
    fn test_emit_effect_perform_console() {
        let c = compile_to_c("fn main() { perform Console.println(\"hello\"); }");
        assert!(
            c.contains("agam_effect_Console_println"),
            "C output should call agam_effect_Console_println"
        );
    }

    #[test]
    fn test_effect_prelude_emitted() {
        let c = compile_to_c("fn main(): return 0");
        assert!(
            c.contains("Agam Effect Runtime"),
            "C output should include the effect runtime prelude"
        );
        assert!(
            c.contains("agam_effect_FileSystem_exists"),
            "effect prelude should define FileSystem.exists"
        );
        assert!(
            c.contains("agam_effect_Console_println"),
            "effect prelude should define Console.println"
        );
    }

    #[test]
    fn test_gpu_host_prelude_emitted() {
        let c = compile_to_c(
            "fn main() { let ptr = gpu_malloc(16); gpu_memcpy_to_device(ptr, \"abcd\", 4); gpu_memcpy_to_host(\"wxyz\", ptr, 4); gpu_free(ptr); }",
        );
        assert!(c.contains("void* agam_gpu_malloc"));
        assert!(c.contains("agam_gpu_memcpy_to_device"));
        assert!(c.contains("agam_gpu_memcpy_to_host"));
        assert!(c.contains("agam_gpu_free"));
        assert!(c.contains("void* __v"));
    }
}
