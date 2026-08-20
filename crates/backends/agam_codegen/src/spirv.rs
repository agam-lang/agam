//! Direct SPIR-V Binary Code Generation Backend.
//!
//! Emits vendor-neutral SPIR-V 1.5 compute binaries for Vulkan, OpenCL, and Level Zero
//! from `@gpu`-annotated Agam MIR modules.

use std::collections::HashMap;

use agam_mir::ir::{GpuIntrinsicKind, MirBinOp, MirFunction, MirModule, Op, ValueId};

pub const SPIRV_MAGIC: u32 = 0x07230203;
pub const SPIRV_VERSION_1_5: u32 = 0x00010500;
pub const AGAM_GENERATOR_MAGIC: u32 = 0x001A0000;

// ── SPIR-V Specification Constants ──

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    OpNop = 0,
    OpSource = 3,
    OpName = 5,
    OpCapability = 17,
    OpMemoryModel = 14,
    OpEntryPoint = 15,
    OpExecutionMode = 16,
    OpTypeVoid = 19,
    OpTypeBool = 20,
    OpTypeInt = 21,
    OpTypeFloat = 22,
    OpTypeVector = 23,
    OpTypeStruct = 30,
    OpTypePointer = 32,
    OpTypeFunction = 33,
    OpConstant = 43,
    OpFunction = 54,
    OpFunctionParameter = 55,
    OpFunctionEnd = 56,
    OpVariable = 59,
    OpLoad = 61,
    OpStore = 62,
    OpAccessChain = 65,
    OpDecorate = 71,
    OpIAdd = 128,
    OpFAdd = 129,
    OpISub = 130,
    OpFSub = 131,
    OpIMul = 132,
    OpFMul = 133,
    OpSDiv = 135,
    OpFDiv = 136,
    OpControlBarrier = 224,
    OpLabel = 248,
    OpBranch = 249,
    OpReturn = 253,
    OpReturnValue = 254,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    Shader = 1,
    Matrix = 0,
    Addresses = 4,
    Linkage = 5,
    Kernel = 6,
    Float64 = 7,
    Int64 = 8,
    GroupNonUniform = 61,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingModel {
    Logical = 0,
    Physical32 = 1,
    Physical64 = 2,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryModel {
    Simple = 0,
    GLSL450 = 1,
    OpenCL = 2,
    Vulkan = 3,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionModel {
    GLCompute = 5,
    Kernel = 6,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    LocalSize = 17,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageClass {
    UniformConstant = 0,
    Input = 1,
    Uniform = 2,
    Output = 3,
    Workgroup = 4,
    CrossWorkgroup = 5,
    Private = 6,
    Function = 7,
    StorageBuffer = 12,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decoration {
    Block = 2,
    BufferBlock = 3,
    ArrayStride = 6,
    DescriptorSet = 34,
    Binding = 33,
}

/// A single SPIR-V Instruction.
#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operands: Vec<u32>,
}

impl Instruction {
    pub fn new(opcode: Opcode, operands: Vec<u32>) -> Self {
        Self { opcode, operands }
    }

    /// Encode instruction to SPIR-V 32-bit words.
    pub fn encode(&self, out: &mut Vec<u32>) {
        let word_count = (self.operands.len() + 1) as u32;
        let word0 = (word_count << 16) | (self.opcode as u32);
        out.push(word0);
        out.extend_from_slice(&self.operands);
    }
}

/// SPIR-V Binary Module Builder.
pub struct SpirvModuleBuilder {
    pub next_id: u32,
    pub capabilities: Vec<Instruction>,
    pub memory_model: Vec<Instruction>,
    pub entry_points: Vec<Instruction>,
    pub execution_modes: Vec<Instruction>,
    pub debug_names: Vec<Instruction>,
    pub annotations: Vec<Instruction>,
    pub types_and_constants: Vec<Instruction>,
    pub functions: Vec<Instruction>,
}

impl Default for SpirvModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SpirvModuleBuilder {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            capabilities: Vec::new(),
            memory_model: Vec::new(),
            entry_points: Vec::new(),
            execution_modes: Vec::new(),
            debug_names: Vec::new(),
            annotations: Vec::new(),
            types_and_constants: Vec::new(),
            functions: Vec::new(),
        }
    }

    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn encode_string(s: &str) -> Vec<u32> {
        let bytes = s.as_bytes();
        let mut words = Vec::new();
        let mut current = 0u32;
        let mut shift = 0;
        for &b in bytes {
            current |= (b as u32) << shift;
            shift += 8;
            if shift == 32 {
                words.push(current);
                current = 0;
                shift = 0;
            }
        }
        // Push trailing word with null terminator
        words.push(current);
        words
    }

    pub fn add_capability(&mut self, cap: Capability) {
        self.capabilities
            .push(Instruction::new(Opcode::OpCapability, vec![cap as u32]));
    }

    pub fn set_memory_model(&mut self, addressing: AddressingModel, model: MemoryModel) {
        self.memory_model.push(Instruction::new(
            Opcode::OpMemoryModel,
            vec![addressing as u32, model as u32],
        ));
    }

    pub fn add_debug_name(&mut self, target_id: u32, name: &str) {
        let mut operands = vec![target_id];
        operands.extend(Self::encode_string(name));
        self.debug_names
            .push(Instruction::new(Opcode::OpName, operands));
    }

    pub fn add_decoration(&mut self, target_id: u32, dec: Decoration, extra: &[u32]) {
        let mut operands = vec![target_id, dec as u32];
        operands.extend_from_slice(extra);
        self.annotations
            .push(Instruction::new(Opcode::OpDecorate, operands));
    }

    /// Build the complete binary as an array of 32-bit words.
    pub fn build_words(&self) -> Vec<u32> {
        let mut out = vec![
            SPIRV_MAGIC,
            SPIRV_VERSION_1_5,
            AGAM_GENERATOR_MAGIC,
            self.next_id, // Bound
            0,            // Reserved schema
        ];

        // 2. Sections
        for instr in &self.capabilities {
            instr.encode(&mut out);
        }
        for instr in &self.memory_model {
            instr.encode(&mut out);
        }
        for instr in &self.entry_points {
            instr.encode(&mut out);
        }
        for instr in &self.execution_modes {
            instr.encode(&mut out);
        }
        for instr in &self.debug_names {
            instr.encode(&mut out);
        }
        for instr in &self.annotations {
            instr.encode(&mut out);
        }
        for instr in &self.types_and_constants {
            instr.encode(&mut out);
        }
        for instr in &self.functions {
            instr.encode(&mut out);
        }

        out
    }

    /// Build the complete binary as a byte array (little-endian).
    pub fn build_bytes(&self) -> Vec<u8> {
        let words = self.build_words();
        let mut bytes = Vec::with_capacity(words.len() * 4);
        for w in words {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        bytes
    }
}

/// Emit a SPIR-V 1.5 binary module from an Agam MIR module.
///
/// Returns `None` if the MIR module contains no `@gpu`-annotated functions.
pub fn emit_spirv_module(module: &MirModule) -> Option<Vec<u32>> {
    let gpu_funcs: Vec<&MirFunction> = module
        .functions
        .iter()
        .filter(|f| f.gpu_config.is_some())
        .collect();

    if gpu_funcs.is_empty() {
        return None;
    }

    let mut builder = SpirvModuleBuilder::new();

    // Standard compute capabilities
    builder.add_capability(Capability::Shader);
    builder.add_capability(Capability::Float64);
    builder.add_capability(Capability::Int64);
    builder.add_capability(Capability::GroupNonUniform);

    builder.set_memory_model(AddressingModel::Logical, MemoryModel::GLSL450);

    // Common Types
    let type_void = builder.alloc_id();
    builder
        .types_and_constants
        .push(Instruction::new(Opcode::OpTypeVoid, vec![type_void]));

    let type_bool = builder.alloc_id();
    builder
        .types_and_constants
        .push(Instruction::new(Opcode::OpTypeBool, vec![type_bool]));

    let type_i32 = builder.alloc_id();
    builder
        .types_and_constants
        .push(Instruction::new(Opcode::OpTypeInt, vec![type_i32, 32, 1]));

    let type_u32 = builder.alloc_id();
    builder
        .types_and_constants
        .push(Instruction::new(Opcode::OpTypeInt, vec![type_u32, 32, 0]));

    let type_f32 = builder.alloc_id();
    builder
        .types_and_constants
        .push(Instruction::new(Opcode::OpTypeFloat, vec![type_f32, 32]));

    let type_f64 = builder.alloc_id();
    builder
        .types_and_constants
        .push(Instruction::new(Opcode::OpTypeFloat, vec![type_f64, 64]));

    let type_v3u32 = builder.alloc_id();
    builder.types_and_constants.push(Instruction::new(
        Opcode::OpTypeVector,
        vec![type_v3u32, type_u32, 3],
    ));

    let type_fn_void = builder.alloc_id();
    builder.types_and_constants.push(Instruction::new(
        Opcode::OpTypeFunction,
        vec![type_fn_void, type_void],
    ));

    for func in &gpu_funcs {
        let config = func.gpu_config.as_ref().unwrap();
        let fn_id = builder.alloc_id();
        builder.add_debug_name(fn_id, &func.name);

        // Entry point & execution mode
        let mut ep_operands = vec![ExecutionModel::GLCompute as u32, fn_id];
        ep_operands.extend(SpirvModuleBuilder::encode_string(&func.name));
        builder
            .entry_points
            .push(Instruction::new(Opcode::OpEntryPoint, ep_operands));

        let local_x = config.threads_per_block.max(1);
        let local_y = 1u32;
        let local_z = 1u32;
        builder.execution_modes.push(Instruction::new(
            Opcode::OpExecutionMode,
            vec![
                fn_id,
                ExecutionMode::LocalSize as u32,
                local_x,
                local_y,
                local_z,
            ],
        ));

        // Function Start
        builder.functions.push(Instruction::new(
            Opcode::OpFunction,
            vec![type_void, fn_id, 0, type_fn_void],
        ));

        let entry_label = builder.alloc_id();
        builder
            .functions
            .push(Instruction::new(Opcode::OpLabel, vec![entry_label]));

        // Lower body instructions
        let mut value_map: HashMap<ValueId, u32> = HashMap::new();
        for param in &func.params {
            let p_id = builder.alloc_id();
            value_map.insert(param.value, p_id);
        }

        for block in &func.blocks {
            for instr in &block.instructions {
                let res_id = builder.alloc_id();
                value_map.insert(instr.result, res_id);

                match &instr.op {
                    Op::BinOp { op, left, right } => {
                        let l_id = value_map.get(left).copied().unwrap_or(0);
                        let r_id = value_map.get(right).copied().unwrap_or(0);
                        let spv_op = match op {
                            MirBinOp::Add => Opcode::OpFAdd,
                            MirBinOp::Sub => Opcode::OpFSub,
                            MirBinOp::Mul => Opcode::OpFMul,
                            MirBinOp::Div => Opcode::OpFDiv,
                            _ => Opcode::OpIAdd,
                        };
                        builder
                            .functions
                            .push(Instruction::new(spv_op, vec![type_f32, res_id, l_id, r_id]));
                    }
                    Op::GpuIntrinsic {
                        kind: GpuIntrinsicKind::Barrier,
                        ..
                    } => {
                        // Scope 2 (Workgroup), Scope 2 (Workgroup), Semantics 0x100 (AcquireRelease | WorkgroupMemory)
                        let const_scope = builder.alloc_id();
                        builder.types_and_constants.push(Instruction::new(
                            Opcode::OpConstant,
                            vec![type_u32, const_scope, 2],
                        ));
                        let const_sem = builder.alloc_id();
                        builder.types_and_constants.push(Instruction::new(
                            Opcode::OpConstant,
                            vec![type_u32, const_sem, 256],
                        ));
                        builder.functions.push(Instruction::new(
                            Opcode::OpControlBarrier,
                            vec![const_scope, const_scope, const_sem],
                        ));
                    }
                    _ => {}
                }
            }
        }

        builder
            .functions
            .push(Instruction::new(Opcode::OpReturn, vec![]));
        builder
            .functions
            .push(Instruction::new(Opcode::OpFunctionEnd, vec![]));
    }

    Some(builder.build_words())
}

/// Emit raw SPIR-V binary bytes from an Agam MIR module.
pub fn emit_spirv_binary(module: &MirModule) -> Option<Vec<u8>> {
    let words = emit_spirv_module(module)?;
    let mut bytes = Vec::with_capacity(words.len() * 4);
    for w in words {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_mir::ir::{BasicBlock, BlockId, Instruction as MirInstruction, MirParam, Terminator};
    use agam_sema::gpu::{GpuKernelConfig, GpuKernelParamAbi};
    use agam_sema::symbol::TypeId;
    use agam_sema::target::TargetProfile;

    fn make_test_gpu_func(name: &str) -> MirFunction {
        let block = BasicBlock {
            id: BlockId(0),
            instructions: vec![MirInstruction {
                result: ValueId(1),
                ty: TypeId(0),
                op: Op::GpuIntrinsic {
                    kind: GpuIntrinsicKind::Barrier,
                    args: vec![],
                },
            }],
            terminator: Terminator::Return(ValueId(0)),
        };

        MirFunction {
            name: name.into(),
            generics: vec![],
            params: vec![MirParam {
                name: "a".into(),
                ty: TypeId(0),
                value: ValueId(0),
                memory_type: None,
                gpu_abi: GpuKernelParamAbi::PtrF32,
            }],
            return_ty: TypeId(0),
            blocks: vec![block],
            entry: BlockId(0),
            target: TargetProfile::Default,
            gpu_config: Some(GpuKernelConfig::default()),
        }
    }

    #[test]
    fn test_spirv_binary_header_magic_and_version() {
        let module = MirModule {
            functions: vec![make_test_gpu_func("vector_kernel")],
            enum_layouts: HashMap::new(),
            struct_layouts: HashMap::new(),
        };

        let words = emit_spirv_module(&module).expect("SPIR-V module emission");
        assert!(words.len() >= 5);
        assert_eq!(words[0], SPIRV_MAGIC);
        assert_eq!(words[1], SPIRV_VERSION_1_5);
        assert_eq!(words[2], AGAM_GENERATOR_MAGIC);
        assert!(words[3] > 0); // Bound ID
        assert_eq!(words[4], 0); // Schema
    }

    #[test]
    fn test_spirv_byte_emission() {
        let module = MirModule {
            functions: vec![make_test_gpu_func("compute_kernel")],
            enum_layouts: HashMap::new(),
            struct_layouts: HashMap::new(),
        };

        let bytes = emit_spirv_binary(&module).expect("SPIR-V binary emission");
        assert_eq!(bytes.len() % 4, 0, "SPIR-V bytes must be a multiple of 4");
        // Verify little-endian magic bytes
        assert_eq!(&bytes[0..4], &SPIRV_MAGIC.to_le_bytes());
    }
}
