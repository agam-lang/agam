//! Plugin Architecture, Extension System, and Zero-Configuration Foreign Language Interop.
//!
//! Provides automatic multi-language foreign source discovery (.c, .cpp, .rs, .py, .js),
//! dynamic compiler lifecycle plugin hooks, and foreign FFI bridge planning.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Classification of supported foreign programming languages and native extension formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ForeignLanguageKind {
    C,
    Cpp,
    Rust,
    Python,
    JavaScript,
    NativeDylib,
}

impl ForeignLanguageKind {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "c" | "h" => Some(ForeignLanguageKind::C),
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Some(ForeignLanguageKind::Cpp),
            "rs" => Some(ForeignLanguageKind::Rust),
            "py" => Some(ForeignLanguageKind::Python),
            "js" | "mjs" => Some(ForeignLanguageKind::JavaScript),
            "dll" | "so" | "dylib" => Some(ForeignLanguageKind::NativeDylib),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ForeignLanguageKind::C => "C (ISO C11)",
            ForeignLanguageKind::Cpp => "C++ (ISO C++17)",
            ForeignLanguageKind::Rust => "Rust (cargo/rustc)",
            ForeignLanguageKind::Python => "Python (embedded CPython)",
            ForeignLanguageKind::JavaScript => "JavaScript (WASM/QuickJS)",
            ForeignLanguageKind::NativeDylib => "Pre-compiled Native Library",
        }
    }
}

/// Discovered foreign source file within the workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForeignSourceFile {
    pub language: ForeignLanguageKind,
    pub relative_path: PathBuf,
    pub absolute_path: PathBuf,
    pub is_header: bool,
}

/// Scanner that auto-detects foreign sources in `foreign/`, `src/`, `native/`, etc.
pub struct ForeignSourceScanner;

impl ForeignSourceScanner {
    /// Scan directory for all recognizable foreign source files.
    pub fn scan_directory(root: &Path) -> Vec<ForeignSourceFile> {
        let mut results = Vec::new();
        Self::scan_recursive(root, root, &mut results);
        results.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        results
    }

    fn scan_recursive(base_root: &Path, current_dir: &Path, results: &mut Vec<ForeignSourceFile>) {
        let entries = match std::fs::read_dir(current_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.starts_with('.') && name != "target" && name != "node_modules" {
                    Self::scan_recursive(base_root, &path, results);
                }
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if let Some(lang) = ForeignLanguageKind::from_extension(ext) {
                        let rel = path.strip_prefix(base_root).unwrap_or(&path).to_path_buf();
                        let is_header =
                            matches!(ext.to_ascii_lowercase().as_str(), "h" | "hpp" | "hxx");
                        results.push(ForeignSourceFile {
                            language: lang,
                            relative_path: rel,
                            absolute_path: path,
                            is_header,
                        });
                    }
                }
            }
        }
    }
}

/// Foreign Compilation Step in the build graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForeignBuildStep {
    pub step_name: String,
    pub language: ForeignLanguageKind,
    pub source_inputs: Vec<PathBuf>,
    pub output_artifact: PathBuf,
    pub compile_flags: Vec<String>,
}

/// Complete build plan for compiling and linking discovered foreign sources.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ForeignBuildPlan {
    pub steps: Vec<ForeignBuildStep>,
    pub link_libraries: Vec<String>,
    pub generated_ffi_bridges: Vec<String>,
}

/// Foreign Build Planner constructing compilation steps.
pub struct ForeignBuildPlanner;

impl ForeignBuildPlanner {
    /// Generate foreign build plan for discovered source files.
    pub fn plan(sources: &[ForeignSourceFile], build_out_dir: &Path) -> ForeignBuildPlan {
        let mut plan = ForeignBuildPlan::default();
        let mut c_sources = Vec::new();
        let mut cpp_sources = Vec::new();
        let mut rust_sources = Vec::new();

        for src in sources {
            if src.is_header {
                continue;
            }
            match src.language {
                ForeignLanguageKind::C => c_sources.push(src.absolute_path.clone()),
                ForeignLanguageKind::Cpp => cpp_sources.push(src.absolute_path.clone()),
                ForeignLanguageKind::Rust => rust_sources.push(src.absolute_path.clone()),
                ForeignLanguageKind::NativeDylib => {
                    if let Some(stem) = src.absolute_path.file_stem().and_then(|s| s.to_str()) {
                        plan.link_libraries.push(stem.to_string());
                    }
                }
                _ => {}
            }
        }

        if !c_sources.is_empty() {
            let out = build_out_dir.join("libforeign_c.a");
            plan.steps.push(ForeignBuildStep {
                step_name: "compile_foreign_c".to_string(),
                language: ForeignLanguageKind::C,
                source_inputs: c_sources,
                output_artifact: out,
                compile_flags: vec!["-O2".to_string(), "-fPIC".to_string()],
            });
            plan.link_libraries.push("foreign_c".to_string());
        }

        if !cpp_sources.is_empty() {
            let out = build_out_dir.join("libforeign_cpp.a");
            plan.steps.push(ForeignBuildStep {
                step_name: "compile_foreign_cpp".to_string(),
                language: ForeignLanguageKind::Cpp,
                source_inputs: cpp_sources,
                output_artifact: out,
                compile_flags: vec![
                    "-O2".to_string(),
                    "-std=c++17".to_string(),
                    "-fPIC".to_string(),
                ],
            });
            plan.link_libraries.push("foreign_cpp".to_string());
        }

        if !rust_sources.is_empty() {
            let out = build_out_dir.join("libforeign_rust.a");
            plan.steps.push(ForeignBuildStep {
                step_name: "compile_foreign_rust".to_string(),
                language: ForeignLanguageKind::Rust,
                source_inputs: rust_sources,
                output_artifact: out,
                compile_flags: vec![
                    "--crate-type=staticlib".to_string(),
                    "-C".to_string(),
                    "opt-level=3".to_string(),
                ],
            });
            plan.link_libraries.push("foreign_rust".to_string());
        }

        plan
    }
}

/// Compiler lifecycle hook points that plugins can intercept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PluginHookKind {
    PreLex,
    PostParse,
    PreTypeCheck,
    PostMirOptimization,
    PreCodegen,
    CustomCommand,
}

/// Metadata describing an installed or declared compiler plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompilerPlugin {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub hooks: Vec<PluginHookKind>,
    pub entry_dylib: PathBuf,
}

/// Plugin registry and execution host for `agamc`.
#[derive(Debug, Clone, Default)]
pub struct PluginHost {
    plugins: BTreeMap<String, CompilerPlugin>,
}

impl PluginHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a compiler plugin.
    pub fn register(&mut self, plugin: CompilerPlugin) {
        self.plugins.insert(plugin.name.clone(), plugin);
    }

    /// Retrieve all registered plugins.
    pub fn list_plugins(&self) -> Vec<&CompilerPlugin> {
        self.plugins.values().collect()
    }

    /// Query plugins subscribed to a specific lifecycle hook.
    pub fn get_hook_subscribers(&self, hook: PluginHookKind) -> Vec<&CompilerPlugin> {
        self.plugins
            .values()
            .filter(|p| p.hooks.contains(&hook))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_foreign_language_classification() {
        assert_eq!(
            ForeignLanguageKind::from_extension("c"),
            Some(ForeignLanguageKind::C)
        );
        assert_eq!(
            ForeignLanguageKind::from_extension("cpp"),
            Some(ForeignLanguageKind::Cpp)
        );
        assert_eq!(
            ForeignLanguageKind::from_extension("rs"),
            Some(ForeignLanguageKind::Rust)
        );
        assert_eq!(
            ForeignLanguageKind::from_extension("py"),
            Some(ForeignLanguageKind::Python)
        );
        assert_eq!(
            ForeignLanguageKind::from_extension("dylib"),
            Some(ForeignLanguageKind::NativeDylib)
        );
        assert_eq!(ForeignLanguageKind::from_extension("unknown"), None);
    }

    #[test]
    fn test_foreign_build_planner() {
        let sources = vec![
            ForeignSourceFile {
                language: ForeignLanguageKind::C,
                relative_path: PathBuf::from("native/fast_simd.c"),
                absolute_path: PathBuf::from("/workspace/native/fast_simd.c"),
                is_header: false,
            },
            ForeignSourceFile {
                language: ForeignLanguageKind::C,
                relative_path: PathBuf::from("native/fast_simd.h"),
                absolute_path: PathBuf::from("/workspace/native/fast_simd.h"),
                is_header: true,
            },
            ForeignSourceFile {
                language: ForeignLanguageKind::Rust,
                relative_path: PathBuf::from("native/tensor_core.rs"),
                absolute_path: PathBuf::from("/workspace/native/tensor_core.rs"),
                is_header: false,
            },
        ];

        let out_dir = PathBuf::from("/workspace/target/foreign");
        let plan = ForeignBuildPlanner::plan(&sources, &out_dir);

        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].language, ForeignLanguageKind::C);
        assert_eq!(plan.steps[1].language, ForeignLanguageKind::Rust);
        assert!(plan.link_libraries.contains(&"foreign_c".to_string()));
        assert!(plan.link_libraries.contains(&"foreign_rust".to_string()));
    }

    #[test]
    fn test_plugin_host_registration_and_hook_dispatch() {
        let mut host = PluginHost::new();

        let linter_plugin = CompilerPlugin {
            name: "agam-sec-linter".to_string(),
            version: "1.0.0".to_string(),
            author: "Security Team".to_string(),
            description: "Advanced AST taint tracking plugin".to_string(),
            hooks: vec![PluginHookKind::PostParse, PluginHookKind::PreTypeCheck],
            entry_dylib: PathBuf::from("/plugins/libsec_linter.so"),
        };

        host.register(linter_plugin);
        assert_eq!(host.list_plugins().len(), 1);

        let parse_subscribers = host.get_hook_subscribers(PluginHookKind::PostParse);
        assert_eq!(parse_subscribers.len(), 1);
        assert_eq!(parse_subscribers[0].name, "agam-sec-linter");

        let codegen_subscribers = host.get_hook_subscribers(PluginHookKind::PreCodegen);
        assert!(codegen_subscribers.is_empty());
    }
}
