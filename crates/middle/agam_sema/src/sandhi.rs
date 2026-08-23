//! Type Sandhi & Trait Bound Representation Lattice Graph (`agam_sema::sandhi`).
//!
//! Provides mathematical lattice representation for composite trait bounds
//! (`TraitA + TraitB`), supertrait transitive closure compilation, and $O(1)$
//! constraint satisfaction queries for generic type instantiation.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::symbol::{SymbolId, TypeId};
use crate::traits::TraitRegistry;
use agam_smt::solver::SmtSolver;

/// A canonical trait bound conjunction representing a node in the trait lattice.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraitLattice {
    /// Canonical sorted set of trait symbols that must be satisfied.
    pub traits: BTreeSet<SymbolId>,
}

impl TraitLattice {
    /// An unconstrained lattice element (universal/top element: no requirements).
    pub fn empty() -> Self {
        Self {
            traits: BTreeSet::new(),
        }
    }

    /// Construct a lattice element from a slice of trait IDs.
    pub fn from_traits(traits: &[SymbolId]) -> Self {
        let mut set = BTreeSet::new();
        for &t in traits {
            set.insert(t);
        }
        Self { traits: set }
    }

    /// Add a trait to this conjunction.
    pub fn with_trait(mut self, trait_id: SymbolId) -> Self {
        self.traits.insert(trait_id);
        self
    }

    /// Meet operation ($\land$ / union of requirements):
    /// Resulting bound requires satisfying BOTH sets of traits.
    pub fn meet(&self, other: &TraitLattice) -> TraitLattice {
        let mut combined = self.traits.clone();
        for &t in &other.traits {
            combined.insert(t);
        }
        TraitLattice { traits: combined }
    }

    /// Join operation ($\lor$ / intersection of common requirements):
    /// Resulting bound requires only the traits shared by both.
    pub fn join(&self, other: &TraitLattice) -> TraitLattice {
        let intersection: BTreeSet<SymbolId> =
            self.traits.intersection(&other.traits).copied().collect();
        TraitLattice {
            traits: intersection,
        }
    }

    /// Subsumption check: Does `self` subsume `other`?
    /// `self` subsumes `other` if any type satisfying `other` also satisfies `self`
    /// (i.e. `self.traits` is a subset of `other.traits`).
    pub fn is_subsumed_by(&self, other: &TraitLattice) -> bool {
        self.traits.is_subset(&other.traits)
    }

    /// Number of trait requirements in this lattice element.
    pub fn len(&self) -> usize {
        self.traits.len()
    }

    /// Check if there are no trait requirements.
    pub fn is_empty(&self) -> bool {
        self.traits.is_empty()
    }
}

/// A node in the trait inheritance directed graph.
#[derive(Debug, Clone)]
pub struct SandhiNode {
    pub trait_id: SymbolId,
    pub name: String,
    pub direct_supertraits: Vec<SymbolId>,
    /// Full transitive closure of all supertraits implied by this trait.
    pub transitive_supertraits: BTreeSet<SymbolId>,
}

/// The Type Sandhi representation harmonic lattice graph.
#[derive(Debug, Clone, Default)]
pub struct SandhiGraph {
    /// Trait nodes indexed by SymbolId.
    pub nodes: HashMap<SymbolId, SandhiNode>,
    /// Cached transitive trait implementations per concrete TypeId.
    pub type_impls: HashMap<TypeId, BTreeSet<SymbolId>>,
}

impl SandhiGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the Sandhi Graph from a populated `TraitRegistry`.
    pub fn build_from_registry(&mut self, registry: &TraitRegistry) {
        self.nodes.clear();
        self.type_impls.clear();

        // 1. Register all trait nodes with direct supertraits.
        for (&sym, def) in &registry.traits {
            self.nodes.insert(
                sym,
                SandhiNode {
                    trait_id: sym,
                    name: def.name.clone(),
                    direct_supertraits: def.super_traits.clone(),
                    transitive_supertraits: BTreeSet::new(),
                },
            );
        }

        // 2. Compute transitive closure of supertraits for each node.
        let all_trait_ids: Vec<SymbolId> = self.nodes.keys().copied().collect();
        for trait_id in all_trait_ids {
            let mut visited = BTreeSet::new();
            let mut stack = Vec::new();
            if let Some(node) = self.nodes.get(&trait_id) {
                stack.extend(node.direct_supertraits.iter().copied());
            }

            while let Some(current) = stack.pop() {
                if visited.insert(current)
                    && let Some(parent) = self.nodes.get(&current)
                {
                    stack.extend(parent.direct_supertraits.iter().copied());
                }
            }

            if let Some(node) = self.nodes.get_mut(&trait_id) {
                node.transitive_supertraits = visited;
            }
        }

        // 3. Register impl blocks and compute transitive traits for each concrete type.
        for imp in &registry.impls {
            if let Some(trait_id) = imp.trait_id {
                let entry = self.type_impls.entry(imp.target_type).or_default();
                entry.insert(trait_id);

                // Add all transitive supertraits of the implemented trait
                if let Some(node) = self.nodes.get(&trait_id) {
                    for &super_t in &node.transitive_supertraits {
                        entry.insert(super_t);
                    }
                }
            }
        }
    }

    /// $O(1)$ Query: Check if a concrete type satisfies a set of trait bounds.
    pub fn satisfies_bounds(&self, target_type: TypeId, bounds: &[SymbolId]) -> bool {
        if bounds.is_empty() {
            return true;
        }

        if let Some(implemented) = self.type_impls.get(&target_type) {
            for &bound in bounds {
                if !implemented.contains(&bound) {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// $O(1)$ Query: Check if a concrete type satisfies a `TraitLattice` constraint.
    pub fn satisfies_lattice(&self, target_type: TypeId, lattice: &TraitLattice) -> bool {
        if lattice.is_empty() {
            return true;
        }

        if let Some(implemented) = self.type_impls.get(&target_type) {
            lattice.traits.is_subset(implemented)
        } else {
            false
        }
    }

    /// Detect any cyclic dependencies in the supertrait hierarchy.
    pub fn detect_cycles(&self) -> Vec<Vec<SymbolId>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();
        let mut current_path = Vec::new();

        for &trait_id in self.nodes.keys() {
            if !visited.contains(&trait_id) {
                self.dfs_cycle(
                    trait_id,
                    &mut visited,
                    &mut in_stack,
                    &mut current_path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn dfs_cycle(
        &self,
        node: SymbolId,
        visited: &mut HashSet<SymbolId>,
        in_stack: &mut HashSet<SymbolId>,
        path: &mut Vec<SymbolId>,
        cycles: &mut Vec<Vec<SymbolId>>,
    ) {
        visited.insert(node);
        in_stack.insert(node);
        path.push(node);

        if let Some(sandhi_node) = self.nodes.get(&node) {
            for &super_t in &sandhi_node.direct_supertraits {
                if !visited.contains(&super_t) {
                    self.dfs_cycle(super_t, visited, in_stack, path, cycles);
                } else if in_stack.contains(&super_t) {
                    // Cycle found
                    if let Some(pos) = path.iter().position(|&x| x == super_t) {
                        let cycle = path[pos..].to_vec();
                        cycles.push(cycle);
                    }
                }
            }
        }

        path.pop();
        in_stack.remove(&node);
    }

    /// Find common supertrait ancestors of two traits in the lattice.
    pub fn find_common_ancestors(&self, trait_a: SymbolId, trait_b: SymbolId) -> Vec<SymbolId> {
        let node_a = match self.nodes.get(&trait_a) {
            Some(n) => n,
            None => return Vec::new(),
        };
        let node_b = match self.nodes.get(&trait_b) {
            Some(n) => n,
            None => return Vec::new(),
        };

        let mut a_all = node_a.transitive_supertraits.clone();
        a_all.insert(trait_a);

        let mut b_all = node_b.transitive_supertraits.clone();
        b_all.insert(trait_b);

        a_all.intersection(&b_all).copied().collect()
    }
}

/// SMT-backed verification engine for global Type Sandhi lattice coherence.
#[derive(Debug, Clone, Copy, Default)]
pub struct SandhiSmtVerifier;

impl SandhiSmtVerifier {
    /// Verify global trait lattice coherence and supertrait acyclicity using SMT logic.
    pub fn verify_coherence(graph: &SandhiGraph) -> bool {
        let mut solver = agam_smt::solver::Z3Solver::new();

        // 1. Declare an integer rank variable for each trait symbol
        for &sym in graph.nodes.keys() {
            solver.declare_int(&format!("trait_rank_{}", sym.0));
        }

        // 2. For every direct supertrait edge Child -> Parent, assert rank(Child) > rank(Parent)
        for (&sym, node) in &graph.nodes {
            for &super_id in &node.direct_supertraits {
                solver.assert(agam_smt::solver::Constraint::Gt(
                    Box::new(agam_smt::solver::Constraint::Var(format!(
                        "trait_rank_{}",
                        sym.0
                    ))),
                    Box::new(agam_smt::solver::Constraint::Var(format!(
                        "trait_rank_{}",
                        super_id.0
                    ))),
                ));
            }
        }

        // 3. Check SAT: If SAT, there exists a valid non-cyclic topological ranking
        match solver.check_sat() {
            agam_smt::solver::SolverResult::Sat => true,
            agam_smt::solver::SolverResult::Unsat | agam_smt::solver::SolverResult::Unknown => {
                false
            }
        }
    }
}

/// Maximum allowed depth of nested generic monomorphization to prevent infinite expansion.
pub const MAX_MONOMORPHIZATION_DEPTH: usize = 64;
/// Maximum total instantiations per compilation unit.
pub const MAX_INSTANTIATION_COUNT: usize = 4096;

/// An instantiated node in the monomorphization dependency graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonomorphizationNode {
    pub instance_id: String,
    pub symbol_id: SymbolId,
    pub type_args: Vec<TypeId>,
    pub callers: Vec<String>,
    pub callees: Vec<String>,
    pub depth: usize,
}

/// Errors occurring during polymorphic monomorphization graph resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonomorphizationError {
    RecursionLimitExceeded { instance_id: String, depth: usize },
    InstantiationLimitExceeded { count: usize },
    CycleDetected { cycle: Vec<String> },
}

/// Directed graph managing concrete instantiations of generic definitions.
#[derive(Debug, Clone, Default)]
pub struct MonomorphizationGraph {
    pub instances: HashMap<String, MonomorphizationNode>,
    pub roots: Vec<String>,
}

impl MonomorphizationGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute canonical instance identifier string from SymbolId and concrete TypeIds.
    pub fn instance_id(symbol_id: SymbolId, type_args: &[TypeId]) -> String {
        let args_str: Vec<String> = type_args.iter().map(|t| format!("T{}", t.0)).collect();
        format!("sym_{}_{}", symbol_id.0, args_str.join("_"))
    }

    /// Register a root entry-point function or struct instantiation.
    pub fn add_root(
        &mut self,
        symbol_id: SymbolId,
        type_args: Vec<TypeId>,
    ) -> Result<String, MonomorphizationError> {
        let id = Self::instance_id(symbol_id, &type_args);
        if !self.instances.contains_key(&id) {
            if self.instances.len() >= MAX_INSTANTIATION_COUNT {
                return Err(MonomorphizationError::InstantiationLimitExceeded {
                    count: self.instances.len(),
                });
            }
            self.instances.insert(
                id.clone(),
                MonomorphizationNode {
                    instance_id: id.clone(),
                    symbol_id,
                    type_args,
                    callers: Vec::new(),
                    callees: Vec::new(),
                    depth: 0,
                },
            );
            self.roots.push(id.clone());
        }
        Ok(id)
    }

    /// Record an instantiation invoked by an existing caller.
    pub fn instantiate(
        &mut self,
        caller_id: Option<&str>,
        symbol_id: SymbolId,
        type_args: Vec<TypeId>,
    ) -> Result<String, MonomorphizationError> {
        let id = Self::instance_id(symbol_id, &type_args);

        let caller_depth = if let Some(caller) = caller_id {
            self.instances.get(caller).map(|n| n.depth).unwrap_or(0)
        } else {
            0
        };

        let next_depth = caller_depth + 1;
        if next_depth > MAX_MONOMORPHIZATION_DEPTH {
            return Err(MonomorphizationError::RecursionLimitExceeded {
                instance_id: id,
                depth: next_depth,
            });
        }

        if let Some(node) = self.instances.get_mut(&id) {
            if let Some(caller) = caller_id
                && !node.callers.iter().any(|c| c == caller)
            {
                node.callers.push(caller.to_string());
            }
        } else {
            if self.instances.len() >= MAX_INSTANTIATION_COUNT {
                return Err(MonomorphizationError::InstantiationLimitExceeded {
                    count: self.instances.len(),
                });
            }

            let callers = if let Some(caller) = caller_id {
                vec![caller.to_string()]
            } else {
                Vec::new()
            };

            self.instances.insert(
                id.clone(),
                MonomorphizationNode {
                    instance_id: id.clone(),
                    symbol_id,
                    type_args,
                    callers,
                    callees: Vec::new(),
                    depth: next_depth,
                },
            );
        }

        if let Some(caller) = caller_id
            && let Some(caller_node) = self.instances.get_mut(caller)
            && !caller_node.callees.iter().any(|c| c == &id)
        {
            caller_node.callees.push(id.clone());
        }

        Ok(id)
    }

    /// Compute reverse topological monomorphization order (leaves first, roots last).
    pub fn compute_compilation_order(&self) -> Result<Vec<String>, MonomorphizationError> {
        let mut in_degrees: HashMap<&str, usize> = HashMap::new();
        for id in self.instances.keys() {
            in_degrees.insert(id.as_str(), 0);
        }

        for node in self.instances.values() {
            for callee in &node.callees {
                if let Some(entry) = in_degrees.get_mut(callee.as_str()) {
                    *entry += 1;
                }
            }
        }

        let mut queue: Vec<&str> = in_degrees
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order = Vec::new();
        while let Some(current) = queue.pop() {
            order.push(current.to_string());
            if let Some(node) = self.instances.get(current) {
                for callee in &node.callees {
                    if let Some(deg) = in_degrees.get_mut(callee.as_str()) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push(callee.as_str());
                        }
                    }
                }
            }
        }

        if order.len() < self.instances.len() {
            return Err(MonomorphizationError::CycleDetected { cycle: order });
        }

        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{ImplEntry, TraitDef};
    use agam_errors::Span;

    #[test]
    fn test_trait_lattice_meet_join() {
        let t1 = SymbolId(1);
        let t2 = SymbolId(2);
        let t3 = SymbolId(3);

        let lat_a = TraitLattice::from_traits(&[t1, t2]);
        let lat_b = TraitLattice::from_traits(&[t2, t3]);

        // Meet = union of constraints (t1, t2, t3)
        let meet = lat_a.meet(&lat_b);
        assert_eq!(meet.len(), 3);
        assert!(meet.traits.contains(&t1));
        assert!(meet.traits.contains(&t2));
        assert!(meet.traits.contains(&t3));

        // Join = common constraints (t2)
        let join = lat_a.join(&lat_b);
        assert_eq!(join.len(), 1);
        assert!(join.traits.contains(&t2));

        // Subsumption
        assert!(join.is_subsumed_by(&lat_a));
        assert!(join.is_subsumed_by(&lat_b));
        assert!(lat_a.is_subsumed_by(&meet));
    }

    #[test]
    fn test_sandhi_graph_transitive_and_satisfies_bounds() {
        let mut registry = TraitRegistry::new();

        let eq = SymbolId(10);
        let ord = SymbolId(11);
        let sortable = SymbolId(12);

        // Eq has no supertraits
        registry.register_trait(TraitDef {
            symbol: eq,
            name: "Eq".to_string(),
            methods: HashMap::new(),
            super_traits: vec![],
        });

        // Ord requires Eq
        registry.register_trait(TraitDef {
            symbol: ord,
            name: "Ord".to_string(),
            methods: HashMap::new(),
            super_traits: vec![eq],
        });

        // Sortable requires Ord (which implies Eq)
        registry.register_trait(TraitDef {
            symbol: sortable,
            name: "Sortable".to_string(),
            methods: HashMap::new(),
            super_traits: vec![ord],
        });

        let my_int_type = TypeId(100);
        // Implement Sortable for my_int_type
        registry.register_impl(ImplEntry {
            target_type: my_int_type,
            trait_id: Some(sortable),
            methods: HashMap::new(),
            span: Span::dummy(),
        });

        let mut graph = SandhiGraph::new();
        graph.build_from_registry(&registry);

        // Sortable's transitive supertraits should include both Ord and Eq
        let sortable_node = graph.nodes.get(&sortable).unwrap();
        assert!(sortable_node.transitive_supertraits.contains(&ord));
        assert!(sortable_node.transitive_supertraits.contains(&eq));

        // my_int_type implements Sortable, so it automatically satisfies Eq, Ord, and Sortable
        assert!(graph.satisfies_bounds(my_int_type, &[eq]));
        assert!(graph.satisfies_bounds(my_int_type, &[ord]));
        assert!(graph.satisfies_bounds(my_int_type, &[sortable]));
        assert!(graph.satisfies_bounds(my_int_type, &[eq, ord, sortable]));

        let other_type = TypeId(200);
        assert!(!graph.satisfies_bounds(other_type, &[eq]));
    }

    #[test]
    fn test_sandhi_graph_cycle_detection() {
        let mut registry = TraitRegistry::new();
        let t1 = SymbolId(1);
        let t2 = SymbolId(2);

        // T1 -> T2 -> T1 (Cycle)
        registry.register_trait(TraitDef {
            symbol: t1,
            name: "T1".to_string(),
            methods: HashMap::new(),
            super_traits: vec![t2],
        });
        registry.register_trait(TraitDef {
            symbol: t2,
            name: "T2".to_string(),
            methods: HashMap::new(),
            super_traits: vec![t1],
        });

        let mut graph = SandhiGraph::new();
        graph.build_from_registry(&registry);

        let cycles = graph.detect_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_sandhi_smt_coherence_verification() {
        let mut registry = TraitRegistry::new();
        let t1 = SymbolId(1);
        let t2 = SymbolId(2);

        registry.register_trait(TraitDef {
            symbol: t1,
            name: "Child".to_string(),
            methods: HashMap::new(),
            super_traits: vec![t2],
        });
        registry.register_trait(TraitDef {
            symbol: t2,
            name: "Parent".to_string(),
            methods: HashMap::new(),
            super_traits: vec![],
        });

        let mut graph = SandhiGraph::new();
        graph.build_from_registry(&registry);

        assert!(SandhiSmtVerifier::verify_coherence(&graph));
    }

    #[test]
    fn test_monomorphization_graph_instantiation_and_ordering() {
        let mut mono = MonomorphizationGraph::new();
        let sym_main = SymbolId(1);
        let sym_helper = SymbolId(2);
        let sym_leaf = SymbolId(3);

        let ty_i32 = TypeId(10);
        let ty_f64 = TypeId(11);

        let root = mono.add_root(sym_main, vec![]).expect("add root");
        let helper = mono
            .instantiate(Some(&root), sym_helper, vec![ty_i32])
            .expect("helper i32");
        let leaf = mono
            .instantiate(Some(&helper), sym_leaf, vec![ty_i32, ty_f64])
            .expect("leaf");

        assert_eq!(mono.instances.len(), 3);
        assert!(mono.instances[&helper].callers.contains(&root));
        assert!(mono.instances[&leaf].callers.contains(&helper));

        let order = mono.compute_compilation_order().expect("compilation order");
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], root);
    }

    #[test]
    fn test_monomorphization_graph_recursion_limit_protection() {
        let mut mono = MonomorphizationGraph::new();
        let sym_fn = SymbolId(1);

        let mut prev = mono
            .add_root(sym_fn, vec![TypeId(0)])
            .expect("initial root");
        let mut hit_limit = false;

        for i in 1..=MAX_MONOMORPHIZATION_DEPTH + 10 {
            match mono.instantiate(Some(&prev), sym_fn, vec![TypeId(i as u32)]) {
                Ok(next) => prev = next,
                Err(MonomorphizationError::RecursionLimitExceeded { depth, .. }) => {
                    assert!(depth > MAX_MONOMORPHIZATION_DEPTH);
                    hit_limit = true;
                    break;
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }

        assert!(
            hit_limit,
            "Recursion limit should protect against infinite monomorphization depth"
        );
    }
}
