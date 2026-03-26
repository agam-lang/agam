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
    AbiParam, InstBuilder, StackSlotData, StackSlotKind, Type as ClifType, UserFuncName, Value,
    types,
};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::Variable;
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JitOptions {
    pub call_cache: bool,
    pub call_cache_only: Vec<String>,
    pub call_cache_exclude: Vec<String>,
    pub call_cache_optimize: bool,
    pub call_cache_optimize_only: Vec<String>,
    pub call_cache_capacity: usize,
    pub call_cache_warmup: u64,
}

impl Default for JitOptions {
    fn default() -> Self {
        Self {
            call_cache: false,
            call_cache_only: Vec::new(),
            call_cache_exclude: Vec::new(),
            call_cache_optimize: false,
            call_cache_optimize_only: Vec::new(),
            call_cache_capacity: DEFAULT_CALL_CACHE_CAPACITY,
            call_cache_warmup: DEFAULT_CALL_CACHE_WARMUP,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct JitCallCacheStats {
    pub enabled: bool,
    pub total_calls: u64,
    pub total_hits: u64,
    pub total_stores: u64,
    pub functions: Vec<JitFunctionCallCacheStats>,
}

#[derive(Clone, Debug, Default)]
pub struct JitFunctionCallCacheStats {
    pub name: String,
    pub calls: u64,
    pub hits: u64,
    pub stores: u64,
    pub entries: usize,
}

#[derive(Clone, Default)]
struct CallCachePlan {
    function_slots: HashMap<String, i32>,
    function_names: Vec<String>,
    function_modes: Vec<CallCacheMode>,
}

impl CallCachePlan {
    fn slot_for(&self, name: &str) -> Option<i32> {
        self.function_slots.get(name).copied()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CallCacheMode {
    Basic,
    Optimize,
}

#[derive(Default)]
struct CallCacheThreadState {
    active: Option<CallCacheRuntime>,
    last_run: Option<JitCallCacheStats>,
}

struct CallCacheRuntime {
    warmup: u64,
    capacity: usize,
    optimized_entries: usize,
    tick: u64,
    functions: Vec<CallCacheFunctionState>,
}

struct CallCacheFunctionState {
    name: String,
    mode: CallCacheMode,
    calls: u64,
    hits: u64,
    stores: u64,
    entries: HashMap<CallCacheKey, CallCacheEntry>,
    pending_candidates: [Option<PendingCallCacheCandidate>; MAX_PENDING_CALL_CACHE_CANDIDATES],
}

#[derive(Clone, Copy)]
struct CallCacheEntry {
    value: u64,
    hits: u32,
    last_touch: u64,
}

#[derive(Clone, Copy)]
struct PendingCallCacheCandidate {
    key: CallCacheKey,
    hits: u16,
    last_touch: u64,
}

impl CallCacheFunctionState {
    fn remove_pending_candidate(&mut self, key: &CallCacheKey) {
        for slot in &mut self.pending_candidates {
            let matches = slot
                .as_ref()
                .map(|candidate| candidate.key == *key)
                .unwrap_or(false);
            if matches {
                *slot = None;
                return;
            }
        }
    }

    fn record_pending_candidate(&mut self, key: CallCacheKey, tick: u64) -> u32 {
        for slot in &mut self.pending_candidates {
            if let Some(candidate) = slot.as_mut() {
                if candidate.key == key {
                    candidate.hits = candidate.hits.saturating_add(1);
                    candidate.last_touch = tick;
                    return candidate.hits as u32;
                }
            }
        }

        if let Some(slot) = self.pending_candidates.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(PendingCallCacheCandidate {
                key,
                hits: 1,
                last_touch: tick,
            });
            return 1;
        }

        let Some(victim_index) = self.pending_candidate_victim_index() else {
            return 1;
        };
        let victim = self.pending_candidates[victim_index]
            .as_mut()
            .expect("pending candidate victim must exist");
        if victim.hits <= OPTIMIZED_CALL_CACHE_MIN_REPEATS {
            *victim = PendingCallCacheCandidate {
                key,
                hits: 1,
                last_touch: tick,
            };
            return 1;
        }

        victim.hits -= 1;
        victim.last_touch = tick;
        1
    }

    fn pending_candidate_victim_index(&self) -> Option<usize> {
        self.pending_candidates
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.map(|candidate| (index, (candidate.hits, candidate.last_touch)))
            })
            .min_by_key(|(_, score)| *score)
            .map(|(index, _)| index)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CallCacheKey {
    len: u8,
    args: [u64; MAX_CALL_CACHE_ARGS],
}

thread_local! {
    static JIT_RUNTIME_ARGS: RefCell<RuntimeArgs> = RefCell::new(RuntimeArgs::default());
    static JIT_CALL_CACHE: RefCell<CallCacheThreadState> = RefCell::new(CallCacheThreadState::default());
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
const RT_MEMO_LOOKUP: &str = "__agam_jit_memo_lookup";
const RT_MEMO_STORE: &str = "__agam_jit_memo_store";
const MAX_CALL_CACHE_ARGS: usize = 4;
const DEFAULT_CALL_CACHE_CAPACITY: usize = 256;
const DEFAULT_CALL_CACHE_WARMUP: u64 = 32;
const OPTIMIZED_CALL_CACHE_MIN_REPEATS: u16 = 2;
const MAX_PENDING_CALL_CACHE_CANDIDATES: usize = 8;

pub fn run_main(module: &MirModule, args: &[String]) -> Result<i32, String> {
    run_main_with_options(module, args, JitOptions::default())
}

pub fn run_main_with_options(
    module: &MirModule,
    args: &[String],
    options: JitOptions,
) -> Result<i32, String> {
    let layouts = analyze_module(module);
    let call_cache_plan = analyze_call_cache_plan(module, &layouts, &options);
    let main_layout = layouts
        .get("main")
        .cloned()
        .ok_or_else(|| "missing `main` function for JIT execution".to_string())?;

    if !main_layout.params.is_empty() {
        return Err("`agamc run --backend jit` currently requires `main` without parameters; use `argc()` / `argv()` inside Agam instead".into());
    }

    let mut flag_builder = settings::builder();
    flag_builder
        .set("opt_level", "speed")
        .map_err(|e| format!("failed to set Cranelift opt level: {e}"))?;
    flag_builder
        .set("use_colocated_libcalls", "false")
        .map_err(|e| format!("failed to configure Cranelift libcalls: {e}"))?;
    flag_builder
        .set("is_pic", "false")
        .map_err(|e| format!("failed to configure Cranelift PIC mode: {e}"))?;
    let isa_builder =
        cranelift_native::builder().map_err(|e| format!("unsupported host ISA for JIT: {e}"))?;
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .map_err(|e| format!("failed to build Cranelift host ISA: {e}"))?;
    let mut builder = JITBuilder::with_isa(isa, default_libcall_names());
    register_runtime_symbols(&mut builder);

    let mut jit = AgamJit {
        module: JITModule::new(builder),
        layouts,
        func_ids: HashMap::new(),
        imported_funcs: HashMap::new(),
        string_data: HashMap::new(),
        next_string_id: 0,
        call_cache_plan,
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

    with_runtime_args(args, || {
        with_call_cache(&jit.call_cache_plan, &options, || unsafe {
            call_main(main_ptr, main_layout)
        })
    })
}

struct AgamJit {
    module: JITModule,
    layouts: HashMap<String, FunctionLayout>,
    func_ids: HashMap<String, FuncId>,
    imported_funcs: HashMap<String, FuncId>,
    string_data: HashMap<String, DataId>,
    next_string_id: usize,
    call_cache_plan: CallCachePlan,
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
            let predecessor_totals = predecessor_counts(func);
            let mut seen_predecessors: HashMap<BlockId, usize> = HashMap::new();

            let entry_block = *blocks
                .get(&func.entry)
                .ok_or_else(|| format!("missing entry block for `{}`", func.name))?;
            builder.switch_to_block(entry_block);
            builder.append_block_params_for_function_params(entry_block);
            builder.seal_block(entry_block);

            let param_block_values = builder.block_params(entry_block).to_vec();
            let mut local_vars = HashMap::new();
            for (name, ty) in &layout.local_types {
                let var = builder.declare_var(ty.clif_type(pointer_type));
                local_vars.insert(name.clone(), var);
            }

            let param_names: HashSet<&str> = func
                .params
                .iter()
                .map(|param| param.name.as_str())
                .collect();
            for (index, param) in func.params.iter().enumerate() {
                let var = *local_vars.get(&param.name).ok_or_else(|| {
                    format!("missing local variable for parameter `{}`", param.name)
                })?;
                let value = *param_block_values.get(index).ok_or_else(|| {
                    format!(
                        "missing entry block parameter {} while compiling `{}`",
                        index, func.name
                    )
                })?;
                builder.def_var(var, value);
            }
            for (name, ty) in &layout.local_types {
                if param_names.contains(name.as_str()) {
                    continue;
                }
                let var = *local_vars
                    .get(name)
                    .ok_or_else(|| format!("missing local variable for `{}`", name))?;
                let zero = default_value(&mut builder, *ty, pointer_type);
                builder.def_var(var, zero);
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
                        &local_vars,
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
                    &mut seen_predecessors,
                    &predecessor_totals,
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
        local_vars: &HashMap<String, Variable>,
        values: &HashMap<ValueId, Value>,
        instr: &Instruction,
        mem_flags: cranelift_codegen::ir::MemFlags,
    ) -> Result<Value, String> {
        let pointer_type = self.module.target_config().pointer_type();
        let result_ty = value_type(layout, instr.result);
        match &instr.op {
            Op::ConstInt(value) => Ok(builder
                .ins()
                .iconst(result_ty.clif_type(pointer_type), *value)),
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
                let var = *local_vars
                    .get(name)
                    .ok_or_else(|| format!("unknown local `{name}` in JIT load"))?;
                let local_ty = layout.local_types.get(name).copied().unwrap_or(result_ty);
                let value = builder.use_var(var);
                self.coerce_value(builder, value, local_ty, result_ty, mem_flags)
            }
            Op::StoreLocal { name, value } => {
                let var = *local_vars
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
                builder.def_var(var, value);
                Ok(value)
            }
            Op::Alloca { name, ty } => {
                let var = *local_vars
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
                builder.def_var(var, zero);
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
                    builder, *op, left_val, right_val, operand_ty, operand_ty, result_ty,
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
                    let user_param_tys =
                        self.layouts.get(callee).map(|layout| layout.params.clone());
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
                    if self.should_emit_cached_call(callee, args.len(), result_ty) {
                        self.emit_cached_call(
                            builder,
                            callee,
                            func_ref,
                            &lowered_args,
                            user_param_tys.as_deref().ok_or_else(|| {
                                format!("missing JIT parameter layout for cached call `{callee}`")
                            })?,
                            result_ty,
                            mem_flags,
                        )
                    } else {
                        let call = builder.ins().call(func_ref, &lowered_args);
                        let results = builder.inst_results(call);
                        if results.is_empty() {
                            Ok(default_value(builder, result_ty, pointer_type))
                        } else {
                            Ok(results[0])
                        }
                    }
                }
            }
            Op::GetField { object, .. } => lookup_value(values, *object),
            Op::GetIndex { object, .. } => lookup_value(values, *object),
            Op::Phi(_) => {
                Err("MIR phi nodes are not yet supported by the Cranelift JIT slice".into())
            }
            Op::Cast {
                value: value_id,
                target_ty,
            } => {
                let value = lookup_value(values, *value_id)?;
                let source_ty = value_type(layout, *value_id);
                let target_ty = infer_jit_type_from_type_id(*target_ty).unwrap_or(result_ty);
                emit_cast(
                    builder,
                    value,
                    source_ty,
                    target_ty,
                    mem_flags,
                    pointer_type,
                )
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
        seen_predecessors: &mut HashMap<BlockId, usize>,
        predecessor_totals: &HashMap<BlockId, usize>,
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
                note_predecessor(
                    builder,
                    *block,
                    target,
                    seen_predecessors,
                    predecessor_totals,
                );
            }
            Terminator::Branch {
                condition,
                then_block,
                else_block,
            } => {
                let then_block_id = *then_block;
                let else_block_id = *else_block;
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
                builder
                    .ins()
                    .brif(condition, then_block, &[], else_block, &[]);
                note_predecessor(
                    builder,
                    then_block_id,
                    then_block,
                    seen_predecessors,
                    predecessor_totals,
                );
                note_predecessor(
                    builder,
                    else_block_id,
                    else_block,
                    seen_predecessors,
                    predecessor_totals,
                );
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
                    let func =
                        self.runtime_func_id(symbol, &[JitType::Int { bits: 64, signed }], None)?;
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

    fn should_emit_cached_call(&self, callee: &str, arg_count: usize, result_ty: JitType) -> bool {
        self.call_cache_plan.slot_for(callee).is_some()
            && arg_count <= MAX_CALL_CACHE_ARGS
            && supports_call_cache_type(result_ty)
    }

    fn emit_cached_call(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        callee: &str,
        func_ref: cranelift_codegen::ir::FuncRef,
        lowered_args: &[Value],
        param_tys: &[JitType],
        result_ty: JitType,
        mem_flags: cranelift_codegen::ir::MemFlags,
    ) -> Result<Value, String> {
        let pointer_type = self.module.target_config().pointer_type();
        let slot = self
            .call_cache_plan
            .slot_for(callee)
            .ok_or_else(|| format!("missing JIT call-cache slot for `{callee}`"))?;
        let lookup = self.runtime_func_id(
            RT_MEMO_LOOKUP,
            &[
                JitType::Int {
                    bits: 32,
                    signed: true,
                },
                JitType::OpaquePtr,
                JitType::Int {
                    bits: 32,
                    signed: true,
                },
                JitType::OpaquePtr,
            ],
            Some(JitType::Bool),
        )?;
        let store = self.runtime_func_id(
            RT_MEMO_STORE,
            &[
                JitType::Int {
                    bits: 32,
                    signed: true,
                },
                JitType::OpaquePtr,
                JitType::Int {
                    bits: 32,
                    signed: true,
                },
                JitType::Int {
                    bits: 64,
                    signed: false,
                },
            ],
            None,
        )?;
        let lookup_ref = self.module.declare_func_in_func(lookup, builder.func);
        let store_ref = self.module.declare_func_in_func(store, builder.func);

        let args_ptr = if lowered_args.is_empty() {
            builder.ins().iconst(pointer_type, 0)
        } else {
            let args_slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (lowered_args.len() * 8) as u32,
                3,
            ));
            for (index, (arg, ty)) in lowered_args.iter().zip(param_tys.iter()).enumerate() {
                let bits = value_to_call_cache_bits(builder, *arg, *ty, mem_flags)?;
                builder
                    .ins()
                    .stack_store(bits, args_slot, (index * 8) as i32);
            }
            builder.ins().stack_addr(pointer_type, args_slot, 0)
        };
        let out_slot =
            builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
        let out_ptr = builder.ins().stack_addr(pointer_type, out_slot, 0);
        let slot_value = builder.ins().iconst(types::I32, i64::from(slot));
        let arg_count = builder.ins().iconst(types::I32, lowered_args.len() as i64);
        let hit = builder
            .ins()
            .call(lookup_ref, &[slot_value, args_ptr, arg_count, out_ptr]);
        let hit = builder.inst_results(hit)[0];

        let hit_block = builder.create_block();
        let miss_block = builder.create_block();
        let cont_block = builder.create_block();
        builder.append_block_param(cont_block, result_ty.clif_type(pointer_type));

        builder.ins().brif(hit, hit_block, &[], miss_block, &[]);
        builder.seal_block(hit_block);
        builder.seal_block(miss_block);

        builder.switch_to_block(hit_block);
        let cached_bits = builder.ins().stack_load(types::I64, out_slot, 0);
        let cached_value = call_cache_bits_to_value(builder, cached_bits, result_ty, mem_flags)?;
        let cached_args = [cranelift_codegen::ir::BlockArg::Value(cached_value)];
        builder.ins().jump(cont_block, &cached_args);

        builder.switch_to_block(miss_block);
        let call = builder.ins().call(func_ref, lowered_args);
        let miss_value = *builder
            .inst_results(call)
            .first()
            .ok_or_else(|| format!("cached call `{callee}` unexpectedly returned no value"))?;
        let result_bits = value_to_call_cache_bits(builder, miss_value, result_ty, mem_flags)?;
        builder
            .ins()
            .call(store_ref, &[slot_value, args_ptr, arg_count, result_bits]);
        let miss_args = [cranelift_codegen::ir::BlockArg::Value(miss_value)];
        builder.ins().jump(cont_block, &miss_args);

        builder.switch_to_block(cont_block);
        builder.seal_block(cont_block);
        Ok(builder.block_params(cont_block)[0])
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
        emit_cast(
            builder,
            value,
            source_ty,
            target_ty,
            mem_flags,
            pointer_type,
        )
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
    builder.symbol(RT_MEMO_LOOKUP, rt_memo_lookup as *const u8);
    builder.symbol(RT_MEMO_STORE, rt_memo_store as *const u8);
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
        JitType::Int {
            bits: 8,
            signed: true,
        } => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> i8>(main_ptr) };
            i32::from(func())
        }
        JitType::Int {
            bits: 8,
            signed: false,
        } => {
            let func = unsafe { mem::transmute::<_, extern "C" fn() -> u8>(main_ptr) };
            i32::from(func())
        }
        JitType::Int {
            bits: 16,
            signed: true,
        } => {
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
        JitType::Int {
            bits: 32,
            signed: true,
        } => {
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
        JitType::Int {
            bits: 64,
            signed: true,
        } => {
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

fn analyze_call_cache_plan(
    module: &MirModule,
    layouts: &HashMap<String, FunctionLayout>,
    options: &JitOptions,
) -> CallCachePlan {
    if !options.call_cache
        && !options.call_cache_optimize
        && options.call_cache_only.is_empty()
        && options.call_cache_optimize_only.is_empty()
    {
        return CallCachePlan::default();
    }

    let mut include_only: HashSet<&str> = options
        .call_cache_only
        .iter()
        .map(String::as_str)
        .collect();
    include_only.extend(options.call_cache_optimize_only.iter().map(String::as_str));
    let exclude: HashSet<&str> = options
        .call_cache_exclude
        .iter()
        .map(String::as_str)
        .collect();
    let mut candidates: HashSet<String> = module
        .functions
        .iter()
        .filter(|func| {
            func.name != "main"
                && !exclude.contains(func.name.as_str())
                && if options.call_cache || options.call_cache_optimize {
                    true
                } else {
                    include_only.contains(func.name.as_str())
                }
                && layouts
                    .get(&func.name)
                    .map(function_layout_supports_call_cache)
                    .unwrap_or(false)
        })
        .map(|func| func.name.clone())
        .collect();

    loop {
        let to_remove: Vec<String> = module
            .functions
            .iter()
            .filter(|func| candidates.contains(&func.name))
            .filter(|func| function_has_impure_or_uncacheable_calls(func, &candidates))
            .map(|func| func.name.clone())
            .collect();
        if to_remove.is_empty() {
            break;
        }
        for name in to_remove {
            candidates.remove(&name);
        }
    }

    let function_names: Vec<String> = module
        .functions
        .iter()
        .filter(|func| candidates.contains(&func.name))
        .map(|func| func.name.clone())
        .collect();
    let optimize_only: HashSet<&str> = options
        .call_cache_optimize_only
        .iter()
        .map(String::as_str)
        .collect();
    let function_modes = function_names
        .iter()
        .map(|name| {
            if options.call_cache_optimize || optimize_only.contains(name.as_str()) {
                CallCacheMode::Optimize
            } else {
                CallCacheMode::Basic
            }
        })
        .collect();
    let function_slots = function_names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index as i32))
        .collect();

    CallCachePlan {
        function_slots,
        function_names,
        function_modes,
    }
}

fn function_layout_supports_call_cache(layout: &FunctionLayout) -> bool {
    !matches!(layout.return_ty, JitType::Unit)
        && layout.params.len() <= MAX_CALL_CACHE_ARGS
        && supports_call_cache_type(layout.return_ty)
        && layout.params.iter().copied().all(supports_call_cache_type)
}

fn function_has_impure_or_uncacheable_calls(
    function: &MirFunction,
    cacheable_functions: &HashSet<String>,
) -> bool {
    function.blocks.iter().any(|block| {
        block.instructions.iter().any(|instr| match &instr.op {
            Op::Call { callee, .. } => {
                is_print_builtin(callee)
                    || builtin_signature(callee).is_some()
                    || !cacheable_functions.contains(callee)
            }
            _ => false,
        })
    })
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
                Op::ConstFloat(_) => {
                    infer_jit_type_from_type_id(instr.ty).unwrap_or(JitType::Float64)
                }
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
                Op::GetField { object, .. } | Op::GetIndex { object, .. } => {
                    value_type(&layout, *object)
                }
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

fn supports_call_cache_type(ty: JitType) -> bool {
    match ty {
        JitType::Int { bits, .. } => bits <= 64,
        JitType::Float32 | JitType::Float64 | JitType::Bool => true,
        JitType::Str | JitType::OpaquePtr | JitType::Unit => false,
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
            } else if result_ty
                .int_spec()
                .map(|(_, signed)| signed)
                .unwrap_or(true)
            {
                builder.ins().sdiv(left, right)
            } else {
                builder.ins().udiv(left, right)
            }
        }
        MirBinOp::Mod => {
            if result_ty.is_float() {
                return Err(
                    "floating-point modulo is not yet supported by the Cranelift JIT slice".into(),
                );
            } else if result_ty
                .int_spec()
                .map(|(_, signed)| signed)
                .unwrap_or(true)
            {
                builder.ins().srem(left, right)
            } else {
                builder.ins().urem(left, right)
            }
        }
        MirBinOp::Eq
        | MirBinOp::NotEq
        | MirBinOp::Lt
        | MirBinOp::LtEq
        | MirBinOp::Gt
        | MirBinOp::GtEq => {
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
                        if left_ty
                            .int_spec()
                            .map(|(_, signed)| signed)
                            .unwrap_or(false)
                        {
                            IntCC::SignedLessThan
                        } else {
                            IntCC::UnsignedLessThan
                        }
                    }
                    MirBinOp::LtEq => {
                        if left_ty
                            .int_spec()
                            .map(|(_, signed)| signed)
                            .unwrap_or(false)
                        {
                            IntCC::SignedLessThanOrEqual
                        } else {
                            IntCC::UnsignedLessThanOrEqual
                        }
                    }
                    MirBinOp::Gt => {
                        if left_ty
                            .int_spec()
                            .map(|(_, signed)| signed)
                            .unwrap_or(false)
                        {
                            IntCC::SignedGreaterThan
                        } else {
                            IntCC::UnsignedGreaterThan
                        }
                    }
                    MirBinOp::GtEq => {
                        if left_ty
                            .int_spec()
                            .map(|(_, signed)| signed)
                            .unwrap_or(false)
                        {
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
            if result_ty
                .int_spec()
                .map(|(_, signed)| signed)
                .unwrap_or(false)
            {
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
        (source, target)
            if source.is_float() && matches!(target, JitType::Int { .. } | JitType::Bool) =>
        {
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
        | (JitType::Bool, JitType::OpaquePtr) => {
            builder
                .ins()
                .bitcast(target_ty.clif_type(pointer_type), mem_flags, value)
        }
        _ => {
            return Err(format!(
                "unsupported JIT cast from {source_ty:?} to {target_ty:?}"
            ));
        }
    })
}

fn value_to_call_cache_bits(
    builder: &mut FunctionBuilder<'_>,
    value: Value,
    ty: JitType,
    mem_flags: cranelift_codegen::ir::MemFlags,
) -> Result<Value, String> {
    Ok(match ty {
        JitType::Int { signed, .. } => normalize_int(builder, value, ty, 64, signed),
        JitType::Bool => normalize_int(builder, value, JitType::Bool, 64, false),
        JitType::Float64 => builder.ins().bitcast(types::I64, mem_flags, value),
        JitType::Float32 => {
            let raw = builder.ins().bitcast(types::I32, mem_flags, value);
            builder.ins().uextend(types::I64, raw)
        }
        _ => return Err(format!("unsupported JIT call-cache value type {ty:?}")),
    })
}

fn call_cache_bits_to_value(
    builder: &mut FunctionBuilder<'_>,
    bits: Value,
    ty: JitType,
    mem_flags: cranelift_codegen::ir::MemFlags,
) -> Result<Value, String> {
    Ok(match ty {
        JitType::Int {
            bits: target_bits,
            signed,
        } => normalize_int(
            builder,
            bits,
            JitType::Int { bits: 64, signed },
            target_bits,
            signed,
        ),
        JitType::Bool => normalize_int(
            builder,
            bits,
            JitType::Int {
                bits: 64,
                signed: false,
            },
            8,
            false,
        ),
        JitType::Float64 => builder.ins().bitcast(types::F64, mem_flags, bits),
        JitType::Float32 => {
            let raw = builder.ins().ireduce(types::I32, bits);
            builder.ins().bitcast(types::F32, mem_flags, raw)
        }
        _ => return Err(format!("unsupported JIT call-cache value type {ty:?}")),
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

fn predecessor_counts(function: &MirFunction) -> HashMap<BlockId, usize> {
    let mut counts = HashMap::new();
    for block in &function.blocks {
        match &block.terminator {
            Terminator::Jump(target) => {
                *counts.entry(*target).or_insert(0) += 1;
            }
            Terminator::Branch {
                then_block,
                else_block,
                ..
            } => {
                *counts.entry(*then_block).or_insert(0) += 1;
                *counts.entry(*else_block).or_insert(0) += 1;
            }
            Terminator::Return(_) | Terminator::ReturnVoid | Terminator::Unreachable => {}
        }
    }
    counts
}

fn note_predecessor(
    builder: &mut FunctionBuilder<'_>,
    block_id: BlockId,
    block: cranelift_codegen::ir::Block,
    seen_predecessors: &mut HashMap<BlockId, usize>,
    predecessor_totals: &HashMap<BlockId, usize>,
) {
    let seen = seen_predecessors.entry(block_id).or_insert(0);
    *seen += 1;
    if predecessor_totals.get(&block_id).copied() == Some(*seen) {
        builder.seal_block(block);
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

fn with_call_cache<T>(plan: &CallCachePlan, options: &JitOptions, f: impl FnOnce() -> T) -> T {
    if !options.call_cache
        && !options.call_cache_optimize
        && options.call_cache_only.is_empty()
        && options.call_cache_optimize_only.is_empty()
    {
        JIT_CALL_CACHE.with(|cell| {
            let mut state = cell.borrow_mut();
            state.active = None;
            state.last_run = None;
        });
        return f();
    }

    JIT_CALL_CACHE.with(|cell| {
        let mut state = cell.borrow_mut();
        state.active = Some(CallCacheRuntime::new(
            &plan.function_names,
            &plan.function_modes,
            options.call_cache_capacity,
            options.call_cache_warmup,
        ));
        state.last_run = None;
    });

    let result = f();

    JIT_CALL_CACHE.with(|cell| {
        let mut state = cell.borrow_mut();
        state.last_run = state.active.take().map(CallCacheRuntime::into_stats);
    });

    result
}

pub fn take_last_call_cache_stats() -> Option<JitCallCacheStats> {
    JIT_CALL_CACHE.with(|cell| cell.borrow_mut().last_run.take())
}

impl CallCacheRuntime {
    fn new(
        function_names: &[String],
        function_modes: &[CallCacheMode],
        capacity: usize,
        warmup: u64,
    ) -> Self {
        Self {
            warmup,
            capacity,
            optimized_entries: 0,
            tick: 0,
            functions: function_names
                .iter()
                .enumerate()
                .map(|(index, name)| CallCacheFunctionState {
                    name: name.clone(),
                    mode: function_modes
                        .get(index)
                        .copied()
                        .unwrap_or(CallCacheMode::Basic),
                    calls: 0,
                    hits: 0,
                    stores: 0,
                    entries: HashMap::new(),
                    pending_candidates: [None; MAX_PENDING_CALL_CACHE_CANDIDATES],
                })
                .collect(),
        }
    }

    fn lookup(&mut self, slot: i32, key: CallCacheKey) -> Option<u64> {
        let function = self.functions.get_mut(slot as usize)?;
        function.calls += 1;
        if function.calls <= self.warmup {
            return None;
        }
        self.tick = self.tick.saturating_add(1);
        let value = {
            let entry = function.entries.get_mut(&key)?;
            entry.hits = entry.hits.saturating_add(1);
            entry.last_touch = self.tick;
            entry.value
        };
        function.hits += 1;
        function.remove_pending_candidate(&key);
        Some(value)
    }

    fn store(&mut self, slot: i32, key: CallCacheKey, value: u64) {
        let function_index = slot as usize;
        let Some(mode) = self.functions.get(function_index).map(|function| function.mode) else {
            return;
        };
        if self
            .functions
            .get(function_index)
            .map(|function| function.calls <= self.warmup)
            .unwrap_or(true)
        {
            return;
        }
        match mode {
            CallCacheMode::Basic => {
                let function = &mut self.functions[function_index];
                if !function.entries.contains_key(&key) && function.entries.len() >= self.capacity {
                    return;
                }
                function.entries.insert(
                    key,
                    CallCacheEntry {
                        value,
                        hits: 1,
                        last_touch: self.tick,
                    },
                );
                function.stores += 1;
            }
            CallCacheMode::Optimize => {
                let candidate_hits = {
                    let function = &mut self.functions[function_index];
                    function.record_pending_candidate(key, self.tick)
                };
                if candidate_hits < OPTIMIZED_CALL_CACHE_MIN_REPEATS as u32 {
                    return;
                }
                {
                    let function = &mut self.functions[function_index];
                    if let Some(entry) = function.entries.get_mut(&key) {
                        entry.value = value;
                        entry.last_touch = self.tick;
                        entry.hits = entry.hits.saturating_add(1);
                        function.remove_pending_candidate(&key);
                        return;
                    }
                }
                if self.optimized_entries < self.capacity {
                    let function = &mut self.functions[function_index];
                    function.entries.insert(
                        key,
                        CallCacheEntry {
                            value,
                            hits: candidate_hits,
                            last_touch: self.tick,
                        },
                    );
                    function.remove_pending_candidate(&key);
                    function.stores += 1;
                    self.optimized_entries += 1;
                    return;
                }

                let Some(victim) = self.global_optimized_victim() else {
                    return;
                };
                let victim_score = self.optimized_entry_score(victim.function_index, &victim.key);
                let candidate_score = (candidate_hits, self.tick);
                if candidate_score <= victim_score {
                    return;
                }
                if let Some(victim_function) = self.functions.get_mut(victim.function_index) {
                    victim_function.entries.remove(&victim.key);
                }
                let function = &mut self.functions[function_index];
                function.entries.insert(
                    key,
                    CallCacheEntry {
                        value,
                        hits: candidate_hits,
                        last_touch: self.tick,
                    },
                );
                function.remove_pending_candidate(&key);
                function.stores += 1;
            }
        }
    }

    fn global_optimized_victim(&self) -> Option<CallCacheVictim> {
        let mut victim: Option<CallCacheVictim> = None;
        for (function_index, function) in self.functions.iter().enumerate() {
            if function.mode != CallCacheMode::Optimize {
                continue;
            }
            for key in function.entries.keys() {
                let score = self.optimized_entry_score(function_index, key);
                let replace = victim
                    .as_ref()
                    .map(|current| score < current.score)
                    .unwrap_or(true);
                if replace {
                    victim = Some(CallCacheVictim {
                        function_index,
                        key: *key,
                        score,
                    });
                }
            }
        }
        victim
    }

    fn optimized_entry_score(&self, function_index: usize, key: &CallCacheKey) -> (u32, u64) {
        self.functions
            .get(function_index)
            .and_then(|function| function.entries.get(key))
            .map(|entry| (entry.hits, entry.last_touch))
            .unwrap_or((0, 0))
    }

    fn into_stats(self) -> JitCallCacheStats {
        let mut stats = JitCallCacheStats {
            enabled: true,
            total_calls: 0,
            total_hits: 0,
            total_stores: 0,
            functions: Vec::with_capacity(self.functions.len()),
        };
        for function in self.functions {
            stats.total_calls += function.calls;
            stats.total_hits += function.hits;
            stats.total_stores += function.stores;
            stats.functions.push(JitFunctionCallCacheStats {
                name: function.name,
                calls: function.calls,
                hits: function.hits,
                stores: function.stores,
                entries: function.entries.len(),
            });
        }
        stats
    }
}

#[derive(Clone, Copy)]
struct CallCacheVictim {
    function_index: usize,
    key: CallCacheKey,
    score: (u32, u64),
}

impl CallCacheKey {
    unsafe fn from_raw_parts(args_ptr: *const u64, arg_count: i32) -> Option<Self> {
        if arg_count < 0 || arg_count as usize > MAX_CALL_CACHE_ARGS {
            return None;
        }
        if arg_count > 0 && args_ptr.is_null() {
            return None;
        }
        let mut args = [0_u64; MAX_CALL_CACHE_ARGS];
        if arg_count > 0 {
            let values = unsafe { std::slice::from_raw_parts(args_ptr, arg_count as usize) };
            args[..arg_count as usize].copy_from_slice(values);
        }
        Some(Self {
            len: arg_count as u8,
            args,
        })
    }
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

extern "C" fn rt_memo_lookup(
    slot: i32,
    args_ptr: *const u64,
    arg_count: i32,
    out_ptr: *mut u64,
) -> u8 {
    let Some(key) = (unsafe { CallCacheKey::from_raw_parts(args_ptr, arg_count) }) else {
        return 0;
    };
    if out_ptr.is_null() {
        return 0;
    }
    JIT_CALL_CACHE.with(|cell| {
        let mut state = cell.borrow_mut();
        let Some(runtime) = state.active.as_mut() else {
            return 0;
        };
        let Some(value) = runtime.lookup(slot, key) else {
            return 0;
        };
        unsafe {
            *out_ptr = value;
        }
        1
    })
}

extern "C" fn rt_memo_store(slot: i32, args_ptr: *const u64, arg_count: i32, result: u64) {
    let Some(key) = (unsafe { CallCacheKey::from_raw_parts(args_ptr, arg_count) }) else {
        return;
    };
    JIT_CALL_CACHE.with(|cell| {
        let mut state = cell.borrow_mut();
        if let Some(runtime) = state.active.as_mut() {
            runtime.store(slot, key, result);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;
    use agam_mir::lower::MirLowering;

    fn run_source(source: &str, args: &[&str]) -> i32 {
        run_source_with_options(source, args, JitOptions::default()).0
    }

    fn run_source_with_options(
        source: &str,
        args: &[&str],
        options: JitOptions,
    ) -> (i32, Option<JitCallCacheStats>) {
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
        let result = run_main_with_options(&mir, &runtime_args, options).expect("jit run failed");
        (result, take_last_call_cache_stats())
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

    #[test]
    fn test_call_cache_hits_repeated_pure_calls() {
        let source = r#"
fn hot(n: i64) -> i64:
    let total: i64 = 0
    let i: i64 = 0
    while i < 2000:
        total = total + ((n * 17) + i) % 97
        i = i + 1
    return total

fn main() -> i32:
    let acc: i64 = 0
    let i: i64 = 0
    while i < 64:
        acc = acc + hot(33)
        i = i + 1
    if acc > 0:
        return 0
    return 1
"#;
        let (result, stats) = run_source_with_options(
            source,
            &[],
            JitOptions {
                call_cache: true,
                call_cache_capacity: 32,
                call_cache_warmup: 2,
                ..Default::default()
            },
        );
        assert_eq!(result, 0);
        let stats = stats.expect("expected call-cache stats");
        let hot = stats
            .functions
            .iter()
            .find(|function| function.name == "hot")
            .expect("missing hot cache stats");
        assert!(hot.hits > 0, "expected repeated cached hits, got {hot:?}");
        assert!(hot.stores > 0, "expected cached stores, got {hot:?}");
    }

    #[test]
    fn test_call_cache_skips_impure_runtime_calls() {
        let source = r#"
fn nowish() -> f64:
    return clock()

fn main() -> i32:
    let a: f64 = nowish()
    let b: f64 = nowish()
    if b >= a:
        return 0
    return 1
"#;
        let (result, stats) = run_source_with_options(
            source,
            &[],
            JitOptions {
                call_cache: true,
                call_cache_capacity: 32,
                call_cache_warmup: 0,
                ..Default::default()
            },
        );
        assert_eq!(result, 0);
        let stats = stats.expect("expected call-cache stats");
        assert!(
            stats
                .functions
                .iter()
                .all(|function| function.name != "nowish"),
            "impure clock-based function should not be cacheable: {stats:?}"
        );
    }

    #[test]
    fn test_call_cache_optimize_mode_works_without_basic_flag() {
        let source = r#"
fn hot(n: i64) -> i64:
    let total: i64 = 0
    let i: i64 = 0
    while i < 256:
        total = total + ((n * 5) + i) % 31
        i = i + 1
    return total

fn main() -> i32:
    let acc: i64 = 0
    let i: i64 = 0
    while i < 32:
        acc = acc + hot(7)
        i = i + 1
    if acc > 0:
        return 0
    return 1
"#;
        let (result, stats) = run_source_with_options(
            source,
            &[],
            JitOptions {
                call_cache_optimize: true,
                call_cache_capacity: 8,
                call_cache_warmup: 0,
                ..Default::default()
            },
        );
        assert_eq!(result, 0);
        let stats = stats.expect("expected call-cache stats");
        let hot = stats
            .functions
            .iter()
            .find(|function| function.name == "hot")
            .expect("missing hot cache stats");
        assert!(hot.hits > 0, "expected optimize mode to cache repeated input: {hot:?}");
        assert!(hot.entries <= 1, "expected optimized cache to keep the hot key small: {hot:?}");
    }

    #[test]
    fn test_call_cache_optimize_pending_candidates_stay_bounded() {
        let mut runtime =
            CallCacheRuntime::new(&["hot".to_string()], &[CallCacheMode::Optimize], 4, 0);
        for value in 0..64u64 {
            let key = CallCacheKey {
                len: 1,
                args: [value, 0, 0, 0],
            };
            runtime.lookup(0, key);
            runtime.store(0, key, value);
        }

        let function = &runtime.functions[0];
        let pending_candidates = function
            .pending_candidates
            .iter()
            .flatten()
            .count();
        assert!(
            pending_candidates <= MAX_PENDING_CALL_CACHE_CANDIDATES,
            "pending candidates grew past the fixed bound: {pending_candidates}"
        );
        assert_eq!(
            function.entries.len(),
            0,
            "unique inputs should not bypass the repeated-input admission policy"
        );
    }
}
