//! High-throughput Metrics Registry and Prometheus/OTLP Exporters.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricKind {
    Counter,
    Gauge,
    Histogram,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricValue {
    pub name: String,
    pub kind: MetricKind,
    pub value: f64,
    pub labels: BTreeMap<String, String>,
    pub timestamp_nanos: u64,
}

/// Global/Thread-safe metrics registry.
#[derive(Debug, Default)]
pub struct MetricsRegistry {
    counters: RwLock<BTreeMap<String, f64>>,
    gauges: RwLock<BTreeMap<String, f64>>,
    histograms: RwLock<BTreeMap<String, Vec<f64>>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn make_key(name: &str, labels: &[(&str, &str)]) -> String {
        if labels.is_empty() {
            name.to_string()
        } else {
            let label_str = labels
                .iter()
                .map(|(k, v)| format!("{k}=\"{v}\""))
                .collect::<Vec<_>>()
                .join(",");
            format!("{name}{{{label_str}}}")
        }
    }

    pub fn increment_counter(&self, name: &str, delta: f64, labels: &[(&str, &str)]) {
        let key = Self::make_key(name, labels);
        let mut counters = self.counters.write().unwrap();
        *counters.entry(key).or_insert(0.0) += delta;
    }

    pub fn set_gauge(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        let key = Self::make_key(name, labels);
        let mut gauges = self.gauges.write().unwrap();
        gauges.insert(key, value);
    }

    pub fn observe_histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        let key = Self::make_key(name, labels);
        let mut hists = self.histograms.write().unwrap();
        hists.entry(key).or_default().push(value);
    }

    /// Export metrics in Prometheus text exposition format.
    pub fn export_prometheus(&self) -> String {
        let mut out = String::new();

        let counters = self.counters.read().unwrap();
        for (k, v) in counters.iter() {
            out.push_str(&format!("{k} {v}\n"));
        }

        let gauges = self.gauges.read().unwrap();
        for (k, v) in gauges.iter() {
            out.push_str(&format!("{k} {v}\n"));
        }

        let histograms = self.histograms.read().unwrap();
        for (k, values) in histograms.iter() {
            let count = values.len();
            let sum: f64 = values.iter().sum();
            let base = k.split('{').next().unwrap_or(k);
            let labels = if k.contains('{') {
                k[k.find('{').unwrap()..].to_string()
            } else {
                String::new()
            };
            out.push_str(&format!("{base}_count{labels} {count}\n"));
            out.push_str(&format!("{base}_sum{labels} {sum}\n"));
        }

        out
    }

    /// Export metrics in OTLP JSON format.
    pub fn export_otlp_metrics_json(&self) -> String {
        let now_str = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_string();

        let counters = self.counters.read().unwrap();
        let counter_metrics: Vec<serde_json::Value> = counters
            .iter()
            .map(|(k, v)| {
                serde_json::json!({
                    "name": k,
                    "sum": {
                        "dataPoints": [{
                            "timeUnixNano": now_str,
                            "asDouble": v
                        }],
                        "isMonotonic": true
                    }
                })
            })
            .collect();

        serde_json::to_string_pretty(&serde_json::json!({
            "resourceMetrics": [{
                "resource": {
                    "attributes": [{ "key": "service.name", "value": { "stringValue": "agam-compiler" } }]
                },
                "scopeMetrics": [{
                    "scope": { "name": "agam_profile::metrics" },
                    "metrics": counter_metrics
                }]
            }]
        })).unwrap_or_default()
    }
}
