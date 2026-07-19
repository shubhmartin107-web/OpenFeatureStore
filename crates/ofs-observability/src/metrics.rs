use prometheus::{Counter, CounterVec, HistogramOpts, HistogramVec, Opts, Registry, TextEncoder};

#[derive(Clone)]
pub struct OfsMetrics {
    pub http_requests: CounterVec,
    pub http_request_duration: HistogramVec,
    pub store_reads: CounterVec,
    pub store_writes: CounterVec,
    pub cache_hits: CounterVec,
    pub cache_misses: CounterVec,
    pub feature_requests: CounterVec,
    pub materialization_rows: Counter,
    pub registry: Registry,
}

impl OfsMetrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let http_requests = CounterVec::new(
            Opts::new("http_requests_total", "Total HTTP requests"),
            &["method", "path", "status"],
        )
        .unwrap();

        let http_request_duration = HistogramVec::new(
            HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request latency in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["method", "path", "status"],
        )
        .unwrap();

        let store_reads = CounterVec::new(
            Opts::new("store_reads_total", "Total store read operations"),
            &["store_type", "backend"],
        )
        .unwrap();

        let store_writes = CounterVec::new(
            Opts::new("store_writes_total", "Total store write operations"),
            &["store_type", "backend"],
        )
        .unwrap();

        let cache_hits =
            CounterVec::new(Opts::new("cache_hits_total", "Total cache hits"), &["tier"]).unwrap();

        let cache_misses = CounterVec::new(
            Opts::new("cache_misses_total", "Total cache misses"),
            &["tier"],
        )
        .unwrap();

        let feature_requests = CounterVec::new(
            Opts::new("feature_requests_total", "Total feature requests"),
            &["feature_view", "status"],
        )
        .unwrap();

        let materialization_rows =
            Counter::new("materialization_rows_total", "Total rows materialized").unwrap();

        registry.register(Box::new(http_requests.clone())).unwrap();
        registry
            .register(Box::new(http_request_duration.clone()))
            .unwrap();
        registry.register(Box::new(store_reads.clone())).unwrap();
        registry.register(Box::new(store_writes.clone())).unwrap();
        registry.register(Box::new(cache_hits.clone())).unwrap();
        registry.register(Box::new(cache_misses.clone())).unwrap();
        registry
            .register(Box::new(feature_requests.clone()))
            .unwrap();
        registry
            .register(Box::new(materialization_rows.clone()))
            .unwrap();

        Self {
            http_requests,
            http_request_duration,
            store_reads,
            store_writes,
            cache_hits,
            cache_misses,
            feature_requests,
            materialization_rows,
            registry,
        }
    }

    pub fn record_request(&self, method: &str, path: &str, status: u16, duration_secs: f64) {
        self.http_requests
            .with_label_values(&[method, path, &status.to_string()])
            .inc();
        self.http_request_duration
            .with_label_values(&[method, path, &status.to_string()])
            .observe(duration_secs);
    }

    pub fn record_store_read(&self, store_type: &str, backend: &str) {
        self.store_reads
            .with_label_values(&[store_type, backend])
            .inc();
    }

    pub fn record_store_write(&self, store_type: &str, backend: &str) {
        self.store_writes
            .with_label_values(&[store_type, backend])
            .inc();
    }

    pub fn record_cache_hit(&self, tier: &str) {
        self.cache_hits.with_label_values(&[tier]).inc();
    }

    pub fn record_cache_miss(&self, tier: &str) {
        self.cache_misses.with_label_values(&[tier]).inc();
    }

    pub fn record_feature_request(&self, feature_view: &str, status: &str) {
        self.feature_requests
            .with_label_values(&[feature_view, status])
            .inc();
    }

    pub fn record_materialization_rows(&self, count: u64) {
        self.materialization_rows.inc_by(count as f64);
    }

    pub fn prometheus_output(&self) -> String {
        let metric_families = self.registry.gather();
        TextEncoder::new()
            .encode_to_string(&metric_families)
            .unwrap_or_default()
    }
}

impl Default for OfsMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let m = OfsMetrics::new();
        m.record_request("GET", "/v1/health", 200, 0.001);
        m.record_store_read("online", "sqlite");
        m.record_store_write("offline", "duckdb");
        m.record_cache_hit("local");
        m.record_cache_miss("redis");
        m.record_feature_request("my_fv", "success");
        m.record_materialization_rows(1000);
        let output = m.prometheus_output();
        assert!(output.contains("http_requests_total"));
        assert!(output.contains("store_reads_total"));
        assert!(output.contains("materialization_rows_total"));
    }
}
