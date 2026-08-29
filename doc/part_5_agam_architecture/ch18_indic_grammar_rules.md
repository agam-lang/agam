# Chapter 18: Indic Grammatical Design Principles (Pāṇini & Tolkāppiyam)

> **System Scope**: Theoretical Design Philosophy (`Phase F6`)  
> **Compiler Module Focus**: [`docs/specification/design-principles.md`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/docs/specification/design-principles.md)

---

## 18.1 Grammatical Principles in Programming Language Design

Agam formalizes seven core language design principles derived from **Pāṇini's Aṣṭādhyāyī** (Sanskrit) and the **Tolkāppiyam** (Tamil) — the world's oldest formal grammar systems:

```text
┌─────────────────────────────────────────────────────────────────┐
│               Indic Grammatical Design Principles               │
├─────────────────────────────────────────────────────────────────┤
│ 1. Dhātu Naming (30 Root Verbs for Core Standard Library APIs)  │
│ 2. Vibhakti Roles (Grammatical case roles for type signatures)  │
│ 3. Type Sandhi (7 Rules governing type composition & unions)   │
│ 4. Pratyāhāra Constraints (Concise type range specifications)   │
│ 5. Anuvṛtti Defaults (Contextual inheritance of defaults)       │
└─────────────────────────────────────────────────────────────────┘
```

---

## 18.2 Dhātu Root Verbs & Vibhakti Roles

### 1. Dhātu Naming Conventions
The standard library API surface is systematically derived from 30 canonical root verbs (*Dhātus*), establishing semantic consistency across all modules:

- `kṛ` (Do/Make) $\rightarrow$ Construct, initialize
- `grah` (Take/Receive) $\rightarrow$ Fetch, parse, extract
- `dā` (Give/Emit) $\rightarrow$ Return, yield, emit

### 2. Vibhakti Roles (Grammatical Cases)
Type parameters and function arguments follow grammatical case roles:
- **Agent (Kartṛ)**: Invoking context
- **Patient/Object (Karman)**: Data target operated upon
- **Instrument (Karaṇa)**: Options or configuration parameters

---

## 18.3 Type Sandhi Rules

**Type Sandhi** establishes formal rules for type composition, union merging, and automatic type coercions:

1. **Vowel Sandhi (Homogeneous Join)**: Merging identical primitive types ($T \cup T \implies T$).
2. **Consonant Sandhi (Subtype Coercion)**: Coercing bounded subtypes to common supertypes.
3. **Visarga Sandhi (Option Transformation)**: Merging optional types ($T \cup \text{Nil} \implies \text{Option}[T]$).
