//! Link-Time Whole-Program Optimization, Cross-Module DCE & Multi-Target Fat-Binary Bundling.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const AGAM_FATBIN_MAGIC: u32 = 0x4147414D; // "AGAM"
pub const AGAM_FATBIN_VERSION: u32 = 1;

/// Target architecture classification for multi-device fat binaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetArtifactKind {
    HostX86_64,
    HostAarch64,
    NvptxPtx,
    NvptxCubin,
    SpirvBinary,
    WasmBinary,
}

/// A single compiled binary artifact entry within a fat binary container.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FatBinaryEntry {
    pub target: TargetArtifactKind,
    pub arch_name: String,
    pub payload: Vec<u8>,
}

/// Multi-Target Fat-Binary container bundle.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FatBinaryBundle {
    pub entries: Vec<FatBinaryEntry>,
}

impl FatBinaryBundle {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_artifact(
        &mut self,
        target: TargetArtifactKind,
        arch_name: impl Into<String>,
        payload: Vec<u8>,
    ) {
        self.entries.push(FatBinaryEntry {
            target,
            arch_name: arch_name.into(),
            payload,
        });
    }

    /// Serialize fat-binary container with header directory into bytes.
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        // Magic + Version + Entry Count
        bytes.extend_from_slice(&AGAM_FATBIN_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&AGAM_FATBIN_VERSION.to_le_bytes());
        bytes.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());

        // Header Table
        for entry in &self.entries {
            let target_id = entry.target as u32;
            bytes.extend_from_slice(&target_id.to_le_bytes());
            let arch_bytes = entry.arch_name.as_bytes();
            bytes.extend_from_slice(&(arch_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(arch_bytes);
            bytes.extend_from_slice(&(entry.payload.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&entry.payload);
        }

        bytes
    }

    /// Locate payload matching the specified target kind.
    pub fn find_payload(&self, target: TargetArtifactKind) -> Option<&[u8]> {
        self.entries
            .iter()
            .find(|e| e.target == target)
            .map(|e| e.payload.as_slice())
    }
}

/// Function summary descriptor for cross-module link-time analysis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionSummary {
    pub name: String,
    pub module_name: String,
    pub is_exported: bool,
    pub is_root: bool, // e.g. main or @gpu kernel entry
    pub callees: HashSet<String>,
}

/// Cross-module whole-program summary index for distributed dead code elimination and inlining.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleSummaryIndex {
    pub functions: HashMap<String, FunctionSummary>,
}

impl ModuleSummaryIndex {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    pub fn register_function(&mut self, summary: FunctionSummary) {
        self.functions.insert(summary.name.clone(), summary);
    }

    /// Compute reachable functions from root entry points using transitive closure.
    pub fn compute_reachable_functions(&self) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut worklist: Vec<String> = self
            .functions
            .values()
            .filter(|f| f.is_root || f.is_exported)
            .map(|f| f.name.clone())
            .collect();

        for root in &worklist {
            reachable.insert(root.clone());
        }

        while let Some(current) = worklist.pop() {
            if let Some(func_summary) = self.functions.get(&current) {
                for callee in &func_summary.callees {
                    if reachable.insert(callee.clone()) {
                        worklist.push(callee.clone());
                    }
                }
            }
        }

        reachable
    }

    /// Perform whole-program Dead Code Elimination (DCE), returning eliminated functions.
    pub fn eliminate_dead_functions(&mut self) -> Vec<String> {
        let reachable = self.compute_reachable_functions();
        let dead: Vec<String> = self
            .functions
            .keys()
            .filter(|name| !reachable.contains(*name))
            .cloned()
            .collect();

        for dead_name in &dead {
            self.functions.remove(dead_name);
        }

        dead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fat_binary_bundle_serialization_and_lookup() {
        let mut bundle = FatBinaryBundle::new();
        bundle.add_artifact(
            TargetArtifactKind::HostX86_64,
            "x86_64-pc-windows-msvc",
            vec![0x4D, 0x5A, 0x90, 0x00], // MZ executable header
        );
        bundle.add_artifact(
            TargetArtifactKind::SpirvBinary,
            "spirv64",
            vec![0x03, 0x02, 0x23, 0x07], // SPIR-V magic header
        );

        let bytes = bundle.serialize();
        assert!(!bytes.is_empty());

        let host_payload = bundle.find_payload(TargetArtifactKind::HostX86_64);
        assert_eq!(host_payload, Some(&[0x4D, 0x5A, 0x90, 0x00][..]));

        let spirv_payload = bundle.find_payload(TargetArtifactKind::SpirvBinary);
        assert_eq!(spirv_payload, Some(&[0x03, 0x02, 0x23, 0x07][..]));
    }

    #[test]
    fn test_cross_module_dce_reachability() {
        let mut index = ModuleSummaryIndex::new();

        index.register_function(FunctionSummary {
            name: "main".into(),
            module_name: "app".into(),
            is_exported: false,
            is_root: true,
            callees: ["helper_a".into()].into_iter().collect(),
        });

        index.register_function(FunctionSummary {
            name: "helper_a".into(),
            module_name: "lib_math".into(),
            is_exported: false,
            is_root: false,
            callees: ["helper_b".into()].into_iter().collect(),
        });

        index.register_function(FunctionSummary {
            name: "helper_b".into(),
            module_name: "lib_core".into(),
            is_exported: false,
            is_root: false,
            callees: HashSet::new(),
        });

        index.register_function(FunctionSummary {
            name: "unused_function".into(),
            module_name: "lib_math".into(),
            is_exported: false,
            is_root: false,
            callees: HashSet::new(),
        });

        let reachable = index.compute_reachable_functions();
        assert!(reachable.contains("main"));
        assert!(reachable.contains("helper_a"));
        assert!(reachable.contains("helper_b"));
        assert!(!reachable.contains("unused_function"));

        let dead = index.eliminate_dead_functions();
        assert_eq!(dead, vec!["unused_function"]);
        assert_eq!(index.functions.len(), 3);
    }
}
