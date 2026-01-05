//! Prometheus-style metrics collection.
//!
//! Provides counters, gauges, and histograms.

use crate::core::now;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// A counter metric (monotonically increasing).
#[derive(Debug, Default)]
pub struct Counter {
    value: AtomicU64,
    labels: HashMap<String, String>,
}

impl Counter {
    /// Create a new counter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with labels.
    pub fn with_labels(labels: HashMap<String, String>) -> Self {
        Self {
            value: AtomicU64::new(0),
            labels,
        }
    }

    /// Increment by 1.
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment by amount.
    pub fn add(&self, amount: u64) {
        self.value.fetch_add(amount, Ordering::Relaxed);
    }

    /// Get current value.
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Reset to zero.
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

/// A gauge metric (can go up or down).
#[derive(Debug, Default)]
pub struct Gauge {
    value: AtomicU64, // Store as bits for f64
    labels: HashMap<String, String>,
}

impl Gauge {
    /// Create a new gauge.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with labels.
    pub fn with_labels(labels: HashMap<String, String>) -> Self {
        Self {
            value: AtomicU64::new(0),
            labels,
        }
    }

    /// Set the gauge value.
    pub fn set(&self, value: f64) {
        self.value.store(value.to_bits(), Ordering::Relaxed);
    }

    /// Get current value.
    pub fn get(&self) -> f64 {
        f64::from_bits(self.value.load(Ordering::Relaxed))
    }

    /// Increment by amount.
    pub fn add(&self, amount: f64) {
        let current = self.get();
        self.set(current + amount);
    }

    /// Decrement by amount.
    pub fn sub(&self, amount: f64) {
        let current = self.get();
        self.set(current - amount);
    }
}

/// A histogram metric for measuring distributions.
#[derive(Debug)]
pub struct Histogram {
    buckets: Vec<f64>,
    bucket_counts: Vec<AtomicU64>,
    sum: AtomicU64,
    count: AtomicU64,
    labels: HashMap<String, String>,
}

impl Histogram {
    /// Create a new histogram with default buckets.
    pub fn new() -> Self {
        Self::with_buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
    }

    /// Create with custom buckets.
    pub fn with_buckets(buckets: Vec<f64>) -> Self {
        let bucket_counts = buckets.iter().map(|_| AtomicU64::new(0)).collect();
        Self {
            buckets,
            bucket_counts,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
            labels: HashMap::new(),
        }
    }

    /// Observe a value.
    pub fn observe(&self, value: f64) {
        // Update bucket counts
        for (i, bucket) in self.buckets.iter().enumerate() {
            if value <= *bucket {
                self.bucket_counts[i].fetch_add(1, Ordering::Relaxed);
            }
        }

        // Update sum and count
        let sum_bits = self.sum.load(Ordering::Relaxed);
        let current_sum = f64::from_bits(sum_bits);
        self.sum.store((current_sum + value).to_bits(), Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get observation count.
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Get sum of observations.
    pub fn sum(&self) -> f64 {
        f64::from_bits(self.sum.load(Ordering::Relaxed))
    }

    /// Get mean value.
    pub fn mean(&self) -> f64 {
        let count = self.count();
        if count == 0 {
            0.0
        } else {
            self.sum() / count as f64
        }
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Metric metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricInfo {
    /// Metric name
    pub name: String,
    /// Help text
    pub help: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Labels
    pub labels: HashMap<String, String>,
}

/// Metric type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Metrics collector for the application.
pub struct MetricsCollector {
    /// Counters
    counters: RwLock<HashMap<String, Counter>>,
    /// Gauges
    gauges: RwLock<HashMap<String, Gauge>>,
    /// Histograms
    histograms: RwLock<HashMap<String, Histogram>>,
    /// Metric info
    info: RwLock<HashMap<String, MetricInfo>>,
}

impl MetricsCollector {
    /// Create a new metrics collector.
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
            info: RwLock::new(HashMap::new()),
        }
    }

    /// Register a counter.
    pub fn register_counter(&self, name: &str, help: &str) {
        let mut counters = self.counters.write().unwrap();
        counters.insert(name.to_string(), Counter::new());

        let mut info = self.info.write().unwrap();
        info.insert(name.to_string(), MetricInfo {
            name: name.to_string(),
            help: help.to_string(),
            metric_type: MetricType::Counter,
            labels: HashMap::new(),
        });
    }

    /// Register a gauge.
    pub fn register_gauge(&self, name: &str, help: &str) {
        let mut gauges = self.gauges.write().unwrap();
        gauges.insert(name.to_string(), Gauge::new());

        let mut info = self.info.write().unwrap();
        info.insert(name.to_string(), MetricInfo {
            name: name.to_string(),
            help: help.to_string(),
            metric_type: MetricType::Gauge,
            labels: HashMap::new(),
        });
    }

    /// Register a histogram.
    pub fn register_histogram(&self, name: &str, help: &str, buckets: Vec<f64>) {
        let mut histograms = self.histograms.write().unwrap();
        histograms.insert(name.to_string(), Histogram::with_buckets(buckets));

        let mut info = self.info.write().unwrap();
        info.insert(name.to_string(), MetricInfo {
            name: name.to_string(),
            help: help.to_string(),
            metric_type: MetricType::Histogram,
            labels: HashMap::new(),
        });
    }

    /// Increment a counter.
    pub fn inc_counter(&self, name: &str) {
        if let Some(counter) = self.counters.read().unwrap().get(name) {
            counter.inc();
        }
    }

    /// Add to counter.
    pub fn add_counter(&self, name: &str, amount: u64) {
        if let Some(counter) = self.counters.read().unwrap().get(name) {
            counter.add(amount);
        }
    }

    /// Set gauge value.
    pub fn set_gauge(&self, name: &str, value: f64) {
        if let Some(gauge) = self.gauges.read().unwrap().get(name) {
            gauge.set(value);
        }
    }

    /// Observe histogram value.
    pub fn observe_histogram(&self, name: &str, value: f64) {
        if let Some(histogram) = self.histograms.read().unwrap().get(name) {
            histogram.observe(value);
        }
    }

    /// Get counter value.
    pub fn get_counter(&self, name: &str) -> Option<u64> {
        self.counters.read().unwrap().get(name).map(|c| c.get())
    }

    /// Get gauge value.
    pub fn get_gauge(&self, name: &str) -> Option<f64> {
        self.gauges.read().unwrap().get(name).map(|g| g.get())
    }

    /// Export metrics in Prometheus format.
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Export counters
        for (name, counter) in self.counters.read().unwrap().iter() {
            if let Some(info) = self.info.read().unwrap().get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, info.help));
                output.push_str(&format!("# TYPE {} counter\n", name));
            }
            output.push_str(&format!("{} {}\n", name, counter.get()));
        }

        // Export gauges
        for (name, gauge) in self.gauges.read().unwrap().iter() {
            if let Some(info) = self.info.read().unwrap().get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, info.help));
                output.push_str(&format!("# TYPE {} gauge\n", name));
            }
            output.push_str(&format!("{} {}\n", name, gauge.get()));
        }

        // Export histograms
        for (name, histogram) in self.histograms.read().unwrap().iter() {
            if let Some(info) = self.info.read().unwrap().get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, info.help));
                output.push_str(&format!("# TYPE {} histogram\n", name));
            }
            output.push_str(&format!("{}_sum {}\n", name, histogram.sum()));
            output.push_str(&format!("{}_count {}\n", name, histogram.count()));
        }

        output
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new();
        assert_eq!(counter.get(), 0);

        counter.inc();
        assert_eq!(counter.get(), 1);

        counter.add(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_gauge() {
        let gauge = Gauge::new();
        assert_eq!(gauge.get(), 0.0);

        gauge.set(10.5);
        assert!((gauge.get() - 10.5).abs() < 1e-10);

        gauge.add(2.0);
        assert!((gauge.get() - 12.5).abs() < 1e-10);

        gauge.sub(5.0);
        assert!((gauge.get() - 7.5).abs() < 1e-10);
    }

    #[test]
    fn test_histogram() {
        let histogram = Histogram::new();
        histogram.observe(0.1);
        histogram.observe(0.5);
        histogram.observe(1.0);

        assert_eq!(histogram.count(), 3);
        assert!((histogram.sum() - 1.6).abs() < 1e-10);
        assert!((histogram.mean() - 0.533).abs() < 0.01);
    }

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        collector.register_counter("requests_total", "Total requests");
        collector.register_gauge("active_connections", "Active connections");

        collector.inc_counter("requests_total");
        collector.inc_counter("requests_total");
        collector.set_gauge("active_connections", 10.0);

        assert_eq!(collector.get_counter("requests_total"), Some(2));
        assert_eq!(collector.get_gauge("active_connections"), Some(10.0));
    }

    #[test]
    fn test_prometheus_export() {
        let collector = MetricsCollector::new();
        collector.register_counter("test_counter", "Test counter");
        collector.inc_counter("test_counter");

        let output = collector.export_prometheus();
        assert!(output.contains("test_counter"));
        assert!(output.contains("HELP"));
        assert!(output.contains("TYPE"));
    }
}
