//! # agam_doc
//!
//! Documentation generator and doctest runner for the Agam language.
//!
//! Generates rich HTML documentation with responsive sidebars, client-side search,
//! syntax-highlighted code blocks, and executes markdown code examples as doctests.

pub mod doctest;
pub mod extract;
pub mod html;
pub mod json;
pub mod model;

use agam_errors::span::SourceId;
use agam_lexer::tokenize;
use agam_parser::Parser;
use std::path::Path;

pub use doctest::{DoctestReport, run_package_doctests};
pub use extract::{extract_module, extract_package};
pub use html::generate_html;
pub use json::generate_json;
pub use model::{DocItem, DocModule, DocPackage};

/// Build a `DocPackage` from source code string.
pub fn build_doc_package_from_source(
    pkg_name: &str,
    version: &str,
    description: Option<&str>,
    source: &str,
) -> Result<DocPackage, String> {
    let tokens = tokenize(source, SourceId(0));
    let mut parser = Parser::new(tokens);
    let module = parser.parse_module(SourceId(0)).map_err(|errs| {
        errs.iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    Ok(extract_package(pkg_name, version, description, &module))
}

/// Generate HTML documentation from an Agam source string into output directory.
pub fn generate_docs_from_source(
    pkg_name: &str,
    version: &str,
    description: Option<&str>,
    source: &str,
    out_dir: &Path,
) -> Result<(), String> {
    let pkg = build_doc_package_from_source(pkg_name, version, description, source)?;
    generate_html(&pkg, out_dir).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_extraction_and_html_generation() {
        let src = r#"//! Core mathematical utilities.
//! High performance vector and matrix routines.

/// Computes the Euclidean norm of a 2D vector.
///
/// ```agam
/// let norm = euclidean_norm(3.0, 4.0);
/// ```
fn euclidean_norm(x: f64, y: f64) -> f64 { return 5.0 }

/// 2D Cartesian Coordinate.
struct Vec2 { x: f64, y: f64 }

/// Status outcomes.
enum Status { Success, Failure }
"#;
        let pkg =
            build_doc_package_from_source("math_core", "1.0.0", Some("Math Core Library"), src)
                .unwrap();
        assert_eq!(pkg.name, "math_core");
        assert_eq!(pkg.root_module.docs.len(), 2);
        assert_eq!(pkg.root_module.items.len(), 3);

        // Verify doctest extraction
        let report = run_package_doctests(&pkg);
        assert_eq!(report.total, 1);
        assert_eq!(report.passed, 1);

        // Verify JSON generation
        let json = generate_json(&pkg).unwrap();
        assert!(json.contains("euclidean_norm"));
        assert!(json.contains("Vec2"));
        assert!(json.contains("Status"));

        // Verify HTML generation
        let temp_dir = std::env::temp_dir().join("agam_doc_test");
        generate_html(&pkg, &temp_dir).unwrap();
        assert!(temp_dir.join("index.html").exists());
        assert!(temp_dir.join("style.css").exists());
        assert!(temp_dir.join("script.js").exists());
        assert!(temp_dir.join("search-index.js").exists());
        assert!(temp_dir.join("fn.euclidean_norm.html").exists());
        assert!(temp_dir.join("struct.Vec2.html").exists());
        assert!(temp_dir.join("enum.Status.html").exists());

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
