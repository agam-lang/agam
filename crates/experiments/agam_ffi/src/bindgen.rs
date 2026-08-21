//! C Header Bindgen Parser and Agam `extern "C"` Interface Generator.

use crate::c_abi::{CFuncSig, CPrimitive, CallingConvention};

fn parse_c_type(ty_str: &str) -> Option<CPrimitive> {
    let s = ty_str.trim();
    if s.ends_with('*') {
        return Some(CPrimitive::Pointer);
    }
    match s {
        "void" => Some(CPrimitive::Void),
        "char" | "int8_t" => Some(CPrimitive::I8),
        "uint8_t" | "unsigned char" => Some(CPrimitive::U8),
        "short" | "int16_t" => Some(CPrimitive::I16),
        "uint16_t" | "unsigned short" => Some(CPrimitive::U16),
        "int" | "int32_t" => Some(CPrimitive::I32),
        "uint32_t" | "unsigned int" => Some(CPrimitive::U32),
        "long" | "long long" | "int64_t" | "ssize_t" | "size_t" | "intptr_t" => {
            Some(CPrimitive::I64)
        }
        "uint64_t" | "unsigned long" | "uintptr_t" => Some(CPrimitive::U64),
        "float" => Some(CPrimitive::F32),
        "double" => Some(CPrimitive::F64),
        _ => None,
    }
}

fn c_type_to_agam_type(prim: CPrimitive) -> &'static str {
    match prim {
        CPrimitive::Void => "()",
        CPrimitive::I8 => "i8",
        CPrimitive::U8 => "u8",
        CPrimitive::I16 => "i16",
        CPrimitive::U16 => "u16",
        CPrimitive::I32 => "i32",
        CPrimitive::U32 => "u32",
        CPrimitive::I64 => "i64",
        CPrimitive::U64 => "u64",
        CPrimitive::F32 => "f32",
        CPrimitive::F64 => "f64",
        CPrimitive::Pointer => "*mut ()",
    }
}

/// Parse a C function prototype into a `CFuncSig`.
///
/// Example: `int calculate_sum(int a, int b);`
pub fn parse_c_function_prototype(line: &str) -> Option<CFuncSig> {
    let clean = line.trim().trim_end_matches(';').trim();
    let paren_start = clean.find('(')?;
    let paren_end = clean.rfind(')')?;

    let left = &clean[..paren_start].trim();
    let params_part = &clean[paren_start + 1..paren_end].trim();

    let last_space = left.rfind(|c: char| c.is_whitespace() || c == '*')?;
    let return_type_str = left[..=last_space].trim();
    let fn_name = left[last_space + 1..].trim();

    let return_type = parse_c_type(return_type_str)?;

    let mut params = Vec::new();
    if !params_part.is_empty() && *params_part != "void" {
        for arg in params_part.split(',') {
            let arg_clean = arg.trim();
            if let Some(space_idx) = arg_clean.rfind(|c: char| c.is_whitespace() || c == '*') {
                let ty_str = arg_clean[..=space_idx].trim();
                let name_str = arg_clean[space_idx + 1..].trim();
                let prim = parse_c_type(ty_str)?;
                params.push((name_str.to_string(), prim));
            } else {
                let prim = parse_c_type(arg_clean)?;
                params.push((format!("arg_{}", params.len()), prim));
            }
        }
    }

    Some(CFuncSig {
        name: fn_name.to_string(),
        params,
        return_type,
        conv: CallingConvention::Cdecl,
    })
}

/// Generate idiomatic Agam `extern "C"` declaration block from C function signatures.
pub fn generate_agam_extern_block(signatures: &[CFuncSig]) -> String {
    let mut out = String::from("extern \"C\" {\n");
    for sig in signatures {
        let params_str = sig
            .params
            .iter()
            .map(|(name, ty)| format!("{}: {}", name, c_type_to_agam_type(*ty)))
            .collect::<Vec<_>>()
            .join(", ");

        let ret_str = c_type_to_agam_type(sig.return_type);
        if sig.return_type == CPrimitive::Void {
            out.push_str(&format!("    fn {}({});\n", sig.name, params_str));
        } else {
            out.push_str(&format!(
                "    fn {}({}) -> {};\n",
                sig.name, params_str, ret_str
            ));
        }
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_c_prototype_and_generate_agam_extern() {
        let proto1 = "int calculate_sum(int a, int b);";
        let sig1 = parse_c_function_prototype(proto1).expect("should parse");
        assert_eq!(sig1.name, "calculate_sum");
        assert_eq!(sig1.params.len(), 2);
        assert_eq!(sig1.return_type, CPrimitive::I32);

        let proto2 = "double compute_hypotenuse(double x, double y);";
        let sig2 = parse_c_function_prototype(proto2).expect("should parse");
        assert_eq!(sig2.name, "compute_hypotenuse");
        assert_eq!(sig2.return_type, CPrimitive::F64);

        let code = generate_agam_extern_block(&[sig1, sig2]);
        assert!(code.contains("fn calculate_sum(a: i32, b: i32) -> i32;"));
        assert!(code.contains("fn compute_hypotenuse(x: f64, y: f64) -> f64;"));
    }
}
