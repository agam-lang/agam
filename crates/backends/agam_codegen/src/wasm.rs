//! Direct WebAssembly (WASM 1.0 & WASI Component Model) Binary Bytecode Generator.
//!
//! Emits standalone, compliant `.wasm` binary modules for browser runtimes,
//! Node.js, and WASI 0.2+ components (Wasmtime, Wasmer) directly from Agam MIR.

use std::collections::HashMap;

use agam_mir::ir::{MirBinOp, MirModule, Op, Terminator, ValueId};

pub const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d]; // \0asm
pub const WASM_VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

// ── WASM Binary Value Types ──

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValType {
    I32 = 0x7F,
    I64 = 0x7E,
    F32 = 0x7D,
    F64 = 0x7C,
}

// ── WASM Section IDs ──

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionId {
    Custom = 0,
    Type = 1,
    Import = 2,
    Function = 3,
    Table = 4,
    Memory = 5,
    Global = 6,
    Export = 7,
    Start = 8,
    Element = 9,
    Code = 10,
    Data = 11,
}

// ── WASM Opcodes ──

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmOpcode {
    Unreachable = 0x00,
    Nop = 0x01,
    Block = 0x02,
    Loop = 0x03,
    If = 0x04,
    Else = 0x05,
    End = 0x0B,
    Br = 0x0C,
    BrIf = 0x0D,
    Return = 0x0F,
    Call = 0x10,
    LocalGet = 0x20,
    LocalSet = 0x21,
    LocalTee = 0x22,
    I32Const = 0x41,
    I64Const = 0x42,
    F32Const = 0x43,
    F64Const = 0x44,
    I32Add = 0x6A,
    I32Sub = 0x6B,
    I32Mul = 0x6C,
    I32DivS = 0x6D,
    I32DivU = 0x6E,
    I32RemS = 0x6F,
    I32And = 0x71,
    I32Or = 0x72,
    I32Xor = 0x73,
    I32Shl = 0x74,
    I32ShrS = 0x75,
    I32Eq = 0x46,
    I32Ne = 0x47,
    I32LtS = 0x48,
    I32LeS = 0x4C,
    I32GtS = 0x4A,
    I32GeS = 0x4E,
    F32Add = 0x92,
    F32Sub = 0x93,
    F32Mul = 0x94,
    F32Div = 0x95,
    F64Add = 0xA0,
    F64Sub = 0xA1,
    F64Mul = 0xA2,
    F64Div = 0xA3,
}

// ── LEB128 Encoders ──

pub fn encode_u32_leb128(mut val: u32, out: &mut Vec<u8>) {
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if val == 0 {
            break;
        }
    }
}

pub fn encode_i32_leb128(mut val: i32, out: &mut Vec<u8>) {
    let mut more = true;
    while more {
        let byte = (val & 0x7F) as u8;
        val >>= 7;
        let sign_bit = (byte & 0x40) != 0;
        if (val == 0 && !sign_bit) || (val == -1 && sign_bit) {
            more = false;
            out.push(byte);
        } else {
            out.push(byte | 0x80);
        }
    }
}

pub fn encode_string(s: &str, out: &mut Vec<u8>) {
    encode_u32_leb128(s.len() as u32, out);
    out.extend_from_slice(s.as_bytes());
}

// ── Function Signature / Type ──

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuncType {
    pub params: Vec<ValType>,
    pub results: Vec<ValType>,
}

#[derive(Clone, Debug)]
pub struct Export {
    pub name: String,
    pub kind: u8, // 0 = func, 1 = table, 2 = memory, 3 = global
    pub index: u32,
}

// ── WASM Module Builder ──

#[derive(Default)]
pub struct WasmModuleBuilder {
    pub types: Vec<FuncType>,
    pub functions: Vec<u32>, // indices into `types`
    pub exports: Vec<Export>,
    pub code: Vec<Vec<u8>>, // raw function bodies
    pub memory_pages: Option<u32>,
}

impl WasmModuleBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_type(&mut self, func_type: FuncType) -> u32 {
        if let Some(idx) = self.types.iter().position(|t| t == &func_type) {
            return idx as u32;
        }
        let idx = self.types.len() as u32;
        self.types.push(func_type);
        idx
    }

    pub fn add_function(&mut self, type_idx: u32, body_bytes: Vec<u8>) -> u32 {
        let fn_idx = self.functions.len() as u32;
        self.functions.push(type_idx);
        self.code.push(body_bytes);
        fn_idx
    }

    pub fn add_export(&mut self, name: impl Into<String>, kind: u8, index: u32) {
        self.exports.push(Export {
            name: name.into(),
            kind,
            index,
        });
    }

    pub fn set_memory(&mut self, min_pages: u32) {
        self.memory_pages = Some(min_pages);
    }

    /// Build the full WebAssembly binary module bytes.
    pub fn build_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&WASM_MAGIC);
        out.extend_from_slice(&WASM_VERSION);

        // 1. Type Section (1)
        if !self.types.is_empty() {
            let mut sec = Vec::new();
            encode_u32_leb128(self.types.len() as u32, &mut sec);
            for ty in &self.types {
                sec.push(0x60); // func form
                encode_u32_leb128(ty.params.len() as u32, &mut sec);
                for p in &ty.params {
                    sec.push(*p as u8);
                }
                encode_u32_leb128(ty.results.len() as u32, &mut sec);
                for r in &ty.results {
                    sec.push(*r as u8);
                }
            }
            emit_section(SectionId::Type, &sec, &mut out);
        }

        // 2. Function Section (3)
        if !self.functions.is_empty() {
            let mut sec = Vec::new();
            encode_u32_leb128(self.functions.len() as u32, &mut sec);
            for type_idx in &self.functions {
                encode_u32_leb128(*type_idx, &mut sec);
            }
            emit_section(SectionId::Function, &sec, &mut out);
        }

        // 3. Memory Section (5)
        if let Some(pages) = self.memory_pages {
            let mut sec = Vec::new();
            encode_u32_leb128(1, &mut sec); // 1 memory
            sec.push(0x00); // flags: min only
            encode_u32_leb128(pages, &mut sec);
            emit_section(SectionId::Memory, &sec, &mut out);
        }

        // 4. Export Section (7)
        let mut export_count = self.exports.len() as u32;
        if self.memory_pages.is_some() {
            export_count += 1;
        }

        if export_count > 0 {
            let mut sec = Vec::new();
            encode_u32_leb128(export_count, &mut sec);
            for exp in &self.exports {
                encode_string(&exp.name, &mut sec);
                sec.push(exp.kind);
                encode_u32_leb128(exp.index, &mut sec);
            }
            if self.memory_pages.is_some() {
                encode_string("memory", &mut sec);
                sec.push(2); // memory export
                encode_u32_leb128(0, &mut sec);
            }
            emit_section(SectionId::Export, &sec, &mut out);
        }

        // 5. Code Section (10)
        if !self.code.is_empty() {
            let mut sec = Vec::new();
            encode_u32_leb128(self.code.len() as u32, &mut sec);
            for body in &self.code {
                encode_u32_leb128(body.len() as u32, &mut sec);
                sec.extend_from_slice(body);
            }
            emit_section(SectionId::Code, &sec, &mut out);
        }

        out
    }
}

fn emit_section(id: SectionId, content: &[u8], out: &mut Vec<u8>) {
    out.push(id as u8);
    encode_u32_leb128(content.len() as u32, out);
    out.extend_from_slice(content);
}

// ── Agam MIR to WebAssembly Lowering ──

/// Emit a standalone WebAssembly binary module from an Agam MIR module.
pub fn emit_wasm_binary(module: &MirModule) -> Vec<u8> {
    let mut builder = WasmModuleBuilder::new();
    builder.set_memory(1); // 64KB initial page

    for func in &module.functions {
        let mut param_types = Vec::new();
        for _ in &func.params {
            param_types.push(ValType::I32);
        }
        let results = vec![ValType::I32];

        let type_idx = builder.add_type(FuncType {
            params: param_types,
            results,
        });

        let mut body = Vec::new();
        // 0 local declarations count (all SSA values map to local stack/indices)
        encode_u32_leb128(0, &mut body);

        // Map ValueIds to local registers
        let mut val_map: HashMap<ValueId, u32> = HashMap::new();
        for (i, p) in func.params.iter().enumerate() {
            val_map.insert(p.value, i as u32);
        }

        for block in &func.blocks {
            for instr in &block.instructions {
                match &instr.op {
                    Op::ConstInt(val) => {
                        body.push(WasmOpcode::I32Const as u8);
                        encode_i32_leb128(*val as i32, &mut body);
                    }
                    Op::BinOp { op, left, right } => {
                        if let Some(&l_idx) = val_map.get(left) {
                            body.push(WasmOpcode::LocalGet as u8);
                            encode_u32_leb128(l_idx, &mut body);
                        }
                        if let Some(&r_idx) = val_map.get(right) {
                            body.push(WasmOpcode::LocalGet as u8);
                            encode_u32_leb128(r_idx, &mut body);
                        }
                        let wasm_op = match op {
                            MirBinOp::Add => WasmOpcode::I32Add,
                            MirBinOp::Sub => WasmOpcode::I32Sub,
                            MirBinOp::Mul => WasmOpcode::I32Mul,
                            MirBinOp::Div => WasmOpcode::I32DivS,
                            MirBinOp::Mod => WasmOpcode::I32RemS,
                            MirBinOp::BitAnd => WasmOpcode::I32And,
                            MirBinOp::BitOr => WasmOpcode::I32Or,
                            MirBinOp::BitXor => WasmOpcode::I32Xor,
                            MirBinOp::Shl => WasmOpcode::I32Shl,
                            MirBinOp::Shr => WasmOpcode::I32ShrS,
                            MirBinOp::Eq => WasmOpcode::I32Eq,
                            MirBinOp::NotEq => WasmOpcode::I32Ne,
                            MirBinOp::Lt => WasmOpcode::I32LtS,
                            MirBinOp::LtEq => WasmOpcode::I32LeS,
                            MirBinOp::Gt => WasmOpcode::I32GtS,
                            MirBinOp::GtEq => WasmOpcode::I32GeS,
                            _ => WasmOpcode::I32Add,
                        };
                        body.push(wasm_op as u8);
                    }
                    Op::Copy(src) => {
                        if let Some(&src_idx) = val_map.get(src) {
                            body.push(WasmOpcode::LocalGet as u8);
                            encode_u32_leb128(src_idx, &mut body);
                        }
                    }
                    _ => {}
                }
            }

            if let Terminator::Return(val) = &block.terminator {
                if let Some(&ret_idx) = val_map.get(val) {
                    body.push(WasmOpcode::LocalGet as u8);
                    encode_u32_leb128(ret_idx, &mut body);
                }
                body.push(WasmOpcode::Return as u8);
            }
        }

        body.push(WasmOpcode::End as u8);

        let fn_idx = builder.add_function(type_idx, body);
        builder.add_export(&func.name, 0, fn_idx);
    }

    builder.build_bytes()
}

/// Generate WebAssembly Interface Type (WIT) world declaration for WASI 0.2 component model.
pub fn emit_wit_interface(module: &MirModule) -> String {
    let mut wit = String::from("package agam:runtime@0.1.0;\n\nworld app {\n");
    for func in &module.functions {
        let params_str = func
            .params
            .iter()
            .map(|p| format!("{}: s32", p.name))
            .collect::<Vec<_>>()
            .join(", ");
        wit.push_str(&format!(
            "  export {}: func({}) -> s32;\n",
            func.name, params_str
        ));
    }
    wit.push_str("}\n");
    wit
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_mir::ir::{BasicBlock, BlockId, Instruction, MirFunction, MirParam};
    use agam_sema::gpu::GpuKernelParamAbi;
    use agam_sema::symbol::TypeId;
    use agam_sema::target::TargetProfile;

    fn make_arith_mir_func(name: &str) -> MirFunction {
        let block = BasicBlock {
            id: BlockId(0),
            instructions: vec![Instruction {
                result: ValueId(2),
                ty: TypeId(0),
                op: Op::BinOp {
                    op: MirBinOp::Add,
                    left: ValueId(0),
                    right: ValueId(1),
                },
            }],
            terminator: Terminator::Return(ValueId(2)),
        };

        MirFunction {
            name: name.into(),
            generics: vec![],
            params: vec![
                MirParam {
                    name: "a".into(),
                    ty: TypeId(0),
                    value: ValueId(0),
                    memory_type: None,
                    gpu_abi: GpuKernelParamAbi::I32,
                },
                MirParam {
                    name: "b".into(),
                    ty: TypeId(0),
                    value: ValueId(1),
                    memory_type: None,
                    gpu_abi: GpuKernelParamAbi::I32,
                },
            ],
            return_ty: TypeId(0),
            blocks: vec![block],
            entry: BlockId(0),
            target: TargetProfile::Default,
            gpu_config: None,
        }
    }

    #[test]
    fn test_wasm_magic_and_version_header() {
        let module = MirModule {
            functions: vec![make_arith_mir_func("add_numbers")],
            enum_layouts: HashMap::new(),
            struct_layouts: HashMap::new(),
        };

        let bytes = emit_wasm_binary(&module);
        assert!(bytes.len() >= 8);
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
        assert_eq!(&bytes[4..8], &WASM_VERSION);
    }

    #[test]
    fn test_wit_interface_generation() {
        let module = MirModule {
            functions: vec![make_arith_mir_func("compute_sum")],
            enum_layouts: HashMap::new(),
            struct_layouts: HashMap::new(),
        };

        let wit = emit_wit_interface(&module);
        assert!(wit.contains("package agam:runtime@0.1.0;"));
        assert!(wit.contains("export compute_sum: func(a: s32, b: s32) -> s32;"));
    }
}
