//! Procedural `@derive` Macro Generators for Traits.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeriveTrait {
    Debug,
    Clone,
    PartialEq,
    Default,
    Serialize,
    Deserialize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructDescriptor {
    pub name: String,
    pub fields: Vec<(String, String)>, // (name, type)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumDescriptor {
    pub name: String,
    pub variants: Vec<String>,
}

/// Generate trait implementation code for a struct.
pub fn generate_derive_struct(trait_kind: DeriveTrait, s: &StructDescriptor) -> String {
    match trait_kind {
        DeriveTrait::Debug => {
            let mut out = format!("impl Debug for {} {{\n", s.name);
            out.push_str("    fn fmt(self, f: &mut Formatter) -> String {\n");
            out.push_str(&format!("        let mut res = \"{} {{ \";\n", s.name));
            for (i, (fname, _)) in s.fields.iter().enumerate() {
                if i > 0 {
                    out.push_str("        res = res + \", \";\n");
                }
                out.push_str(&format!(
                    "        res = res + \"{fname}: \" + self.{fname}.to_string();\n"
                ));
            }
            out.push_str("        res + \" }\"\n");
            out.push_str("    }\n");
            out.push_str("}\n");
            out
        }
        DeriveTrait::Clone => {
            let mut out = format!("impl Clone for {} {{\n", s.name);
            out.push_str("    fn clone(self) -> Self {\n");
            out.push_str(&format!("        {} {{\n", s.name));
            for (fname, _) in &s.fields {
                out.push_str(&format!("            {fname}: self.{fname}.clone(),\n"));
            }
            out.push_str("        }\n");
            out.push_str("    }\n");
            out.push_str("}\n");
            out
        }
        DeriveTrait::PartialEq => {
            let mut out = format!("impl PartialEq for {} {{\n", s.name);
            out.push_str("    fn eq(self, other: Self) -> bool {\n");
            if s.fields.is_empty() {
                out.push_str("        true\n");
            } else {
                let comparisons: Vec<String> = s
                    .fields
                    .iter()
                    .map(|(fname, _)| format!("self.{fname} == other.{fname}"))
                    .collect();
                out.push_str(&format!("        {}\n", comparisons.join(" && ")));
            }
            out.push_str("    }\n");
            out.push_str("}\n");
            out
        }
        DeriveTrait::Default => {
            let mut out = format!("impl Default for {} {{\n", s.name);
            out.push_str("    fn default() -> Self {\n");
            out.push_str(&format!("        {} {{\n", s.name));
            for (fname, ftype) in &s.fields {
                out.push_str(&format!("            {fname}: {ftype}::default(),\n"));
            }
            out.push_str("        }\n");
            out.push_str("    }\n");
            out.push_str("}\n");
            out
        }
        DeriveTrait::Serialize | DeriveTrait::Deserialize => {
            format!("// Auto-generated serde helper for {}\n", s.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_debug_generation() {
        let s = StructDescriptor {
            name: "Point".into(),
            fields: vec![("x".into(), "f32".into()), ("y".into(), "f32".into())],
        };

        let code = generate_derive_struct(DeriveTrait::Debug, &s);
        assert!(code.contains("impl Debug for Point"));
        assert!(code.contains("self.x.to_string()"));
        assert!(code.contains("self.y.to_string()"));
    }

    #[test]
    fn test_derive_partial_eq_generation() {
        let s = StructDescriptor {
            name: "User".into(),
            fields: vec![
                ("id".into(), "u64".into()),
                ("active".into(), "bool".into()),
            ],
        };

        let code = generate_derive_struct(DeriveTrait::PartialEq, &s);
        assert!(code.contains("impl PartialEq for User"));
        assert!(code.contains("self.id == other.id && self.active == other.active"));
    }
}
