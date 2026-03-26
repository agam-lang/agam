//! Cranelift-backed JIT execution for the current Phase 14 scalar MIR subset.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString, c_char};
use std::mem;
use std::sync::OnceLock;
use std::time::Instant;

use agam_mir::ir::*;
use agam_sema::types::{FloatSize, IntSize, Type, builtin_type_by_id};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::immediates::{Ieee32, Ieee64};
use cranelift_codegen::ir::{
    AbiParam, InstBuilder, StackSlot, StackSlotData, StackSlotKind, Type as ClifType,
    UserFuncName, Value, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JitType {
    Int { bits: u16, signed: bool },
    Float32,
    Float64,
    Bool,
    Str,
    OpaquePtr,
    Unit,
}

impl JitType {
    fn clif_type(self, pointer_type: ClifType) -> ClifType {
        match self {
            JitType::Int { bits, .. } => match bits {
                8 => types::I8,
                16 => types::I16,
                32 => types::I32,
                64 => types::I64,
                128 => types::I128,
                _ => types::I64,
            },
            JitType::Float32 => types::F32,
            JitType::Float64 => types::F64,
            JitType::Bool | JitType::Unit => types::I8,
            JitType::Str | JitType::OpaquePtr => pointer_type,
        }
    }

    fn stack_size(self, pointer_type: ClifType) -> u32 {
        self.clif_type(pointer_type).bytes().into()
    }

    fn align_shift(self, pointer_type: ClifType) -> u8 {
        match self.stack_size(pointer_type) {
            0 | 1 => 0,
            2 => 1,
            4 => 2,
            8 => 3,
            _ => 4,
        }
    }

    fn is_float(self) -> bool {
        matches!(self, JitType::Float32 | JitType::Float64)
    }

    fn is_pointer_like(self) -> bool {
        matches!(self, JitType::Str | JitType::OpaquePtr)
    }

    fn int_spec(self) -> Option<(u16, bool)> {
        match self {
            JitType::Int { bits, signed } => Some((bits, signed)),
            JitType::Bool | JitType::Unit => Some((8, false)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct BuiltinSig {
    return_ty: JitType,
}

#[derive(Clone)]
struct FunctionLayout {
    params: Vec<JitType>,
    return_ty: JitType,
    value_types: HashMap<ValueId, JitType>,
    local_types: HashMap<String, JitType>,
}

#[derive(Default)]
struct RuntimeArgs {
    storage: Vec<CString>,
    argv: Vec<*const c_char>,
}

thread_local! {
    static JIT_RUNTIME_ARGS: RefCell<RuntimeArgs> = RefCell::new(RuntimeArgs::default());
}

static START_TIME: OnceLock<Instant> = OnceLock::new();

const RT_PRINT_I64: &str = "__agam_jit_print_i64";
const RT_PRINT_U64: &str = "__agam_jit_print_u64";
const RT_PRINT_F64: &str = "__agam_jit_print_f64";
const RT_PRINT_STR: &str = "__agam_jit_print_str";
const RT_PRINT_BOOL: &str = "__agam_jit_print_bool";
const RT_PRINT_NEWLINE: &str = "__agam_jit_print_newline";
const RT_ARGC: &str = "__agam_jit_argc";
const RT_ARGV: &str = "__agam_jit_argv";
const RT_PARSE_INT: &str = "__agam_jit_parse_int";
const RT_CLOCK: &str = "__agam_jit_clock";

pub fn run_main(module: &MirModule, args: &[String]) -> Result<i32, String> {
    let layouts = analyze_module(module);
    let main_layout = layouts
        .get("main")
        .cloned()
        .ok_or_else(|| "missing `main` function for JIT execution".to_string())?;

    if !main_layout.params.is_empty() {
        return Err("`agamc run --backend jit` currently requires `main` without parameters; use `argc()` / `argv()` inside Agam instead".into());
    }

    let mut builder = JITBuilder::new(default_libcall_names())
        .map_err(|e| format!("failed to create Cranelift JIT builder: {e}"))?;
    register_runtime_symbols(&mut builder);

    let mut jit = AgamJit {
        module: JITModule::new(builder),
        layouts,
        func_ids: HashMap::new(),
        imported_funcs: HashMap::new(),
        string_data: HashMap::new(),
        next_string_id: 0,
    };

    jit.declare_functions(module)?;
    jit.declare_strings(module)?;
    jit.define_functions(module)?;
    jit.module
        .finalize_definitions()
        .map_err(|e| format!("failed to finalize JIT definitions: {e}"))?;

    let main_func = *jit
        .func_ids
        .get("main")
        .ok_or_else(|| "missing JIT handle for `main`".to_string())?;
    let main_ptr = jit.module.get_finalized_function(main_func);

    with_runtime_args(args, || unsafe { call_main(main_ptr, main_layout) })
}

struct AgamJit {
    module: JITModule,
    layouts: HashMap<String, FunctionLayout>,
    func_ids: HashMap<String, FuncId>,
    imported_funcs: HashMap<String, FuncId>,
    string_data: HashMap<String, DataId>,
    next_string_id: usize,
}

impl AgamJit {
    fn declare_functions(&mut self, module: &MirModule) -> Result<(), String> {
        for func in &module.functions {
            let layout = self
                .layouts
                .get(&func.name)
                .ok_or_else(|| format!("missing JIT layout for `{}`", func.name))?;
            let signature = self.signature_for(layout);
            let func_id = self
                .module
                .declare_function(&func.name, Linkage::Local, &signature)
                .map_err(|e| format!("failed to declare JIT function `{}`: {e}", func.name))?;
            self.func_ids.insert(func.name.clone(), func_id);
        }
        Ok(())
    }

    fn declare_strings(&mut self, module: &MirModule) -> Result<(), String> {
        for func in &module.functions {
            for block in &func.blocks {
                for instr in &block.instructions {
                    if let Op::ConstString(value) = &instr.op {
                        let _ = self.string_data_id(value)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn define_functions(&mut self, module: &MirModule) -> Result<(), String> {
        for func in &module.functions {
            self.define_function(func)?;
        }
        Ok(())
    }

    fn define_function(&mut self, func: &MirFunction) -> Result<(), String> {
        let layout = self
            .layouts
            .get(&func.name)
            .cloned()
            .ok_or_else(|| format!("missing JIT layout for `{}`", func.name))?;
        let func_id = *self
            .func_ids
            .get(&func.name)
            .ok_or_else(|| format!("missing JIT id for `{}`", func.name))?;

        let mut ctx = self.module.make_context();
        ctx.func.signature = self.signature_for(&layout);
        ctx.func.name = UserFuncName::user(0, func_id.as_u32());

        let mut func_ctx = FunctionBuilderContext::new();
        {
            let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
            let pointer_type = self.module.target_config().pointer_type();
            let mem_flags = cranelift_codegen::ir::MemFlags::new();

            let mut blocks = HashMap::new();
            for block in &func.blocks {
                blocks.insert(block.id, builder.create_block());
            }

            let entry_block = *blocks
                .get(&func.entry)
                .ok_or_else(|| format!("missing entry block for `{}`", func.name))?;
            builder.switch_to_block(entry_block);
            builder.append_block_params_for_function_params(entry_block);

            let param_block_values = builder.block_params(entry_block).to_vec();
            let mut local_slots = HashMap::new();
            for (name, ty) in &layout.local_types {
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    ty.stack_size(pointer_type),
                    ty.align_shift(pointer_type),
                ));
                local_slots.insert(name.clone(), slot);
            }

            let param_names: HashSet<&str> =
                func.params.iter().map(|param| param.name.as_str()).collect();
            for (index, param) in func.params.iter().enumerate() {
                let slot = *local_slots
                    .get(&param.name)
                    .ok_or_else(|| format!("missing local slot for parameter `{}`", param.name))?;
                let value = *param_block_values.get(index).ok_or_else(|| {
                    format!(
                        "missing entry block parameter {} while compiling `{}`",
                        index, func.name
                    )
                })?;
                builder.ins().stack_store(value, slot, 0);
            }
            for (name, ty) in &layout.local_types {
                if param_names.contains(name.as_str()) {
                    continue;
                }
                let slot = *local_slots
                    .get(name)
                    .ok_or_else(|| format!("missing local slot for `{}`", name))?;
                let zero = default_value(&mut builder, *ty, pointer_type);
                builder.ins().stack_store(zero, slot, 0);
            }

            let mut values = HashMap::new();
            for (index, param) in func.params.iter().enumerate() {
                values.insert(param.value, param_block_values[index]);
            }

            for block in &func.blocks {
                let cl_block = *blocks
                    .get(&block.id)
                    .ok_or_else(|| format!("missing JIT block {}", block.id.0))?;
                if block.id != func.entry {
                    builder.switch_to_block(cl_block);
                }

                for instr in &block.instructions {
                    let value = self.emit_instruction(
                        &mut builder,
                        &layout,
                        &local_slots,
                        &values,
                        instr,
                        mem_flags,
                    )?;
                    values.insert(instr.result, value);
                }

                self.emit_terminator(
                    &mut builder,
                    &layout,
                    &blocks,
                    &values,
                    &block.terminator,
                    mem_flags,
                )?;
            }

            builder.seal_all_blocks();
            builder.finalize();
        }

        self.module
            .define_function(func_id, &mut ctx)
            .map_err(|e| {
                format!(
                    "failed to define JIT function `{}`: {e}\ncranelift ir:\n{}",
                    func.name,
                    ctx.func.display()
                )
            })?;
        self.module.clear_context(&mut ctx);
        Ok(())
    }

    fn emit_instruction(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        layout: &FunctionLayout,
        local_slots: &HashMap<String, StackSlot>,
        values: &HashMap<ValueId, Value>,
        instr: &Instruction,
        mem_flags: cranelift_codegen::ir::MemFlags,
    ) -> Result<Value, String> {
        let pointer_type = self.module.target_config().pointer_type();
        let result_ty = value_type(layout, instr.result);
        match &instr.op {
            Op::ConstInt(value) => Ok(builder.ins().iconst(result_ty.clif_type(pointer_type), *value)),
            Op::ConstFloat(value) => match result_ty {
                JitType::Float32 => Ok(builder.ins().f32const(Ieee32::with_float(*value as f32))),
                _ => Ok(builder.ins().f64const(Ieee64::with_float(*value))),
            },
            Op::ConstBool(value) => Ok(builder
                .ins()
                .iconst(result_ty.clif_type(pointer_type), i64::from(*value))),
            Op::ConstString(value) => {
                let data_id = self.string_data_id(value)?;
                let gv = self.module.declare_data_in_func(data_id, builder.func);
                Ok(builder.ins().symbol_value(pointer_type, gv))
            }
            Op::Unit => Ok(default_value(builder, JitType::Unit, pointer_type)),
            Op::Copy(value) => lookup_value(values, *value),
            Op::LoadLocal(name) => {
                let slot = *local_slots
                    .get(name)
                    .ok_or_else(|| format!("unknown local `{name}` in JIT load"))?;
                Ok(builder
                    .ins()
                    .stack_load(result_ty.clif_type(pointer_type), slot, 0))
            }
            Op::StoreLocal { name, value } => {
                let slot = *local_slots
                    .get(name)
                    .ok_or_else(|| format!("unknown local `{name}` in JIT store"))?;
                let source_ty = value_type(layout, *value);
                let target_ty = layout.local_types.get(name).copied().unwrap_or(source_ty);
                let value = self.coerce_value(
                    builder,
                    lookup_value(values, *value)?,
                    source_ty,
                    target_ty,
                    mem_flags,
                )?;
                builder.ins().stack_store(value, slot, 0);
                Ok(value)
            }
            Op::Alloca { name, ty } => {
                let slot = *local_slots
                    .get(name)
                    .ok_or_else(|| format!("unknown local `{name}` in JIT alloca"))?;
                let local_ty = layout
                    .local_types
                    .get(name)
                    .copied()
                    .or_else(|| infer_jit_type_from_type_id(*ty))
                    .unwrap_or(JitType::Int {
                        bits: 32,
                        signed: true,
                    });
                let zero = default_value(builder, local_ty, pointer_type);
                builder.ins().stack_store(zero, slot, 0);
                Ok(default_value(builder, result_ty, pointer_type))
            }
            Op::BinOp { op, left, right } => {
                let left_ty = value_type(layout, *left);
                let right_ty = value_type(layout, *right);
                let operand_ty = binop_operand_type(*op, left_ty, right_ty, result_ty);
                let left_val = self.coerce_value(
                    builder,
                    lookup_value(values, *left)?,
                    left_ty,
                    operand_ty,
                    mem_flags,
                )?;
                let right_val = self.coerce_value(
                    builder,
                    lookup_value(values, *right)?,
                    right_ty,
                    operand_ty,
                    mem_flags,
                )?;
                emit_binop(
                    builder,
                    *op,
                    left_val,
                    right_val,
                    operand_ty,
                    operand_ty,
                    result_ty,
                )
            }
            Op::UnOp { op, operand } => {
                let operand_val = lookup_value(values, *operand)?;
                let operand_ty = value_type(layout, *operand);
                emit_unop(builder, *op, operand_val, operand_ty, result_ty)
            }
            Op::Call { callee, args } => {
                if is_print_builtin(callee) {
                    self.emit_print_call(builder, layout, values, args)?;
                    Ok(default_value(builder, result_ty, pointer_type))
                } else {
                    let user_param_tys = self.layouts.get(callee).map(|layout| layout.params.clone());
                    let builtin_param_tys = builtin_arg_types(callee);
                    let func_ref = if let Some(func_id) = self.func_ids.get(callee) {
                        self.module.declare_func_in_func(*func_id, builder.func)
                    } else if builtin_signature(callee).is_some() {
                        let builtin_id = self.builtin_func_id(callee)?;
                        self.module.declare_func_in_func(builtin_id, builder.func)
                    } else {
                        return Err(format!("unsupported JIT call target `{callee}`"));
                    };
                    let mut lowered_args = Vec::with_capacity(args.len());
                    for (index, arg) in args.iter().enumerate() {
                        let source_ty = value_type(layout, *arg);
                        let target_ty = user_param_tys
                            .as_ref()
                            .and_then(|params| params.get(index).copied())
                            .or_else(|| {
                                builtin_param_tys
                                    .as_ref()
                                    .and_then(|params| params.get(index).copied())
                            })
                            .unwrap_or(source_ty);
                        let value = self.coerce_value(
                            builder,
                            lookup_value(values, *arg)?,
                            source_ty,
                            target_ty,
                            mem_flags,
                        )?;
                        lowered_args.push(value);
                    }
                    let call = builder.ins().call(func_ref, &lowered_args);
                    let results = builder.inst_results(call);
                    if results.is_empty() {
                        Ok(default_value(builder, result_ty, pointer_type))
                    } else {
                        Ok(results[0])
                    }
                }
            }
            Op::GetField { object, .. } => lookup_value(values, *object),
            Op::GetIndex { object, .. } => lookup_value(values, *object),
            Op::Phi(_) => Err("MIR phi nodes are not yet supported by the Cranelift JIT slice".into()),
            Op::Cast {
                value: value_id,
                target_ty,
            } => {
                let value = lookup_value(values, *value_id)?;
                let source_ty = value_type(layout, *value_id);
                let target_ty = infer_jit_type_from_type_id(*target_ty).unwrap_or(result_ty);
                emit_cast(builder, value, source_ty, target_ty, mem_flags, pointer_type)
            }
        }
    }

    fn emit_terminator(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        layout: &FunctionLayout,
        blocks: &HashMap<BlockId, cranelift_codegen::ir::Block>,
        values: &HashMap<ValueId, Value>,
        terminator: &Terminator,
        mem_flags: cranelift_codegen::ir::MemFlags,
    ) -> Result<(), String> {
        match terminator {
            Terminator::Return(value) => {
                if layout.return_ty == JitType::Unit {
                    builder.ins().return_(&[]);
                } else {
                    let source_ty = value_type(layout, *value);
                    let value = self.coerce_value(
                        builder,
                        lookup_value(values, *value)?,
                        source_ty,
                        layout.return_ty,
                        mem_flags,
                    )?;
                    builder.ins().return_(&[value]);
                }
            }
            Terminator::ReturnVoid => {
                builder.ins().return_(&[]);
            }
            Terminator::Jump(block) => {
                let target = *blocks
                    .get(block)
                    .ok_or_else(|| format!("missing JIT jump target {}", block.0))?;
                builder.ins().jump(target, &[]);
            }
            Terminator::Branch {
                condition,
                then_block,
                else_block,
            } => {
                let condition_id = *condition;
                let condition = lookup_value(values, condition_id)?;
                let then_block = *blocks
                    .get(then_block)
                    .ok_or_else(|| format!("missing JIT branch target {}", then_block.0))?;
                let else_block = *blocks
                    .get(else_block)
                    .ok_or_else(|| format!("missing JIT branch target {}", else_block.0))?;
                let condition = self.coerce_value(
                    builder,
                    condition,
                    value_type(layout, condition_id),
                    JitType::Bool,
                    mem_flags,
                )?;
                builder.ins().brif(condition, then_block, &[], else_block, &[]);
            }
            Terminator::Unreachable => {
                builder
                    .ins()
                    .trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
            }
        }
        Ok(())
    }

    fn emit_print_call(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        layout: &FunctionLayout,
        values: &HashMap<ValueId, Value>,
        args: &[ValueId],
    ) -> Result<(), String> {
        for arg in args {
            let value = lookup_value(values, *arg)?;
            let ty = value_type(layout, *arg);
            match ty {
                JitType::Float32 | JitType::Float64 => {
                    let func = self.runtime_func_id(RT_PRINT_F64, &[JitType::Float64], None)?;
                    let func_ref = self.module.declare_func_in_func(func, builder.func);
                    let float = if ty == JitType::Float32 {
                        builder.ins().fpromote(types::F64, value)
                    } else {
                        value
                    };
                    builder.ins().call(func_ref, &[float]);
                }
                JitType::Str | JitType::OpaquePtr => {
                    let func = self.runtime_func_id(RT_PRINT_STR, &[JitType::Str], None)?;
                    let func_ref = self.module.declare_func_in_func(func, builder.func);
                    builder.ins().call(func_ref, &[value]);
                }
                JitType::Bool => {
                    let func = self.runtime_func_id(RT_PRINT_BOOL, &[JitType::Bool], None)?;
                    let func_ref = self.module.declare_func_in_func(func, builder.func);
                    builder.ins().call(func_ref, &[value]);
                }
                JitType::Unit => {}
                JitType::Int { signed, .. } => {
                    let normalized = normalize_int(builder, value, ty, 64, signed);
                    let symbol = if signed { RT_PRINT_I64 } else { RT_PRINT_U64 };
                    let func = self.runtime_func_id(
                        symbol,
                        &[JitType::Int {
                            bits: 64,
                            signed,
                        }],
                        None,
                    )?;
                    let func_ref = self.module.declare_func_in_func(func, builder.func);
                    builder.ins().call(func_ref, &[normalized]);
                }
            }
        }

        let newline = self.runtime_func_id(RT_PRINT_NEWLINE, &[], None)?;
        let newline_ref = self.module.declare_func_in_func(newline, builder.func);
        builder.ins().call(newline_ref, &[]);
        Ok(())
    }

    fn coerce_value(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        value: Value,
        source_ty: JitType,
        target_ty: JitType,
        mem_flags: cranelift_codegen::ir::MemFlags,
    ) -> Result<Value, String> {
        let pointer_type = self.module.target_config().pointer_type();
        emit_cast(builder, value, source_ty, target_ty, mem_flags, pointer_type)
    }

    fn builtin_func_id(&mut self, name: &str) -> Result<FuncId, String> {
        match name {
            "argc" => self.runtime_func_id(
                RT_ARGC,
                &[],
                Some(JitType::Int {
                    bits: 32,
                    signed: true,
                }),
            ),
            "argv" => self.runtime_func_id(
                RT_ARGV,
                &[JitType::Int {
                    bits: 32,
                    signed: true,
                }],
                Some(JitType::Str),
            ),
            "parse_int" => self.runtime_func_id(
                RT_PARSE_INT,
                &[JitType::Str],
                Some(JitType::Int {
                    bits: 32,
                    signed: true,
                }),
            ),
            "clock" => self.runtime_func_id(RT_CLOCK, &[], Some(JitType::Float64)),
            _ => Err(format!("unsupported JIT builtin `{name}`")),
        }
    }

    fn runtime_func_id(
        &mut self,
        name: &str,
        params: &[JitType],
        return_ty: Option<JitType>,
    ) -> Result<FuncId, String> {
        if let Some(func_id) = self.imported_funcs.get(name) {
            return Ok(*func_id);
        }

        let mut signature = self.module.make_signature();
        let pointer_type = self.module.target_config().pointer_type();
        signature.params.extend(
            params
                .iter()
                .map(|ty| AbiParam::new(ty.clif_type(pointer_type))),
        );
        if let Some(return_ty) = return_ty {
            signature
                .returns
                .push(AbiParam::new(return_ty.clif_type(pointer_type)));
        }

        let func_id = self
            .module
            .declare_function(name, Linkage::Import, &signature)
            .map_err(|e| format!("failed to declare runtime JIT import `{name}`: {e}"))?;
        self.imported_funcs.insert(name.to_string(), func_id);
        Ok(func_id)
    }

    fn string_data_id(&mut self, value: &str) -> Result<DataId, String> {
        if let Some(data_id) = self.string_data.get(value) {
            return Ok(*data_id);
        }

        let symbol = format!("__agam_jit_str_{}", self.next_string_id);
        self.next_string_id += 1;
        let data_id = self
            .module
            .declare_data(&symbol, Linkage::Local, false, false)
            .map_err(|e| format!("failed to declare JIT string data `{symbol}`: {e}"))?;

        let mut data = DataDescription::new();
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0);
        data.define(bytes.into_boxed_slice());
        self.module
            .define_data(data_id, &data)
            .map_err(|e| format!("failed to define JIT string data `{symbol}`: {e}"))?;

        self.string_data.insert(value.to_string(), data_id);
        Ok(data_id)
    }

    fn signature_for(&self, layout: &FunctionLayout) -> cranelift_codegen::ir::Signature {
        let mut signature = self.module.make_signature();
        let pointer_type = self.module.target_config().pointer_type();
        signature.params.extend(
            layout
                .params
                .iter()
                .map(|ty| AbiParam::new(ty.clif_type(pointer_type))),
        );
        if layout.return_ty != JitType::Unit {
            signature
                .returns
                .push(AbiParam::new(layout.return_ty.clif_type(pointer_type)));
        }
        signature
    }
}

fn register_runtime_symbols(builder: &mut JITBuilder) {
    builder.symbol(RT_PRINT_I64, rt_print_i64 as *const u8);
    builder.symbol(RT_PRINT_U64, rt_print_u64 as *const u8);
    builder.symbol(RT_PRINT_F64, rt_print_f64 as *const u8);
    builder.symbol(RT_PRINT_STR, rt_print_str as *const u8);
    builder.symbol(RT_PRINT_BOOL, rt_print_bool as *const u8);
    builder.symbol(RT_PRINT_NEWLINE, rt_print_newline as *const u8);
    builder.symbol(RT_ARGC, rt_argc as *const u8);
    builder.symbol(RT_ARGV, rt_argv as *const u8);
    builder.symbol(RT_PARSE_INT, rt_parse_int as *const u8);
    builder.symbol(RT_CLOCK, rt_clock as *const u8);
}

unsafe fn call_main(main_ptr: *const u8, main_layout: FunctionLayout) -> Result<i32, String> {
    Ok(match main_layout.return_ty {
        JitType::Unit => {
            let func = unsafe { mem::transmute::<_, extern "C" fn()>(main_ptr) };
            func();
            0
        }
        JitType::Float32 => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> f32>(main_ptr) };
            func() as i32
        }
        JitType::Float64 => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> f64>(main_ptr) };
            func() as i32
        }
        JitType::Str | JitType::OpaquePtr => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> usize>(main_ptr) };
            func() as i32
        }
        JitType::Bool => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> u8>(main_ptr) };
            i32::from(func() != 0)
        }
        JitType::Int { bits: 8, signed: true } => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> i8>(main_ptr) };
            i32::from(func())
        }
        JitType::Int { bits: 8, signed: false } => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> u8>(main_ptr) };
            i32::from(func())
        }
        JitType::Int { bits: 16, signed: true } => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> i16>(main_ptr) };
            i32::from(func())
        }
        JitType::Int {
            bits: 16,
            signed: false,
        } => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> u16>(main_ptr) };
            i32::from(func())
        }
        JitType::Int { bits: 32, signed: true } => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> i32>(main_ptr) };
            func()
        }
        JitType::Int {
            bits: 32,
            signed: false,
        } => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> u32>(main_ptr) };
            func() as i32
        }
        JitType::Int { bits: 64, signed: true } => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> i64>(main_ptr) };
            func() as i32
        }
        JitType::Int {
            bits: 64,
            signed: false,
        } => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> u64>(main_ptr) };
            func() as i32
        }
        JitType::Int {
            bits: 128,
            signed: true,
        } => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> i128>(main_ptr) };
            func() as i32
        }
        JitType::Int {
            bits: 128,
            signed: false,
        } => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> u128>(main_ptr) };
            func() as i32
        }
        JitType::Int { bits, .. } => {
            return Err(format!("unsupported JIT main return width: {bits}"));
        }
    })
}

fn analyze_module(module: &MirModule) -> HashMap<String, FunctionLayout> {
    let return_types: HashMap<String, JitType> = module
        .functions
        .iter()
        .map(|func| {
            (
                func.name.clone(),
                infer_jit_type_from_type_id(func.return_ty).unwrap_or(JitType::Int {
                    bits: 32,
                    signed: true,
                }),
            )
        })
        .collect();

    module
        .functions
        .iter()
        .map(|func| (func.name.clone(), analyze_function(func, &return_types)))
        .collect()
}

fn analyze_function(func: &MirFunction, return_types: &HashMap<String, JitType>) -> FunctionLayout {
    let mut layout = FunctionLayout {
        params: func
            .params
            .iter()
            .map(|param| {
                infer_jit_type_from_type_id(param.ty).unwrap_or(JitType::Int {
                    bits: 32,
                    signed: true,
                })
            })
            .collect(),
        return_ty: infer_jit_type_from_type_id(func.return_ty).unwrap_or(JitType::Int {
            bits: 32,
            signed: true,
        }),
        value_types: HashMap::new(),
        local_types: HashMap::new(),
    };

    for (index, param) in func.params.iter().enumerate() {
        let ty = layout.params[index];
        layout.value_types.insert(param.value, ty);
        layout.local_types.insert(param.name.clone(), ty);
    }

    for block in &func.blocks {
        for instr in &block.instructions {
            let ty = match &instr.op {
                Op::ConstInt(_) => infer_jit_type_from_type_id(instr.ty).unwrap_or(JitType::Int {
                    bits: 32,
                    signed: true,
                }),
                Op::ConstFloat(_) => infer_jit_type_from_type_id(instr.ty).unwrap_or(JitType::Float64),
                Op::ConstBool(_) => JitType::Bool,
                Op::ConstString(_) => JitType::Str,
                Op::Unit => JitType::Unit,
                Op::Copy(value) => value_type(&layout, *value),
                Op::BinOp { op, left, right } => {
                    infer_binop_type(*op, value_type(&layout, *left), value_type(&layout, *right))
                }
                Op::UnOp { op, operand } => infer_unop_type(*op, value_type(&layout, *operand)),
                Op::Call { callee, .. } => {
                    if is_print_builtin(callee) {
                        infer_jit_type_from_type_id(instr.ty).unwrap_or(JitType::Unit)
                    } else if let Some(sig) = builtin_signature(callee) {
                        sig.return_ty
                    } else {
                        *return_types.get(callee).unwrap_or(&JitType::Int {
                            bits: 32,
                            signed: true,
                        })
                    }
                }
                Op::LoadLocal(name) => layout
                    .local_types
                    .get(name)
                    .copied()
                    .or_else(|| infer_jit_type_from_type_id(instr.ty))
                    .unwrap_or(JitType::Int {
                        bits: 32,
                        signed: true,
                    }),
                Op::StoreLocal { name, value } => {
                    let ty = layout
                        .local_types
                        .get(name)
                        .copied()
                        .or_else(|| infer_jit_type_from_type_id(instr.ty))
                        .unwrap_or_else(|| value_type(&layout, *value));
                    layout.local_types.insert(name.clone(), ty);
                    ty
                }
                Op::Alloca { name, ty } => {
                    let ty = infer_jit_type_from_type_id(*ty).unwrap_or(JitType::Int {
                        bits: 32,
                        signed: true,
                    });
                    layout.local_types.entry(name.clone()).or_insert(ty);
                    JitType::Unit
                }
                Op::GetField { object, .. } | Op::GetIndex { object, .. } => value_type(&layout, *object),
                Op::Phi(entries) => entries
                    .iter()
                    .map(|(_, value)| value_type(&layout, *value))
                    .reduce(merge_type)
                    .unwrap_or(JitType::Int {
                        bits: 32,
                        signed: true,
                    }),
                Op::Cast { target_ty, value } => infer_jit_type_from_type_id(*target_ty)
                    .unwrap_or_else(|| value_type(&layout, *value)),
            };
            layout.value_types.insert(instr.result, ty);
        }
    }

    let return_values: Vec<JitType> = func
        .blocks
        .iter()
        .filter_map(|block| match block.terminator {
            Terminator::Return(value) => Some(value_type(&layout, value)),
            _ => None,
        })
        .collect();
    if let Some(return_ty) = return_values.into_iter().reduce(merge_type) {
        layout.return_ty = return_ty;
    }

    layout
}

fn infer_jit_type_from_type_id(type_id: agam_sema::symbol::TypeId) -> Option<JitType> {
    match builtin_type_by_id(type_id)? {
        Type::Int(size) => Some(JitType::Int {
            bits: int_bits(size),
            signed: true,
        }),
        Type::UInt(size) => Some(JitType::Int {
            bits: int_bits(size),
            signed: false,
        }),
        Type::Float(FloatSize::F32) => Some(JitType::Float32),
        Type::Float(FloatSize::F64) => Some(JitType::Float64),
        Type::Bool => Some(JitType::Bool),
        Type::Char => Some(JitType::Int {
            bits: 32,
            signed: false,
        }),
        Type::Str => Some(JitType::Str),
        Type::Unit | Type::Never => Some(JitType::Unit),
        Type::Ref { .. } | Type::Ptr { .. } => Some(JitType::OpaquePtr),
        Type::Any | Type::Named(_) | Type::Function { .. } | Type::DynTrait(_) => {
            Some(JitType::OpaquePtr)
        }
        _ => None,
    }
}

fn int_bits(size: IntSize) -> u16 {
    match size {
        IntSize::I8 => 8,
        IntSize::I16 => 16,
        IntSize::I32 => 32,
        IntSize::I64 | IntSize::ISize => 64,
        IntSize::I128 => 128,
    }
}

fn infer_binop_type(op: MirBinOp, left: JitType, right: JitType) -> JitType {
    match op {
        MirBinOp::Eq
        | MirBinOp::NotEq
        | MirBinOp::Lt
        | MirBinOp::LtEq
        | MirBinOp::Gt
        | MirBinOp::GtEq
        | MirBinOp::And
        | MirBinOp::Or => JitType::Bool,
        MirBinOp::Add if left == JitType::Str || right == JitType::Str => JitType::Str,
        _ if left == JitType::Float64 || right == JitType::Float64 => JitType::Float64,
        _ if left == JitType::Float32 || right == JitType::Float32 => JitType::Float32,
        _ => left,
    }
}

fn binop_operand_type(op: MirBinOp, left: JitType, right: JitType, result: JitType) -> JitType {
    match op {
        MirBinOp::Eq
        | MirBinOp::NotEq
        | MirBinOp::Lt
        | MirBinOp::LtEq
        | MirBinOp::Gt
        | MirBinOp::GtEq => merge_type(left, right),
        _ => result,
    }
}

fn infer_unop_type(op: MirUnOp, operand: JitType) -> JitType {
    match op {
        MirUnOp::Not => JitType::Bool,
        _ => operand,
    }
}

fn merge_type(left: JitType, right: JitType) -> JitType {
    if left == right {
        left
    } else if left == JitType::Float64 || right == JitType::Float64 {
        JitType::Float64
    } else if left == JitType::Float32 || right == JitType::Float32 {
        JitType::Float32
    } else if left.is_pointer_like() || right.is_pointer_like() {
        JitType::OpaquePtr
    } else if matches!(left, JitType::Int { .. } | JitType::Bool)
        && matches!(right, JitType::Int { .. } | JitType::Bool)
    {
        let (left_bits, left_signed) = left.int_spec().unwrap_or((32, true));
        let (right_bits, right_signed) = right.int_spec().unwrap_or((32, true));
        JitType::Int {
            bits: left_bits.max(right_bits),
            signed: left_signed || right_signed,
        }
    } else {
        left
    }
}

fn emit_binop(
    builder: &mut FunctionBuilder<'_>,
    op: MirBinOp,
    left: Value,
    right: Value,
    left_ty: JitType,
    right_ty: JitType,
    result_ty: JitType,
) -> Result<Value, String> {
    Ok(match op {
        MirBinOp::Add => {
            if result_ty.is_float() {
                builder.ins().fadd(left, right)
            } else {
                builder.ins().iadd(left, right)
            }
        }
        MirBinOp::Sub => {
            if result_ty.is_float() {
                builder.ins().fsub(left, right)
            } else {
                builder.ins().isub(left, right)
            }
        }
        MirBinOp::Mul => {
            if result_ty.is_float() {
                builder.ins().fmul(left, right)
            } else {
                builder.ins().imul(left, right)
            }
        }
        MirBinOp::Div => {
            if result_ty.is_float() {
                builder.ins().fdiv(left, right)
            } else if result_ty.int_spec().map(|(_, signed)| signed).unwrap_or(true) {
                builder.ins().sdiv(left, right)
            } else {
                builder.ins().udiv(left, right)
            }
        }
        MirBinOp::Mod => {
            if result_ty.is_float() {
                return Err("floating-point modulo is not yet supported by the Cranelift JIT slice".into());
            } else if result_ty.int_spec().map(|(_, signed)| signed).unwrap_or(true) {
                builder.ins().srem(left, right)
            } else {
                builder.ins().urem(left, right)
            }
        }
        MirBinOp::Eq | MirBinOp::NotEq | MirBinOp::Lt | MirBinOp::LtEq | MirBinOp::Gt | MirBinOp::GtEq => {
            let cond = if left_ty.is_float() || right_ty.is_float() {
                let cc = match op {
                    MirBinOp::Eq => FloatCC::Equal,
                    MirBinOp::NotEq => FloatCC::NotEqual,
                    MirBinOp::Lt => FloatCC::LessThan,
                    MirBinOp::LtEq => FloatCC::LessThanOrEqual,
                    MirBinOp::Gt => FloatCC::GreaterThan,
                    MirBinOp::GtEq => FloatCC::GreaterThanOrEqual,
                    _ => unreachable!(),
                };
                builder.ins().fcmp(cc, left, right)
            } else {
                let cc = match op {
                    MirBinOp::Eq => IntCC::Equal,
                    MirBinOp::NotEq => IntCC::NotEqual,
                    MirBinOp::Lt => {
                        if left_ty.int_spec().map(|(_, signed)| signed).unwrap_or(false) {
                            IntCC::SignedLessThan
                        } else {
                            IntCC::UnsignedLessThan
                        }
                    }
                    MirBinOp::LtEq => {
                        if left_ty.int_spec().map(|(_, signed)| signed).unwrap_or(false) {
                            IntCC::SignedLessThanOrEqual
                        } else {
                            IntCC::UnsignedLessThanOrEqual
                        }
                    }
                    MirBinOp::Gt => {
                        if left_ty.int_spec().map(|(_, signed)| signed).unwrap_or(false) {
                            IntCC::SignedGreaterThan
                        } else {
                            IntCC::UnsignedGreaterThan
                        }
                    }
                    MirBinOp::GtEq => {
                        if left_ty.int_spec().map(|(_, signed)| signed).unwrap_or(false) {
                            IntCC::SignedGreaterThanOrEqual
                        } else {
                            IntCC::UnsignedGreaterThanOrEqual
                        }
                    }
                    _ => unreachable!(),
                };
                builder.ins().icmp(cc, left, right)
            };
            let one = builder.ins().iconst(types::I8, 1);
            let zero = builder.ins().iconst(types::I8, 0);
            builder.ins().select(cond, one, zero)
        }
        MirBinOp::And => builder.ins().band(left, right),
        MirBinOp::Or => builder.ins().bor(left, right),
        MirBinOp::BitAnd => builder.ins().band(left, right),
        MirBinOp::BitOr => builder.ins().bor(left, right),
        MirBinOp::BitXor => builder.ins().bxor(left, right),
        MirBinOp::Shl => builder.ins().ishl(left, right),
        MirBinOp::Shr => {
            if result_ty.int_spec().map(|(_, signed)| signed).unwrap_or(false) {
                builder.ins().sshr(left, right)
            } else {
                builder.ins().ushr(left, right)
            }
        }
    })
}

fn emit_unop(
    builder: &mut FunctionBuilder<'_>,
    op: MirUnOp,
    operand: Value,
    operand_ty: JitType,
    result_ty: JitType,
) -> Result<Value, String> {
    Ok(match op {
        MirUnOp::Neg => {
            if operand_ty.is_float() {
                builder.ins().fneg(operand)
            } else {
                let zero = builder.ins().iconst(operand_ty.clif_type(types::I64), 0);
                builder.ins().isub(zero, operand)
            }
        }
        MirUnOp::Not => {
            if result_ty == JitType::Bool {
                let is_zero = builder.ins().icmp_imm(IntCC::Equal, operand, 0);
                let one = builder.ins().iconst(types::I8, 1);
                let zero = builder.ins().iconst(types::I8, 0);
                builder.ins().select(is_zero, one, zero)
            } else {
                builder.ins().bnot(operand)
            }
        }
        MirUnOp::BitNot => builder.ins().bnot(operand),
    })
}

fn emit_cast(
    builder: &mut FunctionBuilder<'_>,
    value: Value,
    source_ty: JitType,
    target_ty: JitType,
    mem_flags: cranelift_codegen::ir::MemFlags,
    pointer_type: ClifType,
) -> Result<Value, String> {
    if source_ty == target_ty {
        return Ok(value);
    }

    Ok(match (source_ty, target_ty) {
        (JitType::Float32, JitType::Float64) => builder.ins().fpromote(types::F64, value),
        (JitType::Float64, JitType::Float32) => builder.ins().fdemote(types::F32, value),
        (source, target) if source.is_float() && matches!(target, JitType::Int { .. } | JitType::Bool) => {
            let int_type = target.clif_type(pointer_type);
            if target.int_spec().map(|(_, signed)| signed).unwrap_or(false) {
                builder.ins().fcvt_to_sint(int_type, value)
            } else {
                builder.ins().fcvt_to_uint(int_type, value)
            }
        }
        (source, target)
            if matches!(source, JitType::Int { .. } | JitType::Bool) && target.is_float() =>
        {
            let (_, signed) = source.int_spec().unwrap_or((8, false));
            let normalized = normalize_int(builder, value, source, 64, signed);
            let float_type = target.clif_type(pointer_type);
            if signed {
                builder.ins().fcvt_from_sint(float_type, normalized)
            } else {
                builder.ins().fcvt_from_uint(float_type, normalized)
            }
        }
        (source, target)
            if matches!(source, JitType::Int { .. } | JitType::Bool)
                && matches!(target, JitType::Int { .. } | JitType::Bool) =>
        {
            let (bits, signed) = target.int_spec().unwrap_or((8, false));
            normalize_int(builder, value, source, bits, signed)
        }
        (JitType::Str, JitType::OpaquePtr)
        | (JitType::OpaquePtr, JitType::Str)
        | (JitType::Str, JitType::Str)
        | (JitType::OpaquePtr, JitType::OpaquePtr) => value,
        (JitType::Str, JitType::Int { .. })
        | (JitType::OpaquePtr, JitType::Int { .. })
        | (JitType::Str, JitType::Bool)
        | (JitType::OpaquePtr, JitType::Bool)
        | (JitType::Int { .. }, JitType::Str)
        | (JitType::Int { .. }, JitType::OpaquePtr)
        | (JitType::Bool, JitType::Str)
        | (JitType::Bool, JitType::OpaquePtr) => builder
            .ins()
            .bitcast(target_ty.clif_type(pointer_type), mem_flags, value),
        _ => return Err(format!("unsupported JIT cast from {source_ty:?} to {target_ty:?}")),
    })
}

fn normalize_int(
    builder: &mut FunctionBuilder<'_>,
    value: Value,
    source_ty: JitType,
    target_bits: u16,
    target_signed: bool,
) -> Value {
    let (source_bits, source_signed) = source_ty.int_spec().unwrap_or((8, false));
    let current = source_ty.clif_type(types::I64);
    let target = match target_bits {
        8 => types::I8,
        16 => types::I16,
        32 => types::I32,
        64 => types::I64,
        128 => types::I128,
        _ => types::I64,
    };

    if source_bits == target_bits {
        value
    } else if source_bits > target_bits {
        builder.ins().ireduce(target, value)
    } else if source_signed || target_signed {
        if current == target {
            value
        } else {
            builder.ins().sextend(target, value)
        }
    } else if current == target {
        value
    } else {
        builder.ins().uextend(target, value)
    }
}

fn default_value(builder: &mut FunctionBuilder<'_>, ty: JitType, pointer_type: ClifType) -> Value {
    match ty {
        JitType::Float32 => builder.ins().f32const(Ieee32::with_float(0.0)),
        JitType::Float64 => builder.ins().f64const(Ieee64::with_float(0.0)),
        JitType::Str | JitType::OpaquePtr => builder.ins().iconst(pointer_type, 0),
        _ => builder.ins().iconst(ty.clif_type(pointer_type), 0),
    }
}

fn value_type(layout: &FunctionLayout, value: ValueId) -> JitType {
    layout
        .value_types
        .get(&value)
        .copied()
        .unwrap_or(JitType::Int {
            bits: 32,
            signed: true,
        })
}

fn lookup_value(values: &HashMap<ValueId, Value>, id: ValueId) -> Result<Value, String> {
    values
        .get(&id)
        .copied()
        .ok_or_else(|| format!("missing JIT SSA value __v{}", id.0))
}

fn is_print_builtin(name: &str) -> bool {
    matches!(name, "print" | "println" | "print_int" | "print_str")
}

fn builtin_signature(name: &str) -> Option<BuiltinSig> {
    match name {
        "argc" => Some(BuiltinSig {
            return_ty: JitType::Int {
                bits: 32,
                signed: true,
            },
        }),
        "argv" => Some(BuiltinSig {
            return_ty: JitType::Str,
        }),
        "parse_int" => Some(BuiltinSig {
            return_ty: JitType::Int {
                bits: 32,
                signed: true,
            },
        }),
        "clock" => Some(BuiltinSig {
            return_ty: JitType::Float64,
        }),
        _ => None,
    }
}

fn builtin_arg_types(name: &str) -> Option<Vec<JitType>> {
    match name {
        "argc" | "clock" => Some(Vec::new()),
        "argv" => Some(vec![JitType::Int {
            bits: 32,
            signed: true,
        }]),
        "parse_int" => Some(vec![JitType::Str]),
        _ => None,
    }
}

fn with_runtime_args<T>(args: &[String], f: impl FnOnce() -> T) -> T {
    JIT_RUNTIME_ARGS.with(|cell| {
        let mut state = RuntimeArgs::default();
        state.storage = args
            .iter()
            .map(|arg| {
                CString::new(arg.as_bytes())
                    .unwrap_or_else(|_| CString::new(arg.replace('\0', "")).expect("valid arg"))
            })
            .collect();
        state.argv = state.storage.iter().map(|value| value.as_ptr()).collect();
        let previous = cell.replace(state);
        let result = f();
        let _ = cell.replace(previous);
        result
    })
}

extern "C" fn rt_print_i64(value: i64) {
    print!("{value}");
}

extern "C" fn rt_print_u64(value: u64) {
    print!("{value}");
}

extern "C" fn rt_print_f64(value: f64) {
    print!("{value:.17}");
}

extern "C" fn rt_print_str(value: *const c_char) {
    if value.is_null() {
        print!("null");
    } else {
        let rendered = unsafe { CStr::from_ptr(value) }.to_string_lossy();
        print!("{rendered}");
    }
}

extern "C" fn rt_print_bool(value: u8) {
    print!("{}", if value == 0 { "false" } else { "true" });
}

extern "C" fn rt_print_newline() {
    println!();
}

extern "C" fn rt_argc() -> i32 {
    JIT_RUNTIME_ARGS.with(|cell| cell.borrow().argv.len() as i32)
}

extern "C" fn rt_argv(index: i32) -> *const c_char {
    JIT_RUNTIME_ARGS.with(|cell| {
        let state = cell.borrow();
        if index < 0 {
            return std::ptr::null();
        }
        state
            .argv
            .get(index as usize)
            .copied()
            .unwrap_or(std::ptr::null())
    })
}

extern "C" fn rt_parse_int(value: *const c_char) -> i32 {
    if value.is_null() {
        return 0;
    }
    unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .trim()
        .parse::<i32>()
        .unwrap_or(0)
}

extern "C" fn rt_clock() -> f64 {
    START_TIME.get_or_init(Instant::now).elapsed().as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;
    use agam_mir::lower::MirLowering;

    fn run_source(source: &str, args: &[&str]) -> i32 {
        let source_id = SourceId(0);
        let mut lexer = Lexer::new(source, source_id);
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            let is_eof = token.kind == agam_lexer::TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        let mut parser = agam_parser::Parser::new(tokens);
        let module = parser.parse_module(source_id).expect("parse failed");

        let mut hir = HirLowering::new();
        let hir = hir.lower_module(&module);

        let mut mir = MirLowering::new();
        let mut mir = mir.lower_module(&hir);
        let _ = agam_mir::opt::optimize_module(&mut mir);

        let runtime_args: Vec<String> = std::iter::once("jit-test".to_string())
            .chain(args.iter().map(|arg| (*arg).to_string()))
            .collect();
        run_main(&mir, &runtime_args).expect("jit run failed")
    }

    #[test]
    fn test_jit_returns_main_result() {
        assert_eq!(run_source("fn main(): return 42", &[]), 42);
    }

    #[test]
    fn test_jit_handles_loops_with_mutable_by_default_lets() {
        let source = r#"
fn main() -> i32:
    let sum: i32 = 0
    let i: i32 = 0
    while i < 5:
        sum = sum + i
        i = i + 1
    return sum
"#;
        assert_eq!(run_source(source, &[]), 10);
    }

    #[test]
    fn test_jit_handles_function_calls() {
        let source = r#"
fn add(x: i32) -> i32:
    return x + 1

fn main() -> i32:
    return add(41)
"#;
        assert_eq!(run_source(source, &[]), 42);
    }

    #[test]
    fn test_jit_runtime_args_and_parse_int() {
        let source = r#"
fn main() -> i32:
    return parse_int(argv(1)) + argc()
"#;
        assert_eq!(run_source(source, &["41"]), 43);
    }

    #[test]
    fn test_jit_handles_i64_loops_with_default_int_literals() {
        let source = r#"
fn main() -> i32:
    let n: i64 = 10
    let total: i64 = 0
    let i: i64 = 0
    while i < n:
        total = total + 1
        i = i + 1
    if total == 10:
        return 0
    return 1
"#;
        assert_eq!(run_source(source, &[]), 0);
    }

    #[test]
    fn test_jit_handles_sum_loop_benchmark_shape() {
        let source = r#"
fn sum_loop(n: i64) -> i64:
    let total: i64 = 0
    let state: i64 = (n % 7919) + 1
    let i: i64 = 0
    while i < n:
        state = (state * 57 + i * 13 + 17) % 1000003
        total = total + (state % 1024)
        i = i + 1
    return total

fn main() -> i32:
    if sum_loop(10) == 5710:
        return 0
    return 1
"#;
        assert_eq!(run_source(source, &[]), 0);
    }
}

