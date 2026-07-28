# Agam Standard Library Naming Conventions

> Based on **Principle 1: Dhātu** — Systematic Root Derivation
>
> *In Pāṇini's grammar, every Sanskrit word derives from a verbal root (dhātu)
> with systematic affixes. Agam applies this to stdlib naming: a small set of
> canonical root verbs, with predictable suffixes, covers all API methods.*

---

## The Root Verb Table

Agam's standard library uses exactly **30 root verbs** organized into 7 action categories.
Every method in the stdlib is derived from one of these roots plus systematic suffixes.

### Category 1: Data Access (Reading)

| Root Verb | Semantics | Derived Forms |
|-----------|-----------|---------------|
| **`get`** | Retrieve by key/index (may fail) | `get`, `get_or`, `get_mut` |
| **`read`** | Sequential byte/char access | `read`, `read_all`, `read_line`, `read_to` |
| **`find`** | Search for element by predicate | `find`, `find_all`, `find_last`, `find_index` |
| **`peek`** | Observe without consuming | `peek`, `peek_at`, `peek_last` |

### Category 2: Data Mutation (Writing)

| Root Verb | Semantics | Derived Forms |
|-----------|-----------|---------------|
| **`add`** | Insert element into collection | `add`, `add_all`, `add_at`, `add_first` |
| **`set`** | Replace value at key/index | `set`, `set_all`, `set_default` |
| **`remove`** | Remove element from collection | `remove`, `remove_all`, `remove_at`, `remove_last`, `remove_first` |
| **`write`** | Sequential byte/char output | `write`, `write_all`, `write_line`, `write_to` |
| **`clear`** | Remove all elements | `clear` |

### Category 3: Transformation

| Root Verb | Semantics | Derived Forms |
|-----------|-----------|---------------|
| **`map`** | Transform each element | `map`, `map_err`, `map_or` |
| **`filter`** | Select elements by predicate | `filter`, `filter_map` |
| **`fold`** | Reduce to single value | `fold`, `fold_right` |
| **`sort`** | Order elements | `sort`, `sort_by`, `sort_desc` |
| **`group`** | Partition by key | `group`, `group_by` |
| **`flat`** | Flatten nested structure | `flat`, `flat_map` |

### Category 4: Construction & Conversion

| Root Verb | Semantics | Derived Forms |
|-----------|-----------|---------------|
| **`new`** | Primary constructor | `new`, `new_with` |
| **`from`** | Convert from another type | `from`, `from_str`, `from_bytes`, `from_iter` |
| **`to`** | Convert to another type | `to_string`, `to_bytes`, `to_vec`, `to_iter` |
| **`build`** | Builder pattern finalization | `build`, `build_with` |
| **`clone`** | Deep copy | `clone`, `clone_from` |

### Category 5: Lifecycle & Resource

| Root Verb | Semantics | Derived Forms |
|-----------|-----------|---------------|
| **`open`** | Acquire resource handle | `open`, `open_or`, `open_with` |
| **`close`** | Release resource handle | `close`, `close_all` |
| **`drop`** | Destructor / finalize | `drop` |

### Category 6: Communication & IO

| Root Verb | Semantics | Derived Forms |
|-----------|-----------|---------------|
| **`send`** | Push data to a channel/socket | `send`, `send_all`, `send_to` |
| **`receive`** | Pull data from a channel/socket | `receive`, `receive_all`, `receive_from` |
| **`connect`** | Establish connection | `connect`, `connect_to`, `connect_with` |
| **`listen`** | Wait for incoming connections | `listen`, `listen_on` |

### Category 7: Inspection & Query

| Root Verb | Semantics | Derived Forms |
|-----------|-----------|---------------|
| **`len`** | Count of elements | `len` |
| **`is`** | Boolean property check | `is_empty`, `is_some`, `is_err`, `is_valid` |
| **`has`** | Containment check | `has`, `has_key`, `has_all` |
| **`check`** | Validate state | `check`, `check_all` |

---

## Suffix System

Suffixes modify root verbs systematically. Each suffix has a fixed meaning:

| Suffix | Meaning | Example |
|--------|---------|---------|
| `_all` | Batch operation on all elements | `read_all`, `add_all`, `remove_all` |
| `_at` | Operation at specific index/position | `add_at`, `remove_at`, `peek_at` |
| `_by` | Parameterized by function/key | `sort_by`, `group_by`, `find_by` |
| `_or` | With fallback value | `get_or`, `open_or`, `map_or` |
| `_to` | With explicit destination | `write_to`, `send_to`, `read_to`, `connect_to` |
| `_from` | With explicit source | `receive_from`, `clone_from`, `from_str` |
| `_with` | With configuration/options | `new_with`, `open_with`, `build_with`, `connect_with` |
| `_mut` | Mutable access variant | `get_mut`, `iter_mut` |
| `_first` | Operation on first element | `add_first`, `remove_first` |
| `_last` | Operation on last element | `find_last`, `peek_last`, `remove_last` |
| `_line` | Line-oriented I/O variant | `read_line`, `write_line` |
| `_err` | Error-path variant | `map_err`, `is_err` |
| `_desc` | Descending/reverse order | `sort_desc` |
| `_index` | Returns index instead of element | `find_index` |
| `_on` | Bind to address/port/event | `listen_on` |

---

## Anti-Patterns (Rejected Verbs)

These verbs are **banned** from the stdlib. Use the canonical root instead:

| ❌ Rejected | ✅ Canonical Root | Reason |
|------------|-------------------|--------|
| `append` | `add` | Redundant synonym |
| `push` | `add` | Container-specific jargon |
| `insert` | `add_at` | Implies position; use suffix |
| `extend` | `add_all` | Redundant synonym |
| `pop` | `remove_last` | Container-specific jargon |
| `delete` | `remove` | Redundant synonym |
| `fetch` | `get` | Redundant synonym |
| `load` | `read` | Redundant synonym |
| `store` | `write` | Redundant synonym |
| `put` | `set` / `write` | Ambiguous |
| `emit` | `send` | Domain jargon |
| `consume` | `receive` | Domain jargon |
| `lookup` | `find` / `get` | Redundant synonym |
| `contains` | `has` | Verb inconsistency (use `has`) |
| `exists` | `has` | Verb inconsistency |
| `count` | `len` | Redundant synonym |
| `size` | `len` | Redundant synonym |
| `length` | `len` | Use short form always |
| `create` | `new` | Redundant synonym |
| `make` | `new` | Redundant synonym |
| `destroy` | `drop` / `close` | Context-dependent; pick one |
| `dispose` | `drop` / `close` | C# jargon |
| `join` | `connect` / `add_all` | Ambiguous |
| `split` | `group` / specific name | Ambiguous |

---

## Naming Decision Process

When naming a new stdlib method, follow this decision tree:

```
1. What ACTION does this method perform?
   → Find the matching category (Access, Mutation, Transform, etc.)

2. What ROOT VERB fits?
   → Use the canonical root from that category
   
3. Does it need a SUFFIX?
   → _all (batch), _at (position), _by (parameterized), etc.
   
4. Is the name already taken by another method on this type?
   → If yes, you may be creating a redundant API. Reconsider.
   
5. Does the name read naturally when chained?
   → list.add(x).sort().take(10)  ✓
   → list.insert(x).arrange().limit(10)  ✗ (violations everywhere)
```

---

## Examples: Stdlib Compliance

### List<T>
```agam
# ✅ All methods derived from root table:
list.add(item)           # root: add
list.add_all(items)      # root: add, suffix: _all
list.add_at(index, item) # root: add, suffix: _at
list.remove(item)        # root: remove
list.remove_at(index)    # root: remove, suffix: _at
list.remove_last()       # root: remove, suffix: _last
list.get(index)          # root: get
list.get_or(index, def)  # root: get, suffix: _or
list.find(predicate)     # root: find
list.sort()              # root: sort
list.sort_by(key_fn)     # root: sort, suffix: _by
list.len()               # root: len
list.is_empty()          # root: is, property: empty
list.has(item)           # root: has
list.clear()             # root: clear
list.clone()             # root: clone
```

### Map<K, V>
```agam
map.add(key, value)      # root: add
map.add_all(pairs)       # root: add, suffix: _all
map.set(key, value)      # root: set (overwrites)
map.get(key)             # root: get (returns Option)
map.get_or(key, default) # root: get, suffix: _or
map.remove(key)          # root: remove
map.has(key)             # root: has (not .contains_key!)
map.has_key(key)         # root: has, suffix: _key (explicit)
map.len()                # root: len
map.is_empty()           # root: is
map.clear()              # root: clear
map.keys()               # accessor (not an action verb)
map.values()             # accessor
```

### File
```agam
file = File.open(path)          # root: open
file = File.open_with(path, opts) # root: open, suffix: _with
content = file.read_all()       # root: read, suffix: _all
line = file.read_line()         # root: read, suffix: _line
file.write(data)                # root: write
file.write_all(data)            # root: write, suffix: _all
file.write_line(text)           # root: write, suffix: _line
file.close()                    # root: close
```

---

## Cross-Reference with Vibhakti

When a root verb needs semantic role labels (Principle 2), apply both systems together:

```agam
# Dhātu root: 'send'   +   Vibhakti roles: from/to
fn send(from source: Socket, to target: Address, data payload: Bytes) -> Result<usize, IoError>

# Dhātu root: 'read'   +   Vibhakti roles: from/into
fn read(from source: File, into buffer: &mut Bytes, count n: usize) -> Result<usize, IoError>
```

The root verb tells you **what** happens. The role labels tell you **who participates** and in **what capacity**.
