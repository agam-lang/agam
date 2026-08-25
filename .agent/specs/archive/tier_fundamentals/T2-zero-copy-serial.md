# Phase T2-zero-copy-serial � Zero-Copy Serialization and IPC

**Status:** open
**Tier:** 2 (Runtime and Concurrency)
**Pillar:** 1 (Performance)

## Vision

Provide **zero-copy serialization** as a language-native capability — Agam structs can be directly memory-mapped, shared across processes, and transmitted over the network without serialization overhead.

## Motivation

In 2026, zero-copy serialization formats (Cap'n Proto, FlatBuffers, rkyv) are critical for:
- **IPC (Inter-Process Communication)**: Shared memory between Agam processes
- **Networking**: gRPC-alternative high-performance RPC
- **Persistence**: Memory-mapped file storage
- **GPU host-device transfer**: Avoid serialization when moving data to/from GPU

Agam's compiler knows struct layouts at compile time — it can generate zero-copy-compatible layouts automatically.

## Deliverables

### Compile-Time Layout Control
- [ ] `@packed` annotation: compiler generates zero-copy-compatible struct layout
- [ ] `@align(N)` for cache-line-aligned types (already in AST attributes)
- [ ] Automatic endianness handling for cross-platform serialization
- [ ] Compile-time size/offset calculations exported as constants

### Zero-Copy API
- [ ] `agam_std::serial::to_bytes(value) -> &[u8]` — zero-copy reference cast
- [ ] `agam_std::serial::from_bytes<T>(bytes) -> &T` — zero-copy deserialization
- [ ] Validation: runtime bounds and alignment checking for untrusted input
- [ ] Schema evolution: versioned structs with backward compatibility

### IPC Primitives
- [ ] `agam_std::ipc::SharedMemory` — cross-process shared memory regions
- [ ] `agam_std::ipc::Channel<T>` — type-safe IPC channel with zero-copy transfer
- [ ] Memory-mapped file I/O with zero-copy struct access
- [ ] Integration with daemon IPC (Phase T1-daemon-prewarm loopback protocol)

### Integration Formats
- [ ] ONNX tensor data: zero-copy from file to GPU buffer (coordinates with EDGE1)
- [ ] Cap'n Proto / FlatBuffers interop for existing ecosystems
- [ ] `@derive(Serialize, Deserialize)` for JSON/MessagePack (traditional serde)

## Design References

- **Cap'n Proto**: Zero-copy serialization with schema evolution. Created by Protobuf author.
- **FlatBuffers (Google)**: Zero-copy access to serialized data. Used in ML/game dev.
- **rkyv (Rust)**: Zero-copy deserialization framework. Fastest Rust serialization.
- **Agam advantage**: Compiler controls struct layout → can guarantee zero-copy compatibility at compile time.

## Responsible Crates

- `agam_sema` — `@packed` validation, layout computation
- `agam_codegen` — zero-copy-compatible struct emission
- `agam_std` — `serial`, `ipc` modules
- `agam_runtime` — shared memory, cross-process primitives

## Dependencies

- Phase T0-type-system (type system) — generic types for Channel<T>, from_bytes<T>
- Phase T0-object-model (object model) — struct layout control
- Phase T2-async-concurrency (async) — async IPC channels
