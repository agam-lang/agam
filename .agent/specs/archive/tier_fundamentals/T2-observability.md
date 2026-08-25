# Phase T2-observability — Native Observability and Telemetry

**Status:** complete
**Tier:** 2 (Runtime, Profiling, and Telemetry)

## Goal

Make Agam programs and compiler pipelines observable by default with high-throughput OpenTelemetry-compatible distributed tracing and Prometheus/OTLP metrics export.

## Deliverables

- [x] **OpenTelemetry Distributed Tracing (`agam_profile::telemetry`)**:
  - `SpanId` (64-bit hex format), `TraceId` (128-bit hex format), `SpanKind`, `SpanStatus`, `SpanEvent`.
  - `Span` lifecycle management (`start_span`, `finish`, `duration_nanos`, attributes, timed events, error states).
  - `Tracer` span collector and standard OTLP JSON trace export (`export_otlp_json()`).
- [x] **Metrics Registry & Exporters (`agam_profile::metrics`)**:
  - High-performance `MetricsRegistry` with support for `Counter`, `Gauge`, and `Histogram`.
  - Dimension labeling and key indexing.
  - Standard Prometheus exposition format exporter (`export_prometheus()`).
  - Standard OTLP JSON metric exporter (`export_otlp_metrics_json()`).
- [x] **Verification**:
  - `test_telemetry_span_lifecycle_and_otlp_export`
  - `test_metrics_registry_and_prometheus_export`
  - 100% test pass rate across all 27 workspace crates.

## Test Results
- 21/21 tests pass in `agam_profile`
- 100% test pass rate across all 27 workspace crates
- 0 Clippy warnings (`-D warnings`)
- 100% formatting compliance (`cargo fmt --check`)
