//! OpenTelemetry-compatible distributed tracing and span instrumentation engine.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_SPAN_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_TRACE_ID_LO: AtomicU64 = AtomicU64::new(1);

/// A 64-bit unique Span identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SpanId(pub u64);

impl SpanId {
    pub fn generate() -> Self {
        Self(NEXT_SPAN_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn to_hex(self) -> String {
        format!("{:016x}", self.0)
    }
}

/// A 128-bit unique Trace identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TraceId(pub u128);

impl TraceId {
    pub fn generate() -> Self {
        let hi = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let lo = NEXT_TRACE_ID_LO.fetch_add(1, Ordering::Relaxed) as u128;
        Self((hi << 64) | lo)
    }

    pub fn to_hex(self) -> String {
        format!("{:032x}", self.0)
    }
}

/// OpenTelemetry Span Kinds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanKind {
    #[default]
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
}

/// Span status.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStatus {
    #[default]
    Unset,
    Ok,
    Error(String),
}

/// A timed event recorded on a span.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp_nanos: u64,
    pub attributes: BTreeMap<String, String>,
}

/// An individual trace span.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub name: String,
    pub kind: SpanKind,
    pub start_time_nanos: u64,
    pub end_time_nanos: Option<u64>,
    pub status: SpanStatus,
    pub attributes: BTreeMap<String, String>,
    pub events: Vec<SpanEvent>,
}

impl Span {
    pub fn new(name: impl Into<String>, trace_id: TraceId, parent: Option<SpanId>) -> Self {
        let start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Self {
            trace_id,
            span_id: SpanId::generate(),
            parent_span_id: parent,
            name: name.into(),
            kind: SpanKind::Internal,
            start_time_nanos: start,
            end_time_nanos: None,
            status: SpanStatus::Unset,
            attributes: BTreeMap::new(),
            events: Vec::new(),
        }
    }

    pub fn finish(&mut self) {
        if self.end_time_nanos.is_none() {
            let end = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            self.end_time_nanos = Some(end);
            if self.status == SpanStatus::Unset {
                self.status = SpanStatus::Ok;
            }
        }
    }

    pub fn duration_nanos(&self) -> u64 {
        match self.end_time_nanos {
            Some(end) => end.saturating_sub(self.start_time_nanos),
            None => 0,
        }
    }

    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    pub fn record_event(&mut self, name: impl Into<String>, attrs: BTreeMap<String, String>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp_nanos: now,
            attributes: attrs,
        });
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.status = SpanStatus::Error(message.into());
    }
}

/// In-memory Tracer and Span Collector.
#[derive(Clone, Debug, Default)]
pub struct Tracer {
    pub current_trace_id: Option<TraceId>,
    pub completed_spans: Vec<Span>,
}

impl Tracer {
    pub fn new() -> Self {
        Self {
            current_trace_id: Some(TraceId::generate()),
            completed_spans: Vec::new(),
        }
    }

    pub fn trace_id(&mut self) -> TraceId {
        match self.current_trace_id {
            Some(id) => id,
            None => {
                let id = TraceId::generate();
                self.current_trace_id = Some(id);
                id
            }
        }
    }

    pub fn start_span(&mut self, name: &str, parent: Option<SpanId>) -> Span {
        let trace_id = self.trace_id();
        Span::new(name, trace_id, parent)
    }

    pub fn record_span(&mut self, mut span: Span) {
        span.finish();
        self.completed_spans.push(span);
    }

    /// Export completed spans in standard OTLP JSON trace format.
    pub fn export_otlp_json(&self) -> String {
        serde_json::to_string_pretty(&serde_json::json!({
            "resourceSpans": [{
                "resource": {
                    "attributes": [
                        { "key": "service.name", "value": { "stringValue": "agam-compiler" } },
                        { "key": "telemetry.sdk.language", "value": { "stringValue": "agam" } }
                    ]
                },
                "scopeSpans": [{
                    "scope": { "name": "agam_profile::telemetry" },
                    "spans": self.completed_spans.iter().map(|s| {
                        serde_json::json!({
                            "traceId": s.trace_id.to_hex(),
                            "spanId": s.span_id.to_hex(),
                            "parentSpanId": s.parent_span_id.map(|p| p.to_hex()).unwrap_or_default(),
                            "name": s.name,
                            "startTimeUnixNano": s.start_time_nanos.to_string(),
                            "endTimeUnixNano": s.end_time_nanos.unwrap_or(s.start_time_nanos).to_string(),
                            "attributes": s.attributes.iter().map(|(k, v)| {
                                serde_json::json!({ "key": k, "value": { "stringValue": v } })
                            }).collect::<Vec<_>>(),
                            "events": s.events.iter().map(|e| {
                                serde_json::json!({
                                    "name": e.name,
                                    "timeUnixNano": e.timestamp_nanos.to_string(),
                                    "attributes": e.attributes.iter().map(|(k, v)| {
                                        serde_json::json!({ "key": k, "value": { "stringValue": v } })
                                    }).collect::<Vec<_>>()
                                })
                            }).collect::<Vec<_>>()
                        })
                    }).collect::<Vec<_>>()
                }]
            }]
        })).unwrap_or_default()
    }
}
