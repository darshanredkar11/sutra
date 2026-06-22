use prometheus::{
    Histogram, HistogramOpts, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
    Result as PromResult,
};
use std::sync::OnceLock;

/// Global Prometheus registry.
pub fn global_registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(Registry::new)
}

/// Creates and registers a counter. Idempotent — returns existing counter if already registered.
pub fn counter(name: &str, help: &str) -> PromResult<IntCounter> {
    let c = IntCounter::new(name, help)?;
    match global_registry().register(Box::new(c.clone())) {
        Ok(_) => Ok(c),
        Err(prometheus::Error::AlreadyReg) => Ok(c),
        Err(e) => Err(e),
    }
}

/// Creates and registers a counter with labels. Idempotent.
pub fn counter_vec(name: &str, help: &str, labels: &[&str]) -> PromResult<IntCounterVec> {
    let cv = IntCounterVec::new(Opts::new(name, help), labels)?;
    match global_registry().register(Box::new(cv.clone())) {
        Ok(_) => Ok(cv),
        Err(prometheus::Error::AlreadyReg) => Ok(cv),
        Err(e) => Err(e),
    }
}

/// Creates and registers a gauge. Idempotent.
pub fn gauge(name: &str, help: &str) -> PromResult<IntGauge> {
    let g = IntGauge::new(name, help)?;
    match global_registry().register(Box::new(g.clone())) {
        Ok(_) => Ok(g),
        Err(prometheus::Error::AlreadyReg) => Ok(g),
        Err(e) => Err(e),
    }
}

/// Creates and registers a gauge with labels. Idempotent.
pub fn gauge_vec(name: &str, help: &str, labels: &[&str]) -> PromResult<IntGaugeVec> {
    let gv = IntGaugeVec::new(Opts::new(name, help), labels)?;
    match global_registry().register(Box::new(gv.clone())) {
        Ok(_) => Ok(gv),
        Err(prometheus::Error::AlreadyReg) => Ok(gv),
        Err(e) => Err(e),
    }
}

/// Creates and registers a histogram. Idempotent.
pub fn histogram(name: &str, help: &str, buckets: Vec<f64>) -> PromResult<Histogram> {
    let h = Histogram::with_opts(HistogramOpts::new(name, help).buckets(buckets))?;
    match global_registry().register(Box::new(h.clone())) {
        Ok(_) => Ok(h),
        Err(prometheus::Error::AlreadyReg) => Ok(h),
        Err(e) => Err(e),
    }
}

/// Metrics for the analysis pipeline.
#[derive(Debug, Clone)]
pub struct AnalysisMetrics {
    pub scans_total: IntCounter,
    pub files_scanned: IntCounter,
    pub findings_total: IntCounterVec,
    pub scan_duration_ms: Histogram,
    pub active_engines: IntGauge,
}

impl AnalysisMetrics {
    /// Creates new analysis metrics. Idempotent — safe to call multiple times.
    pub fn new() -> PromResult<Self> {
        Ok(Self {
            scans_total: counter("sutra_scans_total", "Total number of analysis scans")?,
            files_scanned: counter("sutra_files_scanned", "Total files scanned")?,
            findings_total: counter_vec(
                "sutra_findings_total",
                "Total findings by severity",
                &["severity"],
            )?,
            scan_duration_ms: histogram(
                "sutra_scan_duration_ms",
                "Scan duration in milliseconds",
                vec![10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 30000.0],
            )?,
            active_engines: gauge("sutra_active_engines", "Number of active analysis engines")?,
        })
    }

    /// Returns the global singleton analysis metrics, creating it once.
    pub fn global() -> &'static AnalysisMetrics {
        static METRICS: OnceLock<AnalysisMetrics> = OnceLock::new();
        METRICS.get_or_init(|| Self::new().expect("failed to create global analysis metrics"))
    }

    pub fn record_scan(&self, duration_ms: f64, file_count: u32) {
        self.scans_total.inc();
        self.files_scanned.inc_by(file_count as u64);
        self.scan_duration_ms.observe(duration_ms);
    }

    pub fn record_finding(&self, severity: &str) {
        self.findings_total.with_label_values(&[severity]).inc();
    }

    pub fn set_active_engines(&self, count: i64) {
        self.active_engines.set(count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_creation_and_increment() {
        let c = counter("test_counter_inc", "test").unwrap();
        c.inc();
        assert_eq!(c.get(), 1);
    }

    #[test]
    fn test_counter_vec_creation() {
        let cv = counter_vec("test_cv_labels", "test", &["label"]).unwrap();
        cv.with_label_values(&["a"]).inc_by(3);
        assert_eq!(cv.with_label_values(&["a"]).get(), 3);
    }

    #[test]
    fn test_histogram_creation_and_observe() {
        let h = histogram("test_hist_observe", "test", vec![1.0, 2.0, 5.0]).unwrap();
        h.observe(1.5);
        h.observe(3.0);
        assert_eq!(h.get_sample_count(), 2);
    }

    #[test]
    fn test_analysis_metrics_record_scan() {
        let m = AnalysisMetrics::new().unwrap();
        m.record_scan(150.0, 10);
        assert_eq!(m.scans_total.get(), 1);
        assert_eq!(m.files_scanned.get(), 10);
    }

    #[test]
    fn test_analysis_metrics_record_finding() {
        let m = AnalysisMetrics::new().unwrap();
        m.record_finding("error");
        m.record_finding("error");
        m.record_finding("warning");
        assert_eq!(m.findings_total.with_label_values(&["error"]).get(), 2);
        assert_eq!(m.findings_total.with_label_values(&["warning"]).get(), 1);
    }

    #[test]
    fn test_analysis_metrics_active_engines() {
        let m = AnalysisMetrics::new().unwrap();
        m.set_active_engines(3);
        assert_eq!(m.active_engines.get(), 3);
        m.set_active_engines(2);
        assert_eq!(m.active_engines.get(), 2);
    }

    #[test]
    fn test_global_singleton_is_same_instance() {
        let a = AnalysisMetrics::global() as *const AnalysisMetrics;
        let b = AnalysisMetrics::global() as *const AnalysisMetrics;
        assert_eq!(a, b);
    }
}
