# Chapter 31: Compiler Profiling, Observability & Performance Measurement

> **Part VII: Advanced Tooling, Testing & Ecosystem Engineering**  
> **Compiler Module Focus**: [`agam_profile`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_profile), [`agam_runtime`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_runtime)

---

## 31.1 Compiler Phase Profiling

The Agam compiler includes built-in profiling instrumentation that measures the wall-clock time and memory consumption of each compilation phase:

```bash
# Profile a compilation with phase timing breakdown
agamc build --timings src/main.agam

# Output:
#   Phase            Time      Memory
#   ─────            ────      ──────
#   Lexing           1.2 ms    0.4 MB
#   Parsing          3.8 ms    1.2 MB
#   Sema             12.1 ms   3.6 MB
#   HIR Lowering     5.4 ms    2.1 MB
#   MIR Generation   8.7 ms    4.8 MB
#   MIR Optimization 18.3 ms   5.2 MB
#   LLVM Codegen     42.6 ms   28.4 MB
#   Linking          89.1 ms   —
#   ─────────────────────────────────
#   Total            181.2 ms  45.7 MB
```

### Flamegraph Generation

For deep analysis of compilation bottlenecks, generate a flamegraph of compiler internals:

```bash
# Generate a flamegraph SVG
agamc build --profile flamegraph src/main.agam
# Writes: target/profile/compile_flamegraph.svg
```

The flamegraph captures the call stack of every compiler phase, showing exactly which optimization pass or type inference step is consuming the most time. This is critical for identifying regressions in compiler performance.

---

## 31.2 OpenTelemetry Distributed Observability

Agam provides first-class integration with the **OpenTelemetry** standard for distributed tracing, metrics, and structured logging in production applications.

### Architecture

```text
Agam Application
      │
      ├── @trace annotations ──► agam_profile::TracerProvider
      │                              │
      │                              ▼
      ├── @metric annotations ──► agam_profile::MetricExporter
      │                              │
      │                              ▼
      └── Structured logging ───► agam_profile::LogExporter
                                      │
                                      ▼
                               OTLP gRPC/HTTP Exporter
                                      │
                          ┌───────────┴───────────┐
                          ▼                       ▼
                    Jaeger / Tempo          Prometheus / Grafana
                    (Trace Backend)         (Metric Backend)
```

### Trace Annotations (`@trace`)

The `@trace` annotation automatically instruments a function with OpenTelemetry span creation and propagation:

```agam
@trace
fn process_order(order: Order) -> Result[Receipt, OrderError] {
    // A span named "process_order" is automatically created
    // with attributes: order.id, order.total

    let validated = validate_order(order)?;  // Child span created
    let payment = charge_payment(validated)?; // Child span created
    let receipt = generate_receipt(payment);  // Child span created

    return Result.Ok(receipt);
}

@trace
fn validate_order(order: Order) -> Result[Order, OrderError] {
    // Nested span: "validate_order" is a child of "process_order"
    if order.items.len() == 0 {
        return Result.Err(OrderError.EmptyCart);
    }
    return Result.Ok(order);
}
```

### Compiler Lowering for `@trace`

The `@trace` annotation is lowered by the compiler into explicit span management code:

```text
// Source:
@trace
fn foo(x: Int) -> Int { return x + 1; }

// Lowered MIR equivalent:
fn foo(x: Int) -> Int {
    let _span = agam_profile::tracer().start_span("foo");
    _span.set_attribute("x", x.to_string());
    let _result = { return x + 1; };
    _span.set_status(StatusCode.Ok);
    _span.end();
    return _result;
}

// On error/panic, the span records the error:
//   _span.set_status(StatusCode.Error);
//   _span.record_exception(err);
```

### Metric Annotations (`@metric`)

The `@metric` annotation automatically records counters, histograms, and gauges:

```agam
@metric(counter = "orders.processed", histogram = "orders.latency_ms")
fn process_order(order: Order) -> Result[Receipt, OrderError] {
    // Counter incremented on each call
    // Histogram records execution duration in milliseconds
    // ...
}

@metric(gauge = "connections.active")
fn get_active_connections() -> Int {
    return connection_pool.active_count();
}
```

### OTLP Export Configuration

Configure the telemetry export endpoint in `agam.toml`:

```toml
[telemetry]
exporter = "otlp"
endpoint = "http://localhost:4317"
protocol = "grpc"                    # or "http/protobuf"
service_name = "my-agam-service"
sample_rate = 1.0                    # 100% sampling

[telemetry.resource]
deployment.environment = "production"
service.version = "1.2.0"
```

---

## 31.3 Application Benchmarking

### Built-In Benchmark Harness

Agam provides a built-in benchmark harness for measuring function performance with statistical rigor:

```agam
@bench
fn bench_matrix_multiply() {
    let A = Tensor.random([256, 256]);
    let B = Tensor.random([256, 256]);
    // The harness automatically runs this N times and reports statistics
    let C = A * B;
}

@bench(iterations = 10000, warmup = 1000)
fn bench_fibonacci() {
    let result = fibonacci(30);
}
```

```bash
# Run all benchmarks
agamc bench

# Output:
#   Benchmark                    Iterations   Mean        Std Dev     Min         Max
#   ─────────                    ──────────   ────        ───────     ───         ───
#   bench_matrix_multiply        1000         2.34 ms     ±0.12 ms   2.18 ms     2.71 ms
#   bench_fibonacci              10000        0.87 μs     ±0.03 μs   0.82 μs     1.12 μs
```

### Statistical Methodology

The benchmark harness uses **Criterion-style** statistical analysis:

1. **Warmup phase:** Run the benchmark function N times to warm CPU caches and JIT compilation (discarded).
2. **Measurement phase:** Run the benchmark function M times, recording wall-clock time for each iteration.
3. **Statistical analysis:**
   - Compute **mean**, **median**, **standard deviation**, **min**, **max**
   - Apply the **bootstrap resampling** method to estimate confidence intervals
   - Detect **outliers** using the modified Z-score method (|Z| > 3.5)
4. **Regression detection:** Compare against previous benchmark results stored in `target/bench/baseline.json`. Flag regressions exceeding 5%.

### Comparative Benchmarking

```bash
# Save current results as baseline
agamc bench --save-baseline v1.0

# After code changes, compare against baseline
agamc bench --compare v1.0

# Output highlights regressions:
#   bench_matrix_multiply: 2.34 ms → 2.51 ms (+7.3%) ⚠️ REGRESSION
#   bench_fibonacci:       0.87 μs → 0.85 μs (-2.3%) ✅ improved
```

---

## 31.4 Runtime Performance Instrumentation

### Memory Allocation Profiling

```bash
# Profile heap allocations during execution
agamc run --profile alloc src/main.agam

# Output:
#   Total Allocations: 1,247
#   Total Bytes:       2.3 MB
#   Peak Live Bytes:   890 KB
#   Allocation Sites:
#     src/main.agam:42   Vec.push()     × 1024    (820 KB)
#     src/main.agam:58   String.concat  × 200     (64 KB)
#     src/lib.agam:15    HashMap.insert × 23      (18 KB)
```

### CPU Performance Counters

On supported platforms, the profiler reads hardware performance counters:

```bash
# Profile with hardware counters (Linux perf_events, Windows ETW)
agamc run --profile hwcounters src/main.agam

# Output:
#   Instructions:        12,847,291
#   Cycles:              4,128,903
#   IPC:                 3.11
#   Cache Misses (L1d):  847 (0.007%)
#   Branch Misses:       1,204 (0.09%)
#   TLB Misses:          12
```

---

## 31.5 Continuous Integration Performance Gates

For CI/CD pipelines, benchmarks can enforce performance budgets:

```toml
# In agam.toml
[bench.budget]
bench_matrix_multiply = { max_mean_ms = 3.0 }
bench_fibonacci = { max_mean_us = 1.0 }

# CI command — exits with non-zero code on budget violations
# agamc bench --enforce-budget
```

This ensures that no commit can degrade critical path performance beyond defined thresholds.
