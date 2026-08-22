//! eBPF (Extended Berkeley Packet Filter) Kernel Observability & Tracing Engine.
//!
//! Provides unified in-memory map abstractions (Hash, Array, RingBuffer),
//! bytecode instruction emission, program loaders, and compile-time verifier checks.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;

/// eBPF Map types supported by the Linux kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfMapKind {
    HashMap,
    Array,
    RingBuffer,
    PerCpuArray,
    LpmTrie,
}

/// Type-safe in-memory eBPF Map simulation and kernel bridge.
pub struct EbpfMap<K: Eq + Hash + Clone, V: Clone> {
    pub name: String,
    pub kind: EbpfMapKind,
    pub max_entries: usize,
    storage: Mutex<HashMap<K, V>>,
}

impl<K: Eq + Hash + Clone, V: Clone> EbpfMap<K, V> {
    pub fn new(name: impl Into<String>, kind: EbpfMapKind, max_entries: usize) -> Self {
        Self {
            name: name.into(),
            kind,
            max_entries,
            storage: Mutex::new(HashMap::new()),
        }
    }

    pub fn lookup(&self, key: &K) -> Option<V> {
        let guard = self.storage.lock().unwrap();
        guard.get(key).cloned()
    }

    pub fn update(&self, key: K, value: V) -> Result<(), &'static str> {
        let mut guard = self.storage.lock().unwrap();
        if guard.len() >= self.max_entries && !guard.contains_key(&key) {
            return Err("eBPF Map maximum entries limit reached");
        }
        guard.insert(key, value);
        Ok(())
    }

    pub fn delete(&self, key: &K) -> bool {
        let mut guard = self.storage.lock().unwrap();
        guard.remove(key).is_some()
    }

    pub fn len(&self) -> usize {
        let guard = self.storage.lock().unwrap();
        guard.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// eBPF Program Attachment Hook Kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfProgramKind {
    /// XDP (eXpress Data Path) high-performance network packet filtering.
    Xdp,
    /// Kernel dynamic probe entry.
    Kprobe,
    /// Kernel dynamic probe return.
    Kretprobe,
    /// Kernel tracepoint hook.
    Tracepoint,
    /// Socket filter hook.
    SocketFilter,
}

/// 64-bit eBPF Raw Instruction structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EbpfInstruction {
    pub opcode: u8,
    pub dst_reg: u8,
    pub src_reg: u8,
    pub offset: i16,
    pub imm: i32,
}

impl EbpfInstruction {
    pub fn new(opcode: u8, dst: u8, src: u8, offset: i16, imm: i32) -> Self {
        Self {
            opcode,
            dst_reg: dst,
            src_reg: src,
            offset,
            imm,
        }
    }

    /// `mov64 dst, imm`
    pub fn mov64_imm(dst: u8, imm: i32) -> Self {
        Self::new(0xb7, dst, 0, 0, imm)
    }

    /// `add64 dst, src`
    pub fn add64_reg(dst: u8, src: u8) -> Self {
        Self::new(0x0f, dst, src, 0, 0)
    }

    /// `exit`
    pub fn exit() -> Self {
        Self::new(0x95, 0, 0, 0, 0)
    }
}

/// Static Verifier ensuring kernel safety constraints.
pub struct EbpfVerifier;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifierError {
    EmptyProgram,
    ProgramTooLarge(usize),
    MissingExitInstruction,
    InvalidRegister(u8),
    UnreachableInstructions(usize),
}

impl std::fmt::Display for VerifierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyProgram => write!(f, "eBPF program contains 0 instructions"),
            Self::ProgramTooLarge(len) => {
                write!(
                    f,
                    "eBPF program exceeds max instruction limit (found {len})"
                )
            }
            Self::MissingExitInstruction => {
                write!(f, "eBPF program is missing a terminal BPF_EXIT instruction")
            }
            Self::InvalidRegister(r) => write!(f, "eBPF invalid register r{r} (valid: r0-r10)"),
            Self::UnreachableInstructions(idx) => {
                write!(f, "Unreachable instructions detected after exit at {idx}")
            }
        }
    }
}

impl std::error::Error for VerifierError {}

impl EbpfVerifier {
    /// Verify that an instruction stream complies with eBPF safety rules.
    pub fn verify(insns: &[EbpfInstruction]) -> Result<(), VerifierError> {
        if insns.is_empty() {
            return Err(VerifierError::EmptyProgram);
        }

        if insns.len() > 4096 {
            return Err(VerifierError::ProgramTooLarge(insns.len()));
        }

        // Validate registers
        for insn in insns {
            if insn.dst_reg > 10 {
                return Err(VerifierError::InvalidRegister(insn.dst_reg));
            }
            if insn.src_reg > 10 {
                return Err(VerifierError::InvalidRegister(insn.src_reg));
            }
        }

        // Must end with BPF_EXIT (opcode 0x95)
        if insns.last().map(|i| i.opcode) != Some(0x95) {
            return Err(VerifierError::MissingExitInstruction);
        }

        Ok(())
    }
}

/// Loaded eBPF Program Container.
pub struct EbpfProgram {
    pub name: String,
    pub kind: EbpfProgramKind,
    pub instructions: Vec<EbpfInstruction>,
    pub is_attached: bool,
}

impl EbpfProgram {
    pub fn new(
        name: impl Into<String>,
        kind: EbpfProgramKind,
        instructions: Vec<EbpfInstruction>,
    ) -> Result<Self, VerifierError> {
        EbpfVerifier::verify(&instructions)?;
        Ok(Self {
            name: name.into(),
            kind,
            instructions,
            is_attached: false,
        })
    }

    pub fn attach(&mut self, _target_interface: &str) -> Result<(), &'static str> {
        self.is_attached = true;
        Ok(())
    }

    pub fn detach(&mut self) {
        self.is_attached = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebpf_map_lifecycle() {
        let map: EbpfMap<String, u64> =
            EbpfMap::new("packet_drop_counts", EbpfMapKind::HashMap, 100);

        assert!(map.is_empty());
        map.update("eth0".into(), 42).expect("Update map");
        assert_eq!(map.lookup(&"eth0".into()), Some(42));
        assert_eq!(map.len(), 1);

        assert!(map.delete(&"eth0".into()));
        assert_eq!(map.lookup(&"eth0".into()), None);
    }

    #[test]
    fn test_ebpf_program_verification() {
        let valid_insns = vec![
            EbpfInstruction::mov64_imm(0, 2), // r0 = 2 (XDP_PASS)
            EbpfInstruction::exit(),
        ];

        let prog = EbpfProgram::new("xdp_pass_filter", EbpfProgramKind::Xdp, valid_insns);
        assert!(prog.is_ok());

        let invalid_insns = vec![
            EbpfInstruction::mov64_imm(0, 1),
            // Missing exit
        ];
        let bad_prog = EbpfProgram::new("broken_filter", EbpfProgramKind::Xdp, invalid_insns);
        assert!(bad_prog.is_err());
    }
}
