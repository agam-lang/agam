//! C ABI Layout, Primitive Representation, and Struct Alignment Engine (repr(C)).

use serde::{Deserialize, Serialize};

/// C Primitive Type descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CPrimitive {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Pointer,
    Void,
}

impl CPrimitive {
    /// Return size in bytes on 64-bit platforms.
    pub const fn size(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::I64 | Self::U64 | Self::F64 | Self::Pointer => 8,
            Self::Void => 0,
        }
    }

    /// Return natural alignment in bytes on 64-bit platforms.
    pub const fn align(self) -> usize {
        self.size()
    }
}

/// C Calling Convention.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallingConvention {
    #[default]
    Cdecl,
    Stdcall,
    Fastcall,
    SysV64,
    Win64,
}

/// C Function Signature representation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CFuncSig {
    pub name: String,
    pub params: Vec<(String, CPrimitive)>,
    pub return_type: CPrimitive,
    pub conv: CallingConvention,
}

/// Field description in a `repr(C)` struct.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CField {
    pub name: String,
    pub primitive: CPrimitive,
    pub offset: usize,
}

/// Computed `repr(C)` struct layout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CStructLayout {
    pub name: String,
    pub fields: Vec<CField>,
    pub total_size: usize,
    pub alignment: usize,
}

impl CStructLayout {
    /// Compute strict `repr(C)` layout following ISO C standard alignment and padding.
    pub fn compute(name: impl Into<String>, field_defs: &[(&str, CPrimitive)]) -> Self {
        let mut fields = Vec::new();
        let mut current_offset = 0usize;
        let mut max_align = 1usize;

        for &(fname, prim) in field_defs {
            let align = prim.align().max(1);
            let size = prim.size();
            max_align = max_align.max(align);

            // Pad offset to multiple of field alignment
            if !current_offset.is_multiple_of(align) {
                current_offset += align - (current_offset % align);
            }

            fields.push(CField {
                name: fname.to_string(),
                primitive: prim,
                offset: current_offset,
            });

            current_offset += size;
        }

        // Tail padding to struct alignment
        if max_align > 0 && !current_offset.is_multiple_of(max_align) {
            current_offset += max_align - (current_offset % max_align);
        }

        Self {
            name: name.into(),
            fields,
            total_size: current_offset,
            alignment: max_align,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_struct_layout_padding_and_alignment() {
        // struct Test { char a; int b; char c; }
        // a at 0 (size 1)
        // padding 3 bytes
        // b at 4 (size 4)
        // c at 8 (size 1)
        // tail padding 3 bytes -> total size 12, align 4
        let layout = CStructLayout::compute(
            "Test",
            &[
                ("a", CPrimitive::I8),
                ("b", CPrimitive::I32),
                ("c", CPrimitive::I8),
            ],
        );

        assert_eq!(layout.alignment, 4);
        assert_eq!(layout.total_size, 12);
        assert_eq!(layout.fields[0].offset, 0);
        assert_eq!(layout.fields[1].offset, 4);
        assert_eq!(layout.fields[2].offset, 8);
    }
}
