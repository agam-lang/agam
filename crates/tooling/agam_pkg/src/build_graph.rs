//! Directed Acyclic Graph (DAG) Compilation Scheduler and Parallel Build Engine.
//!
//! Provides deterministic wave scheduling, critical-path analysis, and
//! intra-workspace concurrent compilation coordination for multi-crate builds.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Target compilation artifact kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildTargetKind {
    Binary,
    StaticLib,
    SharedLib,
    GpuKernel,
    WasmModule,
}

/// Compilation stage within a single build task.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum BuildStage {
    #[default]
    Source,
    Parsed,
    TypeChecked,
    MirLowered,
    CodeGenerated,
    Linked,
}

/// A node in the compilation dependency graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildNode {
    pub id: String,
    pub dependencies: Vec<String>,
    pub stage: BuildStage,
    pub target_kind: BuildTargetKind,
    pub estimated_cost_ms: u64,
}

impl BuildNode {
    pub fn new(id: impl Into<String>, target_kind: BuildTargetKind) -> Self {
        Self {
            id: id.into(),
            dependencies: Vec::new(),
            stage: BuildStage::Source,
            target_kind,
            estimated_cost_ms: 100,
        }
    }

    pub fn with_dependency(mut self, dep_id: impl Into<String>) -> Self {
        self.dependencies.push(dep_id.into());
        self
    }

    pub fn with_cost(mut self, cost_ms: u64) -> Self {
        self.estimated_cost_ms = cost_ms;
        self
    }
}

/// Dependency errors in the compilation graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildGraphError {
    CycleDetected(Vec<String>),
    NodeNotFound(String),
    EmptyGraph,
}

impl std::fmt::Display for BuildGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CycleDetected(cycle) => {
                write!(
                    f,
                    "cyclic build dependency detected: {}",
                    cycle.join(" -> ")
                )
            }
            Self::NodeNotFound(id) => write!(f, "build node '{id}' not found in graph"),
            Self::EmptyGraph => write!(f, "build graph is empty"),
        }
    }
}

impl std::error::Error for BuildGraphError {}

/// Directed Acyclic Graph of compilation units.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildGraph {
    pub nodes: HashMap<String, BuildNode>,
}

impl BuildGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: BuildNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_dependency(
        &mut self,
        node_id: &str,
        depends_on: &str,
    ) -> Result<(), BuildGraphError> {
        if !self.nodes.contains_key(depends_on) {
            return Err(BuildGraphError::NodeNotFound(depends_on.to_string()));
        }
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| BuildGraphError::NodeNotFound(node_id.to_string()))?;
        if !node.dependencies.contains(&depends_on.to_string()) {
            node.dependencies.push(depends_on.to_string());
        }
        Ok(())
    }

    /// Compute parallel execution waves (tiers) via Kahn's topological sort.
    /// All tasks in a single wave can be executed concurrently without dependencies.
    pub fn topological_waves(&self) -> Result<Vec<Vec<String>>, BuildGraphError> {
        if self.nodes.is_empty() {
            return Ok(Vec::new());
        }

        let mut in_degrees: HashMap<String, usize> = HashMap::new();
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

        for (id, node) in &self.nodes {
            in_degrees.entry(id.clone()).or_insert(0);
            for dep in &node.dependencies {
                if !self.nodes.contains_key(dep) {
                    return Err(BuildGraphError::NodeNotFound(dep.clone()));
                }
                *in_degrees.entry(id.clone()).or_insert(0) += 1;
                dependents.entry(dep.clone()).or_default().push(id.clone());
            }
        }

        let mut current_wave: Vec<String> = in_degrees
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();
        current_wave.sort();

        let mut waves = Vec::new();
        let mut visited_count = 0;

        while !current_wave.is_empty() {
            visited_count += current_wave.len();
            let mut next_wave_candidates = Vec::new();

            for completed_node in &current_wave {
                if let Some(deps) = dependents.get(completed_node) {
                    for dep in deps {
                        if let Some(deg) = in_degrees.get_mut(dep) {
                            *deg = deg.saturating_sub(1);
                            if *deg == 0 {
                                next_wave_candidates.push(dep.clone());
                            }
                        }
                    }
                }
            }

            waves.push(current_wave);
            next_wave_candidates.sort();
            next_wave_candidates.dedup();
            current_wave = next_wave_candidates;
        }

        if visited_count != self.nodes.len() {
            let cycle_nodes: Vec<String> = in_degrees
                .iter()
                .filter(|&(_, &deg)| deg > 0)
                .map(|(id, _)| id.clone())
                .collect();
            return Err(BuildGraphError::CycleDetected(cycle_nodes));
        }

        Ok(waves)
    }

    /// Calculate the critical path (longest duration sequence) through the build graph.
    pub fn critical_path(&self) -> (Vec<String>, u64) {
        let waves = match self.topological_waves() {
            Ok(w) => w,
            Err(_) => return (Vec::new(), 0),
        };

        let mut longest_dist: HashMap<String, (u64, Vec<String>)> = HashMap::new();

        for wave in &waves {
            for node_id in wave {
                let cost = self
                    .nodes
                    .get(node_id)
                    .map(|n| n.estimated_cost_ms)
                    .unwrap_or(0);
                let deps = self.nodes.get(node_id).map(|n| &n.dependencies);

                let (max_prev_cost, max_prev_path) = deps
                    .map(|d| {
                        d.iter()
                            .filter_map(|dep| longest_dist.get(dep))
                            .max_by_key(|(c, _)| *c)
                            .cloned()
                            .unwrap_or((0, Vec::new()))
                    })
                    .unwrap_or((0, Vec::new()));

                let mut current_path = max_prev_path;
                current_path.push(node_id.clone());
                longest_dist.insert(node_id.clone(), (max_prev_cost + cost, current_path));
            }
        }

        let (cost, path) = longest_dist
            .into_values()
            .max_by_key(|(cost, _)| *cost)
            .unwrap_or((0, Vec::new()));

        (path, cost)
    }

    /// Maximum degree of concurrent compilation possible in this graph.
    pub fn max_parallelism(&self) -> usize {
        self.topological_waves()
            .map(|w| w.iter().map(|tier| tier.len()).max().unwrap_or(0))
            .unwrap_or(0)
    }
}

/// Execution schedule report generated by the parallel build engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildScheduleReport {
    pub total_nodes: usize,
    pub wave_count: usize,
    pub max_parallelism: usize,
    pub critical_path_duration_ms: u64,
    pub critical_path: Vec<String>,
    pub waves: Vec<Vec<String>>,
}

/// Parallel build scheduler and worker dispatcher.
pub struct ParallelBuildScheduler {
    pub concurrency: usize,
}

impl ParallelBuildScheduler {
    pub fn new(concurrency: usize) -> Self {
        Self {
            concurrency: if concurrency == 0 { 1 } else { concurrency },
        }
    }

    pub fn schedule(&self, graph: &BuildGraph) -> Result<BuildScheduleReport, BuildGraphError> {
        let waves = graph.topological_waves()?;
        let (crit_path, crit_cost) = graph.critical_path();
        let max_p = graph.max_parallelism();

        Ok(BuildScheduleReport {
            total_nodes: graph.nodes.len(),
            wave_count: waves.len(),
            max_parallelism: max_p,
            critical_path_duration_ms: crit_cost,
            critical_path: crit_path,
            waves,
        })
    }
}

impl Default for ParallelBuildScheduler {
    fn default() -> Self {
        Self::new(num_cpus())
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_wave_scheduling_and_critical_path() {
        // Build graph:
        // A -> B -> D
        // A -> C -> D
        let mut graph = BuildGraph::new();
        graph.add_node(BuildNode::new("A", BuildTargetKind::StaticLib).with_cost(50));
        graph.add_node(
            BuildNode::new("B", BuildTargetKind::StaticLib)
                .with_dependency("A")
                .with_cost(100),
        );
        graph.add_node(
            BuildNode::new("C", BuildTargetKind::StaticLib)
                .with_dependency("A")
                .with_cost(150),
        );
        graph.add_node(
            BuildNode::new("D", BuildTargetKind::Binary)
                .with_dependency("B")
                .with_dependency("C")
                .with_cost(80),
        );

        let scheduler = ParallelBuildScheduler::new(4);
        let report = scheduler.schedule(&graph).expect("schedule succeeds");

        assert_eq!(report.total_nodes, 4);
        assert_eq!(report.wave_count, 3);
        assert_eq!(report.waves[0], vec!["A"]);
        assert_eq!(report.waves[1], vec!["B", "C"]);
        assert_eq!(report.waves[2], vec!["D"]);
        assert_eq!(report.max_parallelism, 2);

        // Critical path: A (50) + C (150) + D (80) = 280ms
        assert_eq!(report.critical_path_duration_ms, 280);
        assert_eq!(report.critical_path, vec!["A", "C", "D"]);
    }

    #[test]
    fn test_cycle_detection_error() {
        let mut graph = BuildGraph::new();
        graph.add_node(BuildNode::new("X", BuildTargetKind::StaticLib).with_dependency("Y"));
        graph.add_node(BuildNode::new("Y", BuildTargetKind::StaticLib).with_dependency("X"));

        let res = graph.topological_waves();
        assert!(matches!(res, Err(BuildGraphError::CycleDetected(_))));
    }
}
