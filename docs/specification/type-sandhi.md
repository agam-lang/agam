# Agam Type Sandhi Table

> Based on **Principle 3: Sandhi** — Type Junction Rules
>
> *Sanskrit sandhi defines deterministic rules for sound changes at morpheme
> junctions. Agam applies this to type composition: every type combination
> has exactly one predictable result, documented in this table.*

---

## What Is Type Sandhi?

When two types meet at a composition boundary — via operators, method chains, 
generic instantiation, or implicit conversions — the result must be **deterministic 
and documented**. This table is the canonical reference.

There are no implicit coercions in Agam outside this table. If a type combination 
doesn't appear here, it is a compile error.

---

## S1: Optional Sandhi (Svara Junction)

Like Sanskrit vowel sandhi, where two vowels at a junction contract into one, 
nested `Optional` types flatten.

| Junction | Result | Rule |
|----------|--------|------|
| `T?` (= `Optional<T>`) | `Optional<T>` | Identity |
| `T??` (= `Optional<Optional<T>>`) | `Optional<T>` | **Flattening** |
| `T???` | `Optional<T>` | Recursive flattening |
| `T? + .method() -> U?` | `U?` | Chained optional |
| `T? + .method() -> U` | `U?` | Lifting (method might not run) |

```agam
# Practical example:
let user: User? = find_user(id)        # Optional<User>
let email: String? = user?.email        # Optional<String>, NOT Optional<Optional<String>>
let domain: String? = email?.split("@")?.last()  # Still Optional<String>
```

**Mechanism:** The `?.` operator performs optional chaining with automatic flattening.

---

## S2: Result Sandhi (Vyañjana Junction)

Like Sanskrit consonant sandhi, where consonants at junctions transform 
predictably, `Result` types compose via the `?` operator.

| Junction | Result | Rule |
|----------|--------|------|
| `Result<T, E>` | `Result<T, E>` | Identity |
| `Result<Result<T, E>, E>` | `Result<T, E>` | **Flattening via `?`** |
| `Result<T, E1> + Result<U, E2>` | `Result<U, Error>` | **Error union** (if E1, E2: Into<Error>) |
| `Result<T, E> + .map(f) -> Result<U, E>` | `Result<U, E>` | Transform ok path |
| `Result<T, E> + .map_err(f) -> Result<T, F>` | `Result<T, F>` | Transform err path |

```agam
# Practical example — Result sandhi via ? operator:
fn process(path: String) -> Result<Data, Error>:
    let file = File.open(path)?            # Result<File, IoError> → File (or early return)
    let content = file.read_all()?         # Result<String, IoError> → String
    let data = parse(content)?             # Result<Data, ParseError> → Data
    return Ok(data)

# Each ? performs Result sandhi: unwraps Ok or propagates Err.
# Error types auto-convert if they implement Into<Error>.
```

---

## S3: Reference Sandhi (Visarga Junction)

Like Sanskrit visarga sandhi, where breath marks transform based on what follows,
reference types have deterministic composition rules.

| Junction | Result | Rule |
|----------|--------|------|
| `&&T` | `&T` | **Auto-deref** (double ref contracts) |
| `&&&T` | `&T` | Recursive deref |
| `&mut &T` | `&T` | Mutability dropped (inner is immutable) |
| `&&mut T` | `&mut T` | Outer ref derefed, inner preserved |
| `&mut &mut T` | `&mut T` | Double mut contracts |
| `*&T` | `T` | Deref of ref = value (if Copy) |
| `&*T` | Error | Cannot ref a deref without source |

```agam
# Practical example:
let x: i32 = 42
let r1: &i32 = &x
let r2: &&i32 = &r1
let v: i32 = **r2       # Manual deref chain

# With auto-deref sandhi:
fn print_val(v: &i32): print(v)
print_val(r2)           # &&i32 auto-derefs to &i32 — sandhi rule
```

### Auto-Deref Precedence
When a method is called on a reference type, Agam tries these in order:
1. Method on `&T` directly
2. Method on `T` (auto-deref once)
3. Method on `&T` via trait (auto-ref)

This is the same precedence Rust uses, but Agam documents it as a sandhi rule.

---

## S4: Trait Sandhi (Junction of Capabilities)

When traits combine (via `+` or `constraint`), their junction follows these rules:

| Junction | Result | Rule |
|----------|--------|------|
| `A + B` (no conflict) | `A + B` | **Union** — all methods available |
| `A + A` | `A` | **Idempotence** — duplicate absorbed |
| `A + B` (method conflict) | **Compile error** | Must disambiguate explicitly |
| `A: Super` + `B: Super` | `A + B` (Super counted once) | **Diamond resolution** |

```agam
# Trait sandhi examples:
constraint ReadWrite = Read + Write     # Union — no conflicts
constraint Copyable = Clone + Copy      # Copy implies Clone — absorbed

# Conflict requires disambiguation:
trait Render:
    fn draw(self)
trait Canvas:
    fn draw(self)

# impl Render + Canvas for Widget:
#     fn draw(self)  → ERROR: ambiguous, which draw?
#     fn Render::draw(self)  → OK: explicit disambiguation
```

---

## S5: Numeric Sandhi (Arithmetic Junction)

When numeric types meet in arithmetic expressions, promotion follows strict rules:

| Junction | Result | Rule |
|----------|--------|------|
| `i32 + i32` | `i32` | Same type — no change |
| `i32 + i64` | **Compile error** | No implicit widening |
| `i32 as i64 + i64` | `i64` | Explicit cast required |
| `i32 + f64` | **Compile error** | No implicit int→float |
| `f32 + f64` | **Compile error** | No implicit float widening |

```agam
# Agam does NOT do implicit numeric promotion.
# This is deliberate — it prevents a class of subtle bugs.

let a: i32 = 42
let b: i64 = 100

# let c = a + b    # ERROR: type mismatch i32 + i64
let c = (a as i64) + b  # OK: explicit promotion

# For ergonomic literals, the compiler infers the type:
let d = 42 + 100        # OK: both are untyped int literals → i32
let e: i64 = 42 + 100   # OK: context infers both as i64
```

### Design Rationale
> Sanskrit sandhi is **deterministic** — there are no "usually" or "it depends" rules.
> Similarly, Agam's numeric junction rules have zero implicit conversions. Every type
> change is explicit and visible in the source code.

---

## S6: Generic Sandhi (Parameterized Junction)

When generic types compose, type parameters follow these rules:

| Junction | Result | Rule |
|----------|--------|------|
| `Vec<T> + T` (via `.add()`) | `Vec<T>` | Element absorbed into container |
| `Vec<T> + Vec<T>` (via `.add_all()`) | `Vec<T>` | Container union |
| `Vec<T> + Vec<U>` | **Compile error** | Type parameter mismatch |
| `Map<K, V>.get(K)` | `V?` | Key→Value extraction + Optional lift |
| `Result<Vec<T>, E>.map(f)` | `Result<Vec<U>, E>` | Inner transformation |

```agam
# Generic sandhi — predictable type flow:
let scores: Vec<i32> = [90, 85, 72]
scores.add(95)                    # Vec<i32> + i32 → Vec<i32>
scores.add_all([88, 76])         # Vec<i32> + Vec<i32> → Vec<i32>
# scores.add("hello")            # ERROR: Vec<i32> + String — sandhi violation

let lookup: Map<String, i32> = Map.new()
let val: i32? = lookup.get("key")  # Map<String, i32>.get(String) → i32?
```

---

## S7: Async Sandhi (Temporal Junction)

When async types compose, the junction rules are:

| Junction | Result | Rule |
|----------|--------|------|
| `Future<T>` + `await` | `T` | **Resolution** — future unwrapped |
| `Future<Future<T>>` + `await` | `Future<T>` | Single-level unwrap |
| `Future<Result<T, E>>` + `await` | `Result<T, E>` | Await, then handle result |
| `Future<Result<T, E>>` + `.await?` | `T` or early return | Combined async + error sandhi |

```agam
# Async sandhi — combining temporal and error junctions:
async fn fetch_data(url: String) -> Result<Data, Error>:
    let response = http.get(url).await?     # Future<Result> → Result → T
    let body = response.read_all().await?   # Same sandhi chain
    let data = parse(body)?                 # Pure Result sandhi
    return Ok(data)
```

---

## Sandhi Violation Errors

When a type junction violates the sandhi table, the compiler produces a diagnostic 
that names the violated rule:

```
error[E0301]: type sandhi violation (S5: numeric junction)
  ┌─ src/main.agam:12:15
  │
12│     let c = a + b
  │             ^^^^^ cannot add `i32` and `i64` without explicit conversion
  │
  = note: Agam requires explicit numeric casts (Sandhi Rule S5)
  = help: try `(a as i64) + b` or `a + (b as i32)`
```

```
error[E0302]: type sandhi violation (S1: optional flattening)
  ┌─ src/main.agam:8:20
  │
 8│     let x: i32?? = some_fn()
  │            ^^^^^ nested Optional types are not allowed (Sandhi Rule S1)
  │
  = note: Optional<Optional<T>> flattens to Optional<T> automatically
  = help: use `i32?` instead
```

---

## Summary

| Rule | Name | Principle |
|------|------|-----------|
| **S1** | Optional Sandhi | Nested optionals flatten |
| **S2** | Result Sandhi | Results compose via `?` with error conversion |
| **S3** | Reference Sandhi | References auto-deref at junctions |
| **S4** | Trait Sandhi | Trait bounds union; conflicts require disambiguation |
| **S5** | Numeric Sandhi | No implicit numeric promotion — always explicit |
| **S6** | Generic Sandhi | Type parameters must match at composition boundaries |
| **S7** | Async Sandhi | Futures unwrap one level per `await` |

These rules are **exhaustive**. Any type composition not covered here is a compile error.
