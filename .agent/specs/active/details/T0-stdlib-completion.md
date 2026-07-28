# Phase T0-stdlib-completion � Indic Grammatical Design Principles

**Status:** complete
**Tier:** 0 (Foundation Completion)
**Priority:** Design philosophy that shapes F2–F5 implementation — parallel execution

## Scope

Formalize a design philosophy for Agam drawn from the world's two oldest and most rigorous formal grammar systems: Pāṇini's *Aṣṭādhyāyī* (Sanskrit, ~4th century BCE) and the *Tolkāppiyam* (Tamil, ~3rd century BCE). These are not aesthetic influences — they are the direct ancestors of modern formal language theory (BNF, CFG, FST) and solve composition, regularity, and ambiguity problems that contemporary programming languages still struggle with.

## Why This Is a Pillar

- Pāṇini's grammar is the **first formal generative grammar in history** — Chomsky acknowledged this, Ingerman (1967) proved equivalence to BNF
- Tamil's agglutinative morphology was formalized with FST-equivalent precision millennia before finite state transducers were invented
- The name "Agam" (अगम / அகம்) itself carries deep Sanskrit and Tamil roots
- **No other programming language** has systematically drawn from these traditions — this is a genuine, defensible differentiator
- These principles directly improve Agam's regularity, composability, and learnability

## The Seven Principles

### 1. Dhātu (धातु) — Root Verb System
**Source:** Sanskrit's ~2,000 verbal roots with systematic affixes
**PL Application:** Stdlib naming guide with ~30 canonical action roots. Every API method name is derivable from a root + systematic suffix. Eliminates naming chaos (read/load/get/fetch → one canonical verb per action category).

**Deliverable:** `docs/specification/naming-conventions.md`

### 2. Vibhakti (विभक्ति) — Semantic Role Marking
**Source:** Tamil/Sanskrit case system marks every noun's grammatical role (agent, object, recipient, instrument, location)
**PL Application:** Named arguments with semantic role labels. Parameters carry their role (`from:`, `to:`, `using:`, `at:`), not just position. Function signatures read like natural language with explicit case marking.

**Deliverable:** Named argument role conventions in F5, documented in `docs/specification/design-principles.md`

### 3. Sandhi (सन्धि) — Type Junction Rules
**Source:** Sanskrit sandhi defines exact transformations at element junctions (vowel+vowel, consonant+vowel, etc.)
**PL Application:** Formal "sandhi table" for type composition — predictable, documented rules for what happens when types combine. `Optional<Optional<T>>` flattens to `Optional<T>`. `Result` composes via `?`. Reference chains auto-deref. No ad-hoc coercions — every rule specified.

**Deliverable:** `docs/specification/type-sandhi.md`

### 4. Samāsa (समास) — Compound Type Formation
**Source:** Sanskrit compound words follow exactly 4 patterns (tatpuruṣa, dvandva, bahuvrīhi, avyayībhāva)
**PL Application:** Type composition follows a small set of canonical patterns:
- **Dvandva** (coordinative): `Read + Write` trait combination
- **Tatpuruṣa** (determinative): `Parser<Json>` generic specialization
- **Bahuvrīhi** (possessive): `trait HasLength` capability description
- **Avyayībhāva** (adverbial): `@inline`, `@gpu` annotation modifiers

**Deliverable:** Documented in `docs/specification/design-principles.md`

### 5. Anuvṛtti (अनुवृत्ति) — Contextual Rule Inheritance
**Source:** Panini's anuvṛtti carries rules forward from previous sūtras without restating them
**PL Application:** Contextual defaults in declaration blocks. `impl` blocks carry `&self` forward. `pub mod` blocks inherit visibility. Formally specified carry-forward rules, not magic.

**Deliverable:** Design principle for F3/F4, documented in `docs/specification/design-principles.md`

### 6. Pratyāhāra (प्रत्याहार) — Constraint Shorthands
**Source:** Panini invented pratyāhāra — categorical abbreviations where a single code represents an entire class of sounds
**PL Application:** `constraint` keyword for trait bound groups:
```agam
constraint Sortable = Ord + Eq + Clone + Debug
constraint Numeric = Add + Sub + Mul + Div + Ord
fn sort<T: Sortable>(list: List<T>) -> List<T>
```
More principled than Rust's unstable trait aliases.

**Deliverable:** Syntax addition in F2 generics, documented in `docs/specification/design-principles.md`

### 7. Oṭṭu (ஒட்டு) — Agglutinative Composition (Tamil)
**Source:** Tamil builds complex meanings by attaching suffixes to roots without altering them
**PL Application:** Formally guaranteed fluent method chains. Intermediate "suffixes" (filter, map, sort, take) preserve container type identity. Only terminal suffixes (collect, build, to_string) produce new types. This is specified in the type system, not a convention.

**Deliverable:** Design principle for F2/F3, documented in `docs/specification/design-principles.md`

## What This Is NOT

- ❌ Sanskrit or Tamil **keywords** (Agam stays English-keyword)
- ❌ A fourth syntax mode for Devanagari/Tamil script
- ❌ Superficial cultural branding without technical substance
- ❌ Any breaking changes to existing syntax

## What This IS

- ✅ A formal **design philosophy document** with 7 named principles
- ✅ Concrete **stdlib naming conventions** from root verb analysis
- ✅ **Type composition rules** formalized as a "sandhi table"
- ✅ **Constraint shorthands** inspired by pratyāhāra
- ✅ **Named argument semantics** informed by vibhakti
- ✅ Documentation acknowledging these influences as a differentiator

## Deliverables

- [x] `docs/specification/design-principles.md` — the 7 principles with PL mappings
- [x] `docs/specification/naming-conventions.md` — stdlib root verb table
- [x] `docs/specification/type-sandhi.md` — formal type composition rules
- [x] README/architecture doc references

## Responsible Areas

- `docs/specification/` — primary output (new directory)
- Informs: `agam_sema` (type sandhi), `agam_ast` (constraint syntax), `agam_std` (naming)
- Informs: F2 (pratyāhāra constraints, sandhi rules), F3 (anuvṛtti, samāsa), F5 (vibhakti, dhātu)

## Dependencies

- **None** — this is a design specification, can execute immediately
- **Influences** F2, F3, F4, F5 — should be completed before/during those phases

## Test Strategy

- Specification review: every principle maps to at least one concrete syntax/API decision
- Cross-reference: verify existing examples comply with the naming guide
- Future: compiler diagnostics can reference principle names in suggestions
