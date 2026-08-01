# Agam Design Principles — Indic Grammatical Foundations

> *"The first generative grammar in the modern sense was Pāṇini's grammar."*
> — Noam Chomsky

> *"Pāṇini's notation is equivalent in its power to that of Backus."*
> — P.Z. Ingerman, Communications of the ACM, 1967

---

## Overview

Agam's design draws from the world's two oldest and most rigorous formal grammar systems: **Pāṇini's Aṣṭādhyāyī** (Sanskrit, ~4th century BCE) and the **Tolkāppiyam** (Tamil, ~3rd century BCE). These are not aesthetic influences — they are the **direct ancestors of modern formal language theory**.

The name **Agam** itself carries this heritage:
- **Sanskrit** *Āgama* (आगम) — "that which has come down": received wisdom, proven tradition
- **Tamil** *Agam* (அகம்) — "inner self, interior": the essential core

These seven principles shape every design decision in Agam, from type composition to API naming to error messages.

---

## The Eternal Manifesto: Timeless Foundations

Agam is built to transcend temporary industry paradigms and syntax trends. Most programming languages reflect the technology trends of their decade; Agam is built on mathematical invariants that have endured for over 2,500 years.

### The 5 Invariant Axioms of Agam

1. **Grammatical Invariance:** Standard library APIs derive deterministically from root action verbs (*Dhātu*). Syntactic trends fade, but structural root derivation is immutable.
2. **Pure Term Rewriting:** The compiler IR models computation as E-Graph equality saturation and formal term reductions (*Sandhi & Samāsa*). Computation is invariant across hardware shifts (CPU, GPU, Neuromorphic, Quantum).
3. **Absolute Mathematical Safety:** Zero undefined behavior. Every type junction is governed by deterministic reduction rules—the foundation is mathematically uncrackable.
4. **First-Class Primitive Integration:** Compute capabilities (tensors, autodiff, concurrency) are language-native primitives, not fragile C-binding wrappers.
5. **Role-First Transparency:** Execution intent and argument roles (*Vibhakti*) are explicit at every call site. Context inherits cleanly (*Anuvṛtti*), eliminating hidden magic.

---

## Core Philosophy Maxims

| Core Pillar | **The Agam Philosophy Maxim** | Principle / Source |
|---|---|---|
| **Root Derivation** | *One verb per action; every API derives from a canonical root.* | **Dhātu** (Pāṇini) |
| **Role Marking** | *Role over order; arguments state their purpose, not just their position.* | **Vibhakti** (Tolkāppiyam) |
| **Type Junctions** | *Type junctions are absolute laws, never guesses.* | **Sandhi** (Pāṇini) |
| **Compound Structure** | *Every abstraction fits one of four canonical composition patterns.* | **Samāsa** (Pāṇini) |
| **Contextual Flow** | *Context flows forward; express what changes, inherit what stays.* | **Anuvṛtti** (Pāṇini) |
| **Categorical Bounds** | *Group traits into named constraints; never duplicate bound lists.* | **Pratyāhāra** (Pāṇini) |
| **Agglutinative Chains** | *Chain operations like suffixes; container identity is sacred.* | **Oṭṭu** (Tolkāppiyam) |
| **Strategic Lowering** | *Lower progressively through Sāma, Dāna, Bheda, and Daṇḍa.* | **Upāya** (Chāṇakya Nīti) |
| **Diagnostic Proofs** | *Errors are formal proofs (Fact, Reason, Fix, Law), never cryptics.* | **Nyāya** Epistemology |
| **Developer Experience** | *Zero warning bloat; deterministic builds and guided feedback.* | **Tirukkuraḷ** Ethics |
| **State & Effect** | *Separate pure data (Puruṣa) from active energy (Prakṛti).* | **Sāṃkhya** Dualism |

---

## Strategic & Operational Foundations: Chāṇakya Nīti

While Pāṇini and the Tolkāppiyam govern Agam's **Grammatical & Structural Rigor** (language mechanics), **Chāṇakya Nīti** (and Kautilya's *Arthaśāstra*, ~4th c. BCE) governs Agam's **Strategic & Operational Pragmatism** (compiler pipeline, runtime governance, resource management, and execution sandboxing).

### The 4 Strategic Pillars of Operational Execution

#### 1. *Upāya-Chatuṣṭaya* — 4-Tier Progressive Lowering & Optimization
Chāṇakya's 4 strategies for handling any challenge (*Sāma, Dāna, Bheda, Daṇḍa*) dictate Agam's compilation pipeline:
- **Sāma (Conciliation):** *Tier 1 — Fast Inlining & Constant Folding.* Zero-cost AST pass for rapid developer feedback.
- **Dāna (Concession):** *Tier 2 — Profile-Guided Specialization (PGO).* Allocate extra hardware and cache budgets to hot execution paths.
- **Bheda (Decomposition):** *Tier 3 — Auto-Vectorization & GPU Warp Splitting.* Decompose iteration loops across SIMD registers and GPU hardware warps.
- **Daṇḍa (Force):** *Tier 4 — E-Graph Superoptimization & Hard Enforcement.* Heavy formal lowering pass to guarantee optimal assembly and enforce hard OS bounds.

#### 2. *Kosha-Mūlo Daṇḍaḥ* — Strict Resource Treasury Sovereignty
> *"All governance and power rest upon the treasury (Kosha)."*

Memory, cache lines, and thread limits are primary constraints. Agam treats memory as a strictly managed treasury: zero hidden heap allocations, cache-aware data layouts by default, and OS-level job resource limits (`JobObject` / `cgroups`).

#### 3. *Anāgata-Vidhātā* — Proactive Defenses & Failure Prevention
> *"He who foresees trouble and acts before it arrives is never defeated."*

Runtime failures are pre-empted statically at compile time through native effect tracking, static range checks, and borrow invariants—preventing production failures before code ever executes.

#### 4. *Durdharṣa & Gupta-Vidyā* — Unbreachable Execution Sandboxing
> *"Guard secrets and vulnerabilities behind multi-layered fortifications."*

Un-sandboxed execution is prohibited by default. Headless code execution, package builds, and external modules operate inside isolated OS sandboxes with zero ambient privilege.

---

## Epistemology & DX Foundations: Nyāya, Tirukkuraḷ & Sāṃkhya

### Nyāya Epistemology — The Diagnostic Engine (`agam_errors`)
The **Nyāya** school of formal logic (~2nd c. BCE) dictates that every diagnostic emitted by `agamc` must constitute a formal four-part proof (*Pramāṇa*):
1. **Fact (*Pratyakṣa*):** Exact source span and token location.
2. **Reason (*Anumāna*):** Deductive explanation of the violated constraint.
3. **Fix (*Upamāna*):** Analogy-driven contextual suggested replacement.
4. **Law (*Śabda*):** Authoritative link to the Agam formal specification.

### Tirukkuraḷ Principles — Ergonomics & Build Determinism
Grounded in Thiruvalluvar's *Tirukkuraḷ* (~1st c. BCE), Agam enforces developer experience axioms:
- **Zero Warning Bloat:** Compilers must not pollute developer stdout with ignored warnings.
- **Bit-for-Bit Determinism:** Compiling identical source files produces byte-for-byte identical binary targets.
- **Compiler as Mentor:** Diagnostics serve as constructive guidance rather than punitive rejection.

### Sāṃkhya Dualism — Separation of State & Effect
Derived from Sāṃkhya philosophy, Agam strictly decouples pure data structures (*Puruṣa*) from active state mutations and side effects (*Prakṛti*). Side effects (I/O, GPU dispatches, memory mutation) are managed through first-class `perform`/`handle` effect handlers, keeping function bodies side-effect transparent and deterministically testable.

---

## Principle 1: Dhātu (धातु) — Systematic Root Derivation

### Origin
In Pāṇini's system, every Sanskrit word derives from one of ~2,000 verbal roots (*dhātu*) through systematic application of affixes. Given a root and a set of affixes, the derivation is **completely deterministic** — there is exactly one correct form.

### Application to Agam
Agam's standard library uses a **canonical root verb table**. Every API method name is derived from a small set of (~30) root action verbs through systematic suffixing. This eliminates the naming chaos that plagues every other language's stdlib.

**The problem in other languages:**
```
Python:  list.append(), dict.update(), set.add(), str.join()
Rust:    vec.push(), map.insert(), set.insert(), str.push_str()
```
Four different verbs for the same semantic action (mutation/addition). A developer must memorize each one individually.

**Agam's dhātu-based approach:**
The root verb for "add element to collection" is **`add`**. Always. Everywhere.

```agam
list.add(item)         # List<T> — derived from root 'add'
map.add(key, value)    # Map<K,V> — same root
set.add(item)          # Set<T> — same root
string.add(char)       # String — same root

list.add_all(items)    # Systematic suffix: _all = batch operation
map.add_all(pairs)     # Predictable derivation
```

**Full root verb table:** See [naming-conventions.md](naming-conventions.md)

### Design Rule
> Every method name in the standard library must be derivable from the root verb table. If a new action category is needed, a new root verb is added to the table — not an ad-hoc name.

---

## Principle 2: Vibhakti (விபக்தி / विभक्ति) — Semantic Role Marking

### Origin
Both Tamil and Sanskrit mark every noun with its **grammatical role** through case suffixes (*vibhakti*). Tamil has 8 cases; Sanskrit has 8 (7 + vocative). The case suffix tells you the noun's role regardless of word order:

| Case | Tamil Suffix | Sanskrit | Role |
|------|-------------|----------|------|
| Nominative | — | -ḥ | Agent (who does it) |
| Accusative | -ai (ஐ) | -am | Object (what is acted on) |
| Instrumental | -āl (ஆல்) | -ena | Instrument (with what) |
| Dative | -kku (க்கு) | -āya | Recipient (to whom) |
| Ablative | -il irundu (இலிருந்து) | -āt | Source (from where) |
| Locative | -il (இல்) | -e | Location (where) |
| Genitive | -in (இன்) | -asya | Possession (of whom) |

### Application to Agam
Function parameters carry **semantic role labels** at call sites. This is not just "named arguments" — it is a principled system where each label communicates the argument's role in the operation.

**Canonical role labels for common operations:**

| Operation Pattern | Role Labels | Vibhakti Equivalent |
|---|---|---|
| Transfer/Move | `from:`, `to:` | Ablative → Dative |
| Transformation | `source:`, `into:` | Nominative → Accusative |
| Creation | `using:`, `with:` | Instrumental |
| Search | `in:`, `at:` | Locative |
| Association | `of:`, `for:` | Genitive, Dative |

**Example — function signatures with role clarity:**

```agam
@lang.base
fn transfer(from source: Account, to target: Account, amount value: Currency):
    # 'from' and 'to' are external labels (caller sees them)
    # 'source' and 'target' are internal names (function body uses them)
    source.debit(value)
    target.credit(value)

# Call site reads like natural language with explicit roles:
transfer(from: checking, to: savings, amount: Currency(500))
```

```agam
@lang.advance
fn copy<T: Clone>(from source: &[T], to dest: &mut [T], count n: usize) -> usize {
    // Role labels make the direction of data flow unambiguous
    let copied = min(n, source.len(), dest.len());
    dest[..copied].clone_from_slice(&source[..copied]);
    copied
}

copy(from: &input_buffer, to: &mut output_buffer, count: 1024);
```

### Design Rule
> When a function has 2+ parameters of the same type, or when parameter order is ambiguous, role labels are **strongly recommended**. The standard library uses role labels consistently for all non-trivial APIs.

---

## Principle 3: Sandhi (சந்தி / सन्धि) — Type Junction Rules

### Origin
Sanskrit *sandhi* defines exact, deterministic rules for how sounds transform when two morphemes meet at a junction. There are three categories:
- **Svara sandhi** (vowel junction): vowel + vowel → specific result
- **Vyañjana sandhi** (consonant junction): consonant + vowel → specific result
- **Visarga sandhi** (breath junction): visarga + consonant → specific result

The key insight: **junction behavior is never ambiguous**. Given any two elements meeting, the sandhi table tells you exactly what happens.

### Application to Agam
Agam defines a **type sandhi table** — formal, documented rules for what happens when types compose. No ad-hoc coercions. No "it depends." Every composition has one predictable result.

**Full sandhi table:** See [type-sandhi.md](type-sandhi.md)

**Core rules preview:**

```agam
# Sandhi Rule S1: Optional flattening
# Optional<Optional<T>> → Optional<T>  (like vowel contraction)
let a: i32? = get_value()
let b: i32? = a?.transform()    # NOT i32?? — sandhi flattens

# Sandhi Rule S2: Result propagation
# Result<Result<T, E>, E> → Result<T, E>  (via ? operator)
let c: Result<i32, Error> = parse()?.compute()?  # Flattened

# Sandhi Rule S3: Reference contraction
# &&T → &T  (auto-deref sandhi)
# &mut &T → &T  (mutability absorbed)

# Sandhi Rule S4: Trait junction
# (A: Trait1) + (A: Trait2) → A: Trait1 + Trait2  (union)
# Conflicting methods → compile error (explicit resolution required)
```

### Design Rule
> Every type composition in Agam is covered by the sandhi table. If a composition doesn't appear in the table, it is a compile error — never an implicit conversion.

---

## Principle 4: Samāsa (சமாசம் / समास) — Compound Type Formation

### Origin
Sanskrit forms compound words through exactly **four patterns** (samāsa). These four cover all possible semantic relationships between components:

| Samāsa | Meaning | Example |
|--------|---------|---------|
| **Tatpuruṣa** | Determinative — first qualifies second | *rāja-putra* (king-son = prince) |
| **Dvandva** | Coordinative — both parts equally present | *rāma-kṛṣṇau* (Rama-and-Krishna) |
| **Bahuvrīhi** | Possessive — describes what something *has* | *nīla-kaṇṭha* (blue-throat = Shiva) |
| **Avyayībhāva** | Adverbial — first part modifies as a whole | *yathā-śakti* (according-to-ability) |

### Application to Agam
Agam's type system recognizes **four canonical patterns** of type composition. Every compound type in the language maps to one of these patterns:

```agam
# 1. TATPURUṢA — Generic specialization (first qualifies second)
type JsonParser = Parser<Json>       # "a Parser, specialized for Json"
type IntList = List<i32>             # "a List, specialized for i32"
type UserMap = Map<String, User>     # "a Map, specialized for String→User"

# 2. DVANDVA — Trait conjunction (both equally present)
type ReadWrite = Read + Write        # "both Read AND Write"
type SendSync = Send + Sync          # "both Send AND Sync"
constraint Numeric = Add + Sub + Mul + Div  # "all four traits"

# 3. BAHUVRĪHI — Capability description (describes what it HAS)
trait HasLength { fn len(self) -> usize }   # "that which has length"
trait HasIterator { type Item; fn iter(self) -> Iterator<Self::Item> }
trait Hashable { fn hash(self) -> u64 }     # "that which is hashable"

# 4. AVYAYĪBHĀVA — Annotation modifiers (modifies entire declaration)
@gpu fn kernel(data: Buffer<f32>)     # "as GPU" — modifies the whole function
@inline fn fast_add(a: i32, b: i32)   # "as inline" — adverbial modifier
@test fn verify_sandhi()               # "as test" — purpose modifier
```

### Design Rule
> When designing a new type abstraction, identify which samāsa pattern it follows. If it doesn't fit any of the four, the abstraction may be too complex and should be decomposed.

---

## Principle 5: Anuvṛtti (அனுவிருத்தி / अनुवृत्ति) — Contextual Inheritance

### Origin
Panini's most elegant optimization: *anuvṛtti* ("carrying forward"). When a rule is stated, its conditions carry forward to subsequent rules without being restated. This allows Panini to express 4,000 rules with extreme brevity — each sūtra relies on context from its predecessors.

### Application to Agam
Declaration blocks carry context forward, reducing repetitive annotations. The carry-forward rules are **explicit and specified**, not implicit magic.

```agam
@lang.base

# Anuvṛtti in impl blocks: &self carries forward
impl Stack<T>:
    fn push(item: T):         # &mut self carried from impl context
        self.data.add(item)
    
    fn pop() -> T?:            # &mut self carried forward
        self.data.remove_last()
    
    fn peek() -> T?:           # context switches to &self (immutable)
        self.data.last()
    
    fn len() -> usize:         # &self carried forward
        self.data.len()

# Anuvṛtti in pub modules: visibility carries forward
pub mod api:
    fn list_users() -> List<User>:     # inherits pub
        ...
    fn get_user(id: UserId) -> User?:  # inherits pub
        ...
    
    private:                            # context override
        fn validate_token(t: Token) -> bool:  # private
            ...
```

```agam
@lang.advance

// Anuvṛtti in trait implementations
impl Display for Matrix<T: Display> {
    // All methods in this block inherit the trait context
    fn fmt(&self, f: &mut Formatter) -> Result {
        for row in &self.rows {
            for col in row {
                write!(f, "{:8.2}", col)?;   // Display context from trait
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
```

### Carry-Forward Rules (specified)

| Context Block | What Carries Forward | Override Mechanism |
|---|---|---|
| `impl Type` | `&self` receiver, type parameters | `mut self`, `self`, explicit params |
| `pub mod` | `pub` visibility | `private:` section |
| `unsafe` block | unsafe context | — (always explicit) |
| `@gpu` block | GPU target context | `@cpu` escape |
| `trait` | required method signatures | default implementation bodies |

### Design Rule
> Anuvṛtti is syntactic sugar, not semantic change. The expanded form must always be expressible explicitly. The carry-forward rules are documented in this table — no hidden context.

---

## Principle 6: Pratyāhāra (பிரத்தியாகாரம் / प्रत्याहार) — Categorical Shorthands

### Origin
Panini's most famous invention: the *pratyāhāra*. The Māheśvara Sūtras list all Sanskrit sounds in a specific order with marker letters (*anubandha*). By citing the first sound and a marker, you reference the entire class between them. For example, *aC* = all vowels. One code replaces an enumeration that could be dozens of items.

### Application to Agam
The `constraint` keyword creates categorical shorthands for trait bound groups — the pratyāhāra of Agam's type system:

```agam
# Define pratyāhāra (categorical shorthands):
constraint Sortable = Ord + Eq + Clone
constraint Serializable = Encode + Decode + Schema + Debug
constraint Numeric = Add + Sub + Mul + Div + Rem + Neg + Ord + Eq + Copy
constraint Printable = Display + Debug
constraint ThreadSafe = Send + Sync + 'static

# Use them — one code replaces many bounds:
fn sort<T: Sortable>(list: List<T>) -> List<T>:
    ...

fn serialize<T: Serializable>(value: T) -> Bytes:
    ...

fn parallel_sum<T: Numeric + ThreadSafe>(data: List<T>) -> T:
    ...

# Constraints compose (pratyāhāra of pratyāhāras):
constraint SortableAndPrintable = Sortable + Printable

# Constraints can have associated requirements:
constraint Collection:
    type Item
    requires Iterable<Self::Item>
    requires HasLength
    requires Display
```

### Comparison to Competitors

| Language | Feature | Status |
|---|---|---|
| **Rust** | Trait aliases | Unstable, `#![feature(trait_alias)]` |
| **Swift** | Protocol compositions | `typealias X = P1 & P2` (limited) |
| **Haskell** | Constraint synonyms | `type C a = (Eq a, Show a)` |
| **Agam** | `constraint` keyword | First-class, stable, composable |

### Design Rule
> When a trait bound combination appears in 3+ function signatures, extract it into a named `constraint`. The stdlib ships with standard constraints (`Sortable`, `Numeric`, `Serializable`, `ThreadSafe`) as first-class vocabulary.

---

## Principle 7: Oṭṭu (ஒட்டு) — Agglutinative Composition

### Origin
Tamil is an **agglutinative language**: complex meanings are built by attaching suffixes (*oṭṭu*) to roots without altering the root form. The key property: **each suffix is independent and composable** — adding one suffix never changes the meaning of another.

For example, Tamil builds verb forms by stacking:
*paditt-u-k-koṇḍ-irunt-ēn* (I had been reading):
- *paditt* (read — root unchanged)
- *-u-k-koṇḍ* (continuous aspect — suffix)
- *-irunt* (past tense — suffix)
- *-ēn* (first person — suffix)

### Application to Agam
Fluent method chains in Agam follow **agglutinative rules**: each method (suffix) is independent, composable, and the root container type's identity is preserved through intermediate operations.

```agam
# Agglutinative chain — each "suffix" is independent and composable:
let result = users                    # Root: List<User>
    .filter(|u| u.active)            # Suffix 1: preserves List<User>
    .map(|u| u.score)                # Suffix 2: transforms to List<i32>
    .filter(|s| s > threshold)       # Suffix 3: preserves List<i32>
    .sort()                          # Suffix 4: preserves List<i32>
    .take(top_n)                     # Suffix 5: preserves List<i32>
    .collect()                       # Terminal suffix: produces Vec<i32>

# The agglutinative guarantee:
# 1. Intermediate suffixes (filter, sort, take) NEVER change container kind
# 2. Transform suffixes (map) change element type but preserve container kind
# 3. Only TERMINAL suffixes (collect, build, to_string, sum) produce new types
# 4. Suffixes can be reordered freely when semantically valid
```

**Formal suffix classification:**

| Suffix Type | Behavior | Examples |
|---|---|---|
| **Preserving** | Container type unchanged | `.filter()`, `.sort()`, `.reverse()`, `.take()` |
| **Transforming** | Element type changes, container preserved | `.map()`, `.flat_map()`, `.enumerate()` |
| **Terminal** | Produces a new type entirely | `.collect()`, `.sum()`, `.count()`, `.build()` |
| **Side-effect** | Returns same type, performs action | `.inspect()`, `.log()`, `.tap()` |

### Design Rule
> Standard library collection APIs must classify every method as preserving, transforming, terminal, or side-effect. The type system enforces that preserving methods return the same container type. This is documented in the API reference alongside each method.

---

## Summary: The Seven Pillars

| # | Principle | Sanskrit/Tamil | Agam Feature |
|---|-----------|---------------|-------------|
| 1 | **Dhātu** | Root derivation | Systematic stdlib naming (~30 root verbs) |
| 2 | **Vibhakti** | Case roles | Named args with semantic role labels |
| 3 | **Sandhi** | Junction rules | Formal type composition table |
| 4 | **Samāsa** | Compound formation | 4 canonical type composition patterns |
| 5 | **Anuvṛtti** | Rule inheritance | Contextual defaults in declaration blocks |
| 6 | **Pratyāhāra** | Categorical shorthand | `constraint` keyword for bound groups |
| 7 | **Oṭṭu** | Agglutination | Classified fluent method chains |

---

## References

- Pāṇini, *Aṣṭādhyāyī* (~4th century BCE) — 3,959 sūtras defining the complete Sanskrit grammar
- Tolkāppiyar, *Tolkāppiyam* (~3rd century BCE) — the oldest extant Tamil grammar
- Chāṇakya (Kautilya), *Arthaśāstra* & *Chāṇakya Nīti* (~4th century BCE) — foundational treatises on strategy, governance, and resource management
- P.Z. Ingerman, "Pāṇini-Backus Form Suggested," *Communications of the ACM* 10.3 (1967): 137
- Noam Chomsky, acknowledgment of Pāṇini as first generative grammarian in *Current Issues in Linguistic Theory* (1964)
- Amba Kulkarni, "Computational Linguistics and Sanskrit" — modern formalization of Paninian grammar
- T. Lehmann, "A Grammar of Old Tamil for Students" — computational analysis of Tolkāppiyam morphology

---

## Architectural Cross-Reference (2026)

The Indic design and strategic principles are not merely frontend syntactic sugar; they directly inform Agam's world-class 2026 compiler architecture:

- **E-Graph Superoptimization (Middle-end)**: The `agam_mir` E-Graph native representation uses **Sandhi** (junction rules) and **Samāsa** (compound formation) as its foundational, deterministic rewrite rules. Equality saturation relies on Sandhi to guarantee that type combinations reduce to a canonical, optimized form.
- **Progressive Lowering & Sandboxing (Compiler & Runtime)**: The 4-tier lowering strategy (*Upāya-Chatuṣṭaya*) and runtime isolation (*Durdharṣa*) derive directly from **Chāṇakya Nīti**, governing OS-level JobObject limits, capability security, and profile-guided tier escalations.
- **LLVM 22.1 & GPU AutoWS (Backend)**: Advanced targets leverage **Avyayībhāva** (adverbial modifiers). The `@gpu(warp_specialize)` and `@llvm(denormal_fpenv)` annotations follow strict adverbial composition rules, guaranteeing that progressive lowering through MLIR dialects preserves the original hardware-introspection intent.
- **Salsa Query Engine (Frontend)**: The demand-driven frontend relies on **Anuvṛtti** (contextual inheritance) to invalidate caches efficiently. When a `pub mod` or `impl` block changes, the inherited context strictly bounds the required re-evaluation graph.
