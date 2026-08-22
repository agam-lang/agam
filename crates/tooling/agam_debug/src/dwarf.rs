//! DWARF 5 & CodeView Debug Line and Type Table Generator Metadata.
//!
//! Provides the compiler IR-to-debug-emitter abstraction mapping Agam source spans,
//! functions, variable storage locations, and composite type layouts into DWARF DIEs and CodeView records.

use serde::{Deserialize, Serialize};

/// Debug symbol format standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugFormat {
    /// DWARF 5 (Linux, macOS, Android, WebAssembly).
    Dwarf5,
    /// CodeView / PDB (Windows MSVC).
    CodeView,
}

/// Source line table entry mapping an instruction offset (PC) to source coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineEntry {
    pub address_offset: u64,
    pub file_index: u32,
    pub line: u32,
    pub column: u32,
    pub is_stmt: bool,
    pub is_prologue_end: bool,
    pub is_epilogue_begin: bool,
}

/// Source file metadata registered in debug compilation unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub file_name: String,
    pub directory: String,
    pub source_text: Option<String>,
    pub md5_checksum: Option<[u8; 16]>,
}

/// Function / Subprogram debug symbol record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubprogramEntry {
    pub name: String,
    pub linkage_name: String,
    pub file_index: u32,
    pub start_line: u32,
    pub low_pc: u64,
    pub high_pc: u64,
    pub is_external: bool,
    pub frame_base_reg: u16,
    pub variables: Vec<VariableLocationEntry>,
}

/// Storage location of a variable (stack slot, register, or constant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableLocation {
    /// Stored on stack at [frame_pointer + offset].
    StackOffset(i64),
    /// Stored in a physical register.
    Register(u16),
    /// Compile-time known constant value.
    Constant(Vec<u8>),
    /// Value was optimized out / dead.
    OptimizedOut,
}

/// Variable debug metadata record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableLocationEntry {
    pub name: String,
    pub type_name: String,
    pub file_index: u32,
    pub line: u32,
    pub location: VariableLocation,
}

/// Type representation in debug information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugType {
    Primitive {
        name: String,
        size_bytes: usize,
        encoding: PrimitiveEncoding,
    },
    Pointer {
        name: String,
        target_type: String,
        size_bytes: usize,
    },
    Struct {
        name: String,
        size_bytes: usize,
        fields: Vec<DebugStructField>,
    },
    Enum {
        name: String,
        size_bytes: usize,
        discriminant_offset: usize,
        variants: Vec<DebugEnumVariant>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveEncoding {
    SignedInt,
    UnsignedInt,
    Float,
    Boolean,
    Address,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugStructField {
    pub name: String,
    pub type_name: String,
    pub offset_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugEnumVariant {
    pub name: String,
    pub discriminant_value: i64,
    pub payload_type: Option<String>,
}

/// Debug compilation unit collecting line tables, subprograms, and types for a module.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugCompilationUnit {
    pub producer: String,
    pub language: String,
    pub source_root: String,
    pub files: Vec<FileEntry>,
    pub line_table: Vec<LineEntry>,
    pub subprograms: Vec<SubprogramEntry>,
    pub types: Vec<DebugType>,
}

impl DebugCompilationUnit {
    pub fn new(source_root: impl Into<String>) -> Self {
        Self {
            producer: "agamc (Agam Compiler with LLVM backend)".into(),
            language: "Agam (Sanskrit-Grammar High-Performance Language)".into(),
            source_root: source_root.into(),
            files: Vec::new(),
            line_table: Vec::new(),
            subprograms: Vec::new(),
            types: Vec::new(),
        }
    }

    pub fn add_file(&mut self, file_name: impl Into<String>, directory: impl Into<String>) -> u32 {
        let index = self.files.len() as u32;
        self.files.push(FileEntry {
            file_name: file_name.into(),
            directory: directory.into(),
            source_text: None,
            md5_checksum: None,
        });
        index
    }

    pub fn add_line(&mut self, entry: LineEntry) {
        self.line_table.push(entry);
    }

    pub fn add_subprogram(&mut self, subprog: SubprogramEntry) {
        self.subprograms.push(subprog);
    }
}
