//! Automated C Header Parser and Foreign Binding Synthesizer (`agam_ffi::bindgen`).
//!
//! Ingests standard C header declarations (`typedef`, `struct`, `union`, `enum`, function prototypes),
//! maps C types to canonical Agam types, and synthesizes type-safe `foreign "C"` modules.

#![deny(clippy::unwrap_used)]

use std::fmt;

use crate::c_abi::{CFuncSig, CPrimitive, CallingConvention};

/// Representation of C types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CType {
    Void,
    Bool,
    Char,
    UChar,
    Short,
    UShort,
    Int,
    UInt,
    Long,
    ULong,
    LongLong,
    ULongLong,
    Float,
    Double,
    Pointer(Box<CType>),
    ConstPointer(Box<CType>),
    Array(Box<CType>, usize),
    Named(String),
}

impl CType {
    /// Format this C type into its canonical Agam type representation.
    pub fn to_agam_type(&self) -> String {
        match self {
            CType::Void => "unit".to_string(),
            CType::Bool => "bool".to_string(),
            CType::Char => "i8".to_string(),
            CType::UChar => "u8".to_string(),
            CType::Short => "i16".to_string(),
            CType::UShort => "u16".to_string(),
            CType::Int => "i32".to_string(),
            CType::UInt => "u32".to_string(),
            CType::Long => "i64".to_string(),
            CType::ULong => "u64".to_string(),
            CType::LongLong => "i64".to_string(),
            CType::ULongLong => "u64".to_string(),
            CType::Float => "f32".to_string(),
            CType::Double => "f64".to_string(),
            CType::ConstPointer(inner) => match &**inner {
                CType::Char => "c_string".to_string(),
                _ => format!("const_ptr[{}]", inner.to_agam_type()),
            },
            CType::Pointer(inner) => match &**inner {
                CType::Char => "c_string".to_string(),
                CType::Void => "ptr[unit]".to_string(),
                _ => format!("ptr[{}]", inner.to_agam_type()),
            },
            CType::Array(inner, size) => format!("[{}; {}]", inner.to_agam_type(), size),
            CType::Named(name) => name.clone(),
        }
    }
}

/// Struct or union field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CField {
    pub name: String,
    pub ty: CType,
}

/// Struct or union declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CStructDecl {
    pub name: String,
    pub fields: Vec<CField>,
    pub is_union: bool,
}

/// Enum variant with optional explicit integer value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CEnumVariant {
    pub name: String,
    pub value: Option<i64>,
}

/// Enum declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CEnumDecl {
    pub name: String,
    pub variants: Vec<CEnumVariant>,
}

/// Function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CParam {
    pub name: String,
    pub ty: CType,
}

/// Function prototype declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CFunctionDecl {
    pub name: String,
    pub return_type: CType,
    pub params: Vec<CParam>,
    pub is_variadic: bool,
}

/// Configuration options for the C header parser and code generator.
#[derive(Debug, Clone, Default)]
pub struct BindgenConfig {
    pub library_name: String,
    pub type_prefix: Option<String>,
    pub allowlist_functions: Vec<String>,
    pub allowlist_types: Vec<String>,
}

/// Result of parsing a C header.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BindgenResult {
    pub structs: Vec<CStructDecl>,
    pub enums: Vec<CEnumDecl>,
    pub functions: Vec<CFunctionDecl>,
}

/// Structured bindgen diagnostic error formatted in the Agam Nyāya voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindgenError {
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl BindgenError {
    pub fn new(
        cause: impl fmt::Display,
        context: impl fmt::Display,
        remedy: impl fmt::Display,
    ) -> Self {
        Self {
            cause: cause.to_string(),
            context: context.to_string(),
            remedy: remedy.to_string(),
        }
    }
}

impl fmt::Display for BindgenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Bindgen Diagnostic: {}\n  Context: {}\n  Remedy:  {}",
            self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for BindgenError {}

/// Automated C header parser and code generator.
pub struct CHeaderParser {
    config: BindgenConfig,
}

impl CHeaderParser {
    pub fn new(config: BindgenConfig) -> Self {
        Self { config }
    }

    /// Parse raw C header source code into AST declarations.
    pub fn parse_header(&self, header_content: &str) -> Result<BindgenResult, BindgenError> {
        let clean = remove_comments_and_preprocessor(header_content);
        let mut result = BindgenResult::default();

        let mut rest = clean.as_str();
        while !rest.trim().is_empty() {
            rest = rest.trim_start();
            if rest.is_empty() {
                break;
            }

            // Check for enum
            if rest.starts_with("enum ") || rest.starts_with("typedef enum") {
                let (enum_decl, next) = parse_enum_decl(rest)?;
                let included = self.config.allowlist_types.is_empty()
                    || self.config.allowlist_types.contains(&enum_decl.name);
                if included {
                    result.enums.push(enum_decl);
                }
                rest = next;
                continue;
            }

            // Check for struct or union
            if rest.starts_with("struct ")
                || rest.starts_with("typedef struct")
                || rest.starts_with("union ")
                || rest.starts_with("typedef union")
            {
                let (struct_decl, next) = parse_struct_or_union_decl(rest)?;
                let included = self.config.allowlist_types.is_empty()
                    || self.config.allowlist_types.contains(&struct_decl.name);
                if included {
                    result.structs.push(struct_decl);
                }
                rest = next;
                continue;
            }

            // Check for function declaration up to ';'
            if let Some(semi_pos) = rest.find(';') {
                let decl_str = rest[..semi_pos].trim();
                if !decl_str.is_empty() {
                    let func_decl = parse_function_decl(decl_str)?;
                    let included = self.config.allowlist_functions.is_empty()
                        || self.config.allowlist_functions.contains(&func_decl.name);
                    if included {
                        result.functions.push(func_decl);
                    }
                }
                rest = &rest[semi_pos + 1..];
            } else {
                return Err(BindgenError::new(
                    "Unexpected end of header input without terminating semicolon",
                    format!("Trailing unparsed content: '{}'", rest.trim()),
                    "Ensure all C declarations end with a valid semicolon ';'",
                ));
            }
        }

        Ok(result)
    }

    /// Parse a header and synthesize an Agam `foreign "C"` module.
    pub fn generate_agam_module(&self, header_content: &str) -> Result<String, BindgenError> {
        let result = self.parse_header(header_content)?;
        Ok(self.generate_agam_code(&result))
    }

    /// Synthesize Agam source code from parsed declarations.
    pub fn generate_agam_code(&self, result: &BindgenResult) -> String {
        let mut out = String::new();

        if self.config.library_name.is_empty() {
            out.push_str("foreign \"C\" {\n");
        } else {
            out.push_str(&format!(
                "foreign \"C\" from \"{}\" {{\n",
                self.config.library_name
            ));
        }

        // Structs and Unions
        for s in &result.structs {
            let kind = if s.is_union { "union" } else { "struct" };
            out.push_str(&format!("    type {} = {} {{\n", s.name, kind));
            for f in &s.fields {
                out.push_str(&format!("        {}: {},\n", f.name, f.ty.to_agam_type()));
            }
            out.push_str("    }\n");
        }

        // Enums
        for e in &result.enums {
            out.push_str(&format!("    type {} = enum {{\n", e.name));
            for v in &e.variants {
                if let Some(val) = v.value {
                    out.push_str(&format!("        {} = {},\n", v.name, val));
                } else {
                    out.push_str(&format!("        {},\n", v.name));
                }
            }
            out.push_str("    }\n");
        }

        // Functions
        for f in &result.functions {
            let mut params_vec = Vec::new();
            for p in &f.params {
                params_vec.push(format!("{}: {}", p.name, p.ty.to_agam_type()));
            }
            if f.is_variadic {
                params_vec.push("...".to_string());
            }
            let params_str = params_vec.join(", ");

            if f.return_type == CType::Void {
                out.push_str(&format!("    fn {}({})\n", f.name, params_str));
            } else {
                out.push_str(&format!(
                    "    fn {}({}) -> {}\n",
                    f.name,
                    params_str,
                    f.return_type.to_agam_type()
                ));
            }
        }

        out.push_str("}\n");
        out
    }
}

fn remove_comments_and_preprocessor(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_block_comment = false;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }

        let mut idx = 0;
        let bytes = line.as_bytes();
        let len = bytes.len();

        while idx < len {
            if in_block_comment {
                if idx + 1 < len && bytes[idx] == b'*' && bytes[idx + 1] == b'/' {
                    in_block_comment = false;
                    idx += 2;
                } else {
                    idx += 1;
                }
            } else if idx + 1 < len && bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
                in_block_comment = true;
                idx += 2;
            } else if idx + 1 < len && bytes[idx] == b'/' && bytes[idx + 1] == b'/' {
                break;
            } else {
                out.push(bytes[idx] as char);
                idx += 1;
            }
        }
        out.push('\n');
    }
    out
}

fn parse_c_type_str(s: &str) -> Result<CType, BindgenError> {
    let raw = s.trim();
    if raw.is_empty() {
        return Err(BindgenError::new(
            "Empty C type specifier",
            "Found blank type string during header parsing",
            "Specify a valid C type like 'int', 'void*', or 'const char*'",
        ));
    }

    let is_const = raw.starts_with("const ") || raw.starts_with("const\t");
    let clean = if is_const {
        raw.trim_start_matches("const").trim()
    } else {
        raw
    };

    // Check pointer suffix
    if clean.ends_with('*') {
        let base_str = clean.trim_end_matches('*').trim();
        let base_ty = parse_c_type_str(base_str)?;
        let ptr_ty = if is_const {
            CType::ConstPointer(Box::new(base_ty))
        } else {
            CType::Pointer(Box::new(base_ty))
        };
        return Ok(ptr_ty);
    }

    // Check array suffix [N]
    if let Some(open_brk) = clean.rfind('[') {
        if let Some(close_brk) = clean.rfind(']') {
            if open_brk < close_brk {
                let base_str = clean[..open_brk].trim();
                let size_str = clean[open_brk + 1..close_brk].trim();
                let size = size_str.parse::<usize>().map_err(|_| {
                    BindgenError::new(
                        format!("Invalid array dimension '{}'", size_str),
                        format!("Failed to parse array size in type '{}'", raw),
                        "Use integer literal constants for C array dimensions",
                    )
                })?;
                let base_ty = parse_c_type_str(base_str)?;
                return Ok(CType::Array(Box::new(base_ty), size));
            }
        }
    }

    let resolved = match clean {
        "void" => CType::Void,
        "bool" | "_Bool" => CType::Bool,
        "char" | "int8_t" | "signed char" => CType::Char,
        "unsigned char" | "uint8_t" | "uchar" => CType::UChar,
        "short" | "short int" | "int16_t" | "signed short" => CType::Short,
        "unsigned short" | "uint16_t" | "ushort" => CType::UShort,
        "int" | "int32_t" | "signed int" | "signed" => CType::Int,
        "unsigned int" | "uint32_t" | "unsigned" | "uint" => CType::UInt,
        "long" | "long int" | "signed long" => CType::Long,
        "unsigned long" | "uint64_t" | "unsigned long int" => CType::ULong,
        "long long" | "long long int" | "int64_t" | "ssize_t" | "size_t" | "intptr_t" => {
            CType::LongLong
        }
        "unsigned long long" | "uintptr_t" => CType::ULongLong,
        "float" => CType::Float,
        "double" => CType::Double,
        other => {
            let clean_name = other
                .trim_start_matches("struct ")
                .trim_start_matches("union ")
                .trim_start_matches("enum ")
                .trim();
            CType::Named(clean_name.to_string())
        }
    };

    Ok(resolved)
}

fn parse_function_decl(decl_str: &str) -> Result<CFunctionDecl, BindgenError> {
    let clean = decl_str.trim().trim_end_matches(';').trim();
    let paren_open = clean.find('(').ok_or_else(|| {
        BindgenError::new(
            "Missing opening parenthesis '(' in function declaration",
            format!("Invalid prototype signature: '{}'", clean),
            "Ensure C function prototype includes '(parameter_list)'",
        )
    })?;
    let paren_close = clean.rfind(')').ok_or_else(|| {
        BindgenError::new(
            "Missing closing parenthesis ')' in function declaration",
            format!("Invalid prototype signature: '{}'", clean),
            "Ensure C function prototype includes '(parameter_list)'",
        )
    })?;

    let left = clean[..paren_open].trim();
    let params_part = clean[paren_open + 1..paren_close].trim();

    let last_space = left
        .rfind(|c: char| c.is_whitespace() || c == '*')
        .ok_or_else(|| {
            BindgenError::new(
                "Cannot separate return type from function name",
                format!("Invalid signature prefix: '{}'", left),
                "Provide both return type and identifier (e.g. 'int my_func')",
            )
        })?;

    let return_type_str = left[..=last_space].trim();
    let fn_name = left[last_space + 1..].trim().trim_start_matches('*');

    let return_type = parse_c_type_str(return_type_str)?;

    let mut params = Vec::new();
    let mut is_variadic = false;

    if !params_part.is_empty() && params_part != "void" {
        for arg in params_part.split(',') {
            let arg_clean = arg.trim();
            if arg_clean == "..." {
                is_variadic = true;
                continue;
            }

            if let Some(space_idx) = arg_clean.rfind(|c: char| c.is_whitespace() || c == '*') {
                let ty_str = arg_clean[..=space_idx].trim();
                let name_str = arg_clean[space_idx + 1..].trim().trim_start_matches('*');
                let ty = parse_c_type_str(ty_str)?;
                params.push(CParam {
                    name: name_str.to_string(),
                    ty,
                });
            } else {
                let ty = parse_c_type_str(arg_clean)?;
                params.push(CParam {
                    name: format!("arg_{}", params.len()),
                    ty,
                });
            }
        }
    }

    Ok(CFunctionDecl {
        name: fn_name.to_string(),
        return_type,
        params,
        is_variadic,
    })
}

fn parse_struct_or_union_decl(rest: &str) -> Result<(CStructDecl, &str), BindgenError> {
    let is_union = rest.starts_with("union ") || rest.starts_with("typedef union");
    let brace_open = rest.find('{').ok_or_else(|| {
        BindgenError::new(
            "Missing opening brace '{' in struct/union definition",
            "Expected '{' after struct or union keyword",
            "Check syntax for struct/union body",
        )
    })?;
    let brace_close = rest.find('}').ok_or_else(|| {
        BindgenError::new(
            "Missing closing brace '}' in struct/union definition",
            "Expected '}' at end of struct or union body",
            "Check for unmatched braces in struct definition",
        )
    })?;

    let semi_pos = rest[brace_close..].find(';').ok_or_else(|| {
        BindgenError::new(
            "Missing semicolon ';' after struct/union definition",
            "Expected ';' after struct/union closing brace",
            "Terminate struct declaration with ';'",
        )
    })? + brace_close;

    let header_prefix = rest[..brace_open].trim();
    let body = &rest[brace_open + 1..brace_close];
    let tail = rest[brace_close + 1..semi_pos].trim();

    let struct_name = if !tail.is_empty() {
        tail.to_string()
    } else {
        let tag = header_prefix
            .trim_start_matches("typedef")
            .trim_start_matches("struct")
            .trim_start_matches("union")
            .trim();
        if tag.is_empty() {
            return Err(BindgenError::new(
                "Anonymous struct/union without type alias name",
                "Struct lacks both a tag name and typedef alias",
                "Name the struct: 'struct MyStruct { ... };' or 'typedef struct { ... } MyStruct;'",
            ));
        }
        tag.to_string()
    };

    let mut fields = Vec::new();
    for field_str in body.split(';') {
        let f_clean = field_str.trim();
        if f_clean.is_empty() {
            continue;
        }

        if let Some(space_idx) = f_clean.rfind(|c: char| c.is_whitespace() || c == '*') {
            let ty_str = f_clean[..=space_idx].trim();
            let name_str = f_clean[space_idx + 1..].trim().trim_start_matches('*');
            let ty = parse_c_type_str(ty_str)?;
            fields.push(CField {
                name: name_str.to_string(),
                ty,
            });
        } else {
            return Err(BindgenError::new(
                format!("Malformed field declaration: '{}'", f_clean),
                format!("Failed to parse struct field in '{}'", struct_name),
                "Provide both type and field name (e.g. 'int x;') in struct body",
            ));
        }
    }

    Ok((
        CStructDecl {
            name: struct_name,
            fields,
            is_union,
        },
        &rest[semi_pos + 1..],
    ))
}

fn parse_enum_decl(rest: &str) -> Result<(CEnumDecl, &str), BindgenError> {
    let brace_open = rest.find('{').ok_or_else(|| {
        BindgenError::new(
            "Missing opening brace '{' in enum definition",
            "Expected '{' after enum keyword",
            "Check syntax for enum body",
        )
    })?;
    let brace_close = rest.find('}').ok_or_else(|| {
        BindgenError::new(
            "Missing closing brace '}' in enum definition",
            "Expected '}' at end of enum body",
            "Check for unmatched braces in enum definition",
        )
    })?;

    let semi_pos = rest[brace_close..].find(';').ok_or_else(|| {
        BindgenError::new(
            "Missing semicolon ';' after enum definition",
            "Expected ';' after enum closing brace",
            "Terminate enum declaration with ';'",
        )
    })? + brace_close;

    let header_prefix = rest[..brace_open].trim();
    let body = &rest[brace_open + 1..brace_close];
    let tail = rest[brace_close + 1..semi_pos].trim();

    let enum_name = if !tail.is_empty() {
        tail.to_string()
    } else {
        let tag = header_prefix
            .trim_start_matches("typedef")
            .trim_start_matches("enum")
            .trim();
        if tag.is_empty() {
            return Err(BindgenError::new(
                "Anonymous enum without type alias name",
                "Enum lacks both a tag name and typedef alias",
                "Name the enum: 'enum MyEnum { ... };' or 'typedef enum { ... } MyEnum;'",
            ));
        }
        tag.to_string()
    };

    let mut variants = Vec::new();
    let mut next_auto_value = 0i64;

    for variant_str in body.split(',') {
        let v_clean = variant_str.trim();
        if v_clean.is_empty() {
            continue;
        }

        if let Some(eq_pos) = v_clean.find('=') {
            let v_name = v_clean[..eq_pos].trim().to_string();
            let val_str = v_clean[eq_pos + 1..].trim();
            let val = val_str.parse::<i64>().map_err(|_| {
                BindgenError::new(
                    format!("Invalid integer value '{}' for enum variant '{}'", val_str, v_name),
                    format!("Failed to parse enum variant value in '{}'", enum_name),
                    "Use valid integer literals for enum variant assignments",
                )
            })?;
            next_auto_value = val + 1;
            variants.push(CEnumVariant {
                name: v_name,
                value: Some(val),
            });
        } else {
            let v_name = v_clean.to_string();
            let val = next_auto_value;
            next_auto_value += 1;
            variants.push(CEnumVariant {
                name: v_name,
                value: Some(val),
            });
        }
    }

    Ok((
        CEnumDecl {
            name: enum_name,
            variants,
        },
        &rest[semi_pos + 1..],
    ))
}

// ---------------------------------------------------------------------------
// Backward-compatibility wrappers for existing signatures
// ---------------------------------------------------------------------------

fn legacy_parse_c_type(ty_str: &str) -> Option<CPrimitive> {
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

fn legacy_c_type_to_agam(prim: CPrimitive) -> &'static str {
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

pub fn parse_c_function_prototype(line: &str) -> Option<CFuncSig> {
    let clean = line.trim().trim_end_matches(';').trim();
    let paren_start = clean.find('(')?;
    let paren_end = clean.rfind(')')?;

    let left = clean[..paren_start].trim();
    let params_part = clean[paren_start + 1..paren_end].trim();

    let last_space = left.rfind(|c: char| c.is_whitespace() || c == '*')?;
    let return_type_str = left[..=last_space].trim();
    let fn_name = left[last_space + 1..].trim();

    let return_type = legacy_parse_c_type(return_type_str)?;

    let mut params = Vec::new();
    if !params_part.is_empty() && params_part != "void" {
        for arg in params_part.split(',') {
            let arg_clean = arg.trim();
            if let Some(space_idx) = arg_clean.rfind(|c: char| c.is_whitespace() || c == '*') {
                let ty_str = arg_clean[..=space_idx].trim();
                let name_str = arg_clean[space_idx + 1..].trim();
                let prim = legacy_parse_c_type(ty_str)?;
                params.push((name_str.to_string(), prim));
            } else {
                let prim = legacy_parse_c_type(arg_clean)?;
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

pub fn generate_agam_extern_block(signatures: &[CFuncSig]) -> String {
    let mut out = String::from("extern \"C\" {\n");
    for sig in signatures {
        let params_str = sig
            .params
            .iter()
            .map(|(name, ty)| format!("{}: {}", name, legacy_c_type_to_agam(*ty)))
            .collect::<Vec<_>>()
            .join(", ");

        let ret_str = legacy_c_type_to_agam(sig.return_type);
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
    fn test_bindgen_parse_primitive_functions() {
        let header = r#"
            // Basic math and utility functions
            int calculate_sum(int a, int b);
            double compute_hypotenuse(double x, double y);
            void reset_counter(int* counter);
            int printf(const char* format, ...);
        "#;

        let parser = CHeaderParser::new(BindgenConfig {
            library_name: "libexample".to_string(),
            ..Default::default()
        });

        let result = parser
            .parse_header(header)
            .unwrap_or_else(|_| unreachable!());

        assert_eq!(result.functions.len(), 4);
        assert_eq!(result.functions[0].name, "calculate_sum");
        assert_eq!(result.functions[0].return_type, CType::Int);
        assert_eq!(result.functions[1].name, "compute_hypotenuse");
        assert_eq!(result.functions[1].return_type, CType::Double);
        assert_eq!(result.functions[2].name, "reset_counter");
        assert_eq!(result.functions[2].return_type, CType::Void);
        assert_eq!(result.functions[3].name, "printf");
        assert!(result.functions[3].is_variadic);

        let code = parser.generate_agam_code(&result);
        assert!(code.contains("foreign \"C\" from \"libexample\" {"));
        assert!(code.contains("fn calculate_sum(a: i32, b: i32) -> i32"));
        assert!(code.contains("fn compute_hypotenuse(x: f64, y: f64) -> f64"));
        assert!(code.contains("fn reset_counter(counter: ptr[i32])"));
        assert!(code.contains("fn printf(format: c_string, ...) -> i32"));
    }

    #[test]
    fn test_bindgen_parse_struct_and_union() {
        let header = r#"
            typedef struct {
                int id;
                double value;
                const char* label;
            } EntryRecord;

            typedef union {
                int as_int;
                float as_float;
            } DataPayload;
        "#;

        let parser = CHeaderParser::new(BindgenConfig::default());
        let result = parser
            .parse_header(header)
            .unwrap_or_else(|_| unreachable!());

        assert_eq!(result.structs.len(), 2);
        assert_eq!(result.structs[0].name, "EntryRecord");
        assert!(!result.structs[0].is_union);
        assert_eq!(result.structs[0].fields.len(), 3);
        assert_eq!(result.structs[0].fields[0].name, "id");
        assert_eq!(result.structs[0].fields[0].ty, CType::Int);
        assert_eq!(result.structs[0].fields[2].name, "label");
        assert_eq!(
            result.structs[0].fields[2].ty,
            CType::ConstPointer(Box::new(CType::Char))
        );

        assert_eq!(result.structs[1].name, "DataPayload");
        assert!(result.structs[1].is_union);

        let code = parser.generate_agam_code(&result);
        assert!(code.contains("type EntryRecord = struct {"));
        assert!(code.contains("id: i32,"));
        assert!(code.contains("value: f64,"));
        assert!(code.contains("label: c_string,"));
        assert!(code.contains("type DataPayload = union {"));
    }

    #[test]
    fn test_bindgen_parse_enum_variants() {
        let header = r#"
            typedef enum {
                STATUS_OK = 0,
                STATUS_PENDING = 1,
                STATUS_ERROR = 10,
                STATUS_FATAL
            } StatusCode;
        "#;

        let parser = CHeaderParser::new(BindgenConfig::default());
        let result = parser
            .parse_header(header)
            .unwrap_or_else(|_| unreachable!());

        assert_eq!(result.enums.len(), 1);
        assert_eq!(result.enums[0].name, "StatusCode");
        assert_eq!(result.enums[0].variants.len(), 4);
        assert_eq!(result.enums[0].variants[0].name, "STATUS_OK");
        assert_eq!(result.enums[0].variants[0].value, Some(0));
        assert_eq!(result.enums[0].variants[2].name, "STATUS_ERROR");
        assert_eq!(result.enums[0].variants[2].value, Some(10));
        assert_eq!(result.enums[0].variants[3].name, "STATUS_FATAL");
        assert_eq!(result.enums[0].variants[3].value, Some(11));

        let code = parser.generate_agam_code(&result);
        assert!(code.contains("type StatusCode = enum {"));
        assert!(code.contains("STATUS_OK = 0,"));
        assert!(code.contains("STATUS_FATAL = 11,"));
    }

    #[test]
    fn test_bindgen_allowlist_filtering() {
        let header = r#"
            int included_fn(int a);
            int omitted_fn(int b);
            typedef struct { int x; } IncludedType;
            typedef struct { int y; } OmittedType;
        "#;

        let config = BindgenConfig {
            allowlist_functions: vec!["included_fn".to_string()],
            allowlist_types: vec!["IncludedType".to_string()],
            ..Default::default()
        };

        let parser = CHeaderParser::new(config);
        let result = parser
            .parse_header(header)
            .unwrap_or_else(|_| unreachable!());

        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "included_fn");
        assert_eq!(result.structs.len(), 1);
        assert_eq!(result.structs[0].name, "IncludedType");

        let code = parser.generate_agam_code(&result);
        assert!(code.contains("fn included_fn"));
        assert!(!code.contains("omitted_fn"));
        assert!(code.contains("type IncludedType"));
        assert!(!code.contains("OmittedType"));
    }

    #[test]
    fn test_bindgen_invalid_syntax_returns_nyaya_error() {
        let invalid_header = "int invalid_func(int a, ;";
        let parser = CHeaderParser::new(BindgenConfig::default());
        let res = parser.parse_header(invalid_header);
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(!err.cause.is_empty());
            assert!(!err.context.is_empty());
            assert!(!err.remedy.is_empty());
        }
    }

    #[test]
    fn test_legacy_c_prototype_compatibility() {
        let proto = "int calculate_sum(int a, int b);";
        let sig = parse_c_function_prototype(proto);
        assert!(sig.is_some());
        if let Some(s) = sig {
            assert_eq!(s.name, "calculate_sum");
            assert_eq!(s.return_type, CPrimitive::I32);
        }
    }
}
