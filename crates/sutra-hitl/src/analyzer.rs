use sutra_common::error::SutraResult;
use sutra_schema::v1::{Engine, Finding, Severity};

use crate::store::FeedbackStore;
use crate::types::{FeedbackMetrics, FeedbackVerdict, HitlConfig};

pub struct FeedbackAnalyzer<'a> {
    store: &'a dyn FeedbackStore,
}

impl<'a> FeedbackAnalyzer<'a> {
    pub fn new(store: &'a dyn FeedbackStore) -> Self {
        Self { store }
    }

    pub fn metrics(&self) -> SutraResult<FeedbackMetrics> {
        self.store.metrics()
    }

    pub fn engine_reliability(&self) -> SutraResult<Vec<(Engine, f64)>> {
        let metrics = self.store.metrics()?;
        let mut result: Vec<(Engine, f64)> = Vec::new();

        for (name, precision) in &metrics.precision_by_engine {
            if let Some(engine) = Engine::from_name(name) {
                result.push((engine, *precision));
            }
        }

        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(result)
    }

    pub fn adjust_finding(
        &self,
        finding: &Finding,
        config: &HitlConfig,
    ) -> SutraResult<Finding> {
        if !config.adjust_confidence {
            return Ok(finding.clone());
        }

        let entries = self.store.get_by_finding_id(&finding.id)?;
        if entries.len() < config.min_feedback_count as usize {
            return Ok(finding.clone());
        }

        let mut adjusted = finding.clone();
        let correct = entries.iter().filter(|e| e.verdict == FeedbackVerdict::Correct).count();
        let incorrect = entries.iter().filter(|e| e.verdict == FeedbackVerdict::Incorrect).count();
        let total_valid = correct + incorrect;

        if total_valid == 0 {
            return Ok(finding.clone());
        }

        let confirm_ratio = correct as f64 / total_valid as f64;
        let reject_ratio = incorrect as f64 / total_valid as f64;

        if confirm_ratio >= config.auto_confirm_threshold {
            adjusted.validated = true;
        }

        if reject_ratio >= config.auto_reject_threshold {
            adjusted.severity = match adjusted.severity {
                Severity::Critical => Severity::Warning,
                Severity::Error => Severity::Info,
                _ => Severity::Info,
            };
        }

        Ok(adjusted)
    }

    pub fn adjust_findings(
        &self,
        findings: &[Finding],
        config: &HitlConfig,
    ) -> SutraResult<Vec<Finding>> {
        findings.iter().map(|f| self.adjust_finding(f, config)).collect()
    }

    pub fn findings_with_feedback(&self, findings: &[Finding]) -> SutraResult<Vec<(Finding, Vec<crate::types::FeedbackEntry>)>> {
        let mut result = Vec::new();
        for finding in findings {
            let entries = self.store.get_by_finding_id(&finding.id)?;
            if !entries.is_empty() {
                result.push((finding.clone(), entries));
            }
        }
        Ok(result)
    }

    pub fn precision_at_k(&self, engine: &Engine, k: usize) -> SutraResult<f64> {
        let entries = self.store.get_by_engine(engine)?;
        let mut valid: Vec<&crate::types::FeedbackEntry> = entries
            .iter()
            .filter(|e| e.verdict != FeedbackVerdict::Uncertain)
            .collect();
        valid.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let count = valid.len().min(k);
        if count == 0 {
            return Ok(0.0);
        }

        let correct = valid.iter().take(k).filter(|e| e.verdict == FeedbackVerdict::Correct).count();
        Ok(correct as f64 / count as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryFeedbackStore;
    use crate::types::FeedbackEntry;
    use sutra_schema::v1::{Engine, Finding, Severity};

    fn make_store(entries: Vec<FeedbackEntry>) -> InMemoryFeedbackStore {
        let mut store = InMemoryFeedbackStore::new();
        for e in entries {
            store.store(e).unwrap();
        }
        store
    }

    #[test]
    fn test_engine_reliability_ordered() {
        let mut store = InMemoryFeedbackStore::new();
        store.store(FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t")).unwrap();
        store.store(FeedbackEntry::new("e2", "f2", Engine::Mgtg, "f.rs", 2, FeedbackVerdict::Incorrect, "t")).unwrap();
        store.store(FeedbackEntry::new("e3", "f3", Engine::Process, "f.rs", 3, FeedbackVerdict::Correct, "t")).unwrap();

        let analyzer = FeedbackAnalyzer::new(&store);
        let reliability = analyzer.engine_reliability().unwrap();
        assert_eq!(reliability.len(), 2);
        assert_eq!(reliability[0].0, Engine::Process);
        assert!((reliability[0].1 - 1.0).abs() < 1e-9);
        assert_eq!(reliability[1].0, Engine::Mgtg);
        assert!((reliability[1].1 - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_adjust_finding_below_threshold() {
        let store = make_store(vec![]);
        let analyzer = FeedbackAnalyzer::new(&store);
        let finding = Finding::new("f1", Engine::Mgtg, "f.rs", 1, "test", Severity::Error);
        let config = HitlConfig::default();

        let adjusted = analyzer.adjust_finding(&finding, &config).unwrap();
        assert!(!adjusted.validated);
        assert_eq!(adjusted.severity, Severity::Error);
    }

    #[test]
    fn test_adjust_finding_auto_confirm() {
        let entries = vec![
            FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t"),
            FeedbackEntry::new("e2", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t"),
            FeedbackEntry::new("e3", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t"),
        ];
        let store = make_store(entries);
        let analyzer = FeedbackAnalyzer::new(&store);
        let finding = Finding::new("f1", Engine::Mgtg, "f.rs", 1, "test", Severity::Error);
        let config = HitlConfig::default();

        let adjusted = analyzer.adjust_finding(&finding, &config).unwrap();
        assert!(adjusted.validated);
        assert_eq!(adjusted.severity, Severity::Error);
    }

    #[test]
    fn test_adjust_finding_auto_reject() {
        let entries = vec![
            FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Incorrect, "t"),
            FeedbackEntry::new("e2", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Incorrect, "t"),
            FeedbackEntry::new("e3", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Incorrect, "t"),
        ];
        let store = make_store(entries);
        let analyzer = FeedbackAnalyzer::new(&store);
        let finding = Finding::new("f1", Engine::Mgtg, "f.rs", 1, "test", Severity::Error);
        let config = HitlConfig::default();

        let adjusted = analyzer.adjust_finding(&finding, &config).unwrap();
        assert_eq!(adjusted.severity, Severity::Info);
    }

    #[test]
    fn test_adjust_finding_critical_downgraded_to_warning() {
        let entries = vec![
            FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Incorrect, "t"),
            FeedbackEntry::new("e2", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Incorrect, "t"),
            FeedbackEntry::new("e3", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Incorrect, "t"),
        ];
        let store = make_store(entries);
        let analyzer = FeedbackAnalyzer::new(&store);
        let finding = Finding::new("f1", Engine::Mgtg, "f.rs", 1, "test", Severity::Critical);
        let config = HitlConfig::default();

        let adjusted = analyzer.adjust_finding(&finding, &config).unwrap();
        assert_eq!(adjusted.severity, Severity::Warning);
    }

    #[test]
    fn test_adjust_findings_batch() {
        let mut store = InMemoryFeedbackStore::new();
        store.store(FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t")).unwrap();
        store.store(FeedbackEntry::new("e2", "f2", Engine::Process, "f.rs", 1, FeedbackVerdict::Incorrect, "t")).unwrap();
        store.store(FeedbackEntry::new("e3", "f2", Engine::Process, "f.rs", 1, FeedbackVerdict::Incorrect, "t")).unwrap();
        store.store(FeedbackEntry::new("e4", "f2", Engine::Process, "f.rs", 1, FeedbackVerdict::Incorrect, "t")).unwrap();

        let analyzer = FeedbackAnalyzer::new(&store);
        let findings = vec![
            Finding::new("f1", Engine::Mgtg, "f.rs", 1, "t1", Severity::Warning),
            Finding::new("f2", Engine::Process, "f.rs", 1, "t2", Severity::Error),
        ];
        let config = HitlConfig::default();

        let adjusted = analyzer.adjust_findings(&findings, &config).unwrap();
        assert_eq!(adjusted.len(), 2);
        assert!(!adjusted[0].validated); // only 1 entry, below threshold
        assert_eq!(adjusted[0].severity, Severity::Warning);
        assert_eq!(adjusted[1].severity, Severity::Info); // rejected
    }

    #[test]
    fn test_findings_with_feedback() {
        let mut store = InMemoryFeedbackStore::new();
        store.store(FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t")).unwrap();

        let analyzer = FeedbackAnalyzer::new(&store);
        let findings = vec![
            Finding::new("f1", Engine::Mgtg, "f.rs", 1, "t1", Severity::Warning),
            Finding::new("f2", Engine::Process, "f.rs", 1, "t2", Severity::Error),
        ];

        let result = analyzer.findings_with_feedback(&findings).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.id, "f1");
        assert_eq!(result[0].1.len(), 1);
    }

    #[test]
    fn test_precision_at_k() {
        let mut store = InMemoryFeedbackStore::new();
        store.store(FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t")).unwrap();
        store.store(FeedbackEntry::new("e2", "f2", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t")).unwrap();
        store.store(FeedbackEntry::new("e3", "f3", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Incorrect, "t")).unwrap();
        store.store(FeedbackEntry::new("e4", "f4", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Uncertain, "t")).unwrap();

        let analyzer = FeedbackAnalyzer::new(&store);
        let prec = analyzer.precision_at_k(&Engine::Mgtg, 2).unwrap();
        assert!((prec - 1.0).abs() < 1e-9);

        let prec_all = analyzer.precision_at_k(&Engine::Mgtg, 10).unwrap();
        assert!((prec_all - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_adjust_finding_disabled_config() {
        let mut store = InMemoryFeedbackStore::new();
        store.store(FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Incorrect, "t")).unwrap();
        store.store(FeedbackEntry::new("e2", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Incorrect, "t")).unwrap();
        store.store(FeedbackEntry::new("e3", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Incorrect, "t")).unwrap();

        let analyzer = FeedbackAnalyzer::new(&store);
        let finding = Finding::new("f1", Engine::Mgtg, "f.rs", 1, "test", Severity::Error);
        let config = HitlConfig {
            adjust_confidence: false,
            ..HitlConfig::default()
        };

        let adjusted = analyzer.adjust_finding(&finding, &config).unwrap();
        assert!(!adjusted.validated);
        assert_eq!(adjusted.severity, Severity::Error);
    }

    #[test]
    fn test_metrics_access() {
        let mut store = InMemoryFeedbackStore::new();
        store.store(FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t")).unwrap();

        let analyzer = FeedbackAnalyzer::new(&store);
        let m = analyzer.metrics().unwrap();
        assert_eq!(m.total_entries, 1);
        assert_eq!(m.correct_count, 1);
    }

    // ── Edge case tests ───────────────────────────────────────────────

    #[test]
    fn test_adjust_finding_exactly_minus_one() {
        let entries = vec![
            FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t"),
            FeedbackEntry::new("e2", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t"),
        ];
        let store = make_store(entries);
        let analyzer = FeedbackAnalyzer::new(&store);
        // default min_feedback_count is 3, so 2 entries should NOT adjust
        let finding = Finding::new("f1", Engine::Mgtg, "f.rs", 1, "test", Severity::Error);
        let config = HitlConfig::default();
        let adjusted = analyzer.adjust_finding(&finding, &config).unwrap();
        assert!(!adjusted.validated);
        assert_eq!(adjusted.severity, Severity::Error);
    }

    #[test]
    fn test_adjust_finding_exactly_min_feedback_count() {
        let entries = vec![
            FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t"),
            FeedbackEntry::new("e2", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t"),
            FeedbackEntry::new("e3", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t"),
        ];
        let store = make_store(entries);
        let analyzer = FeedbackAnalyzer::new(&store);
        // default min_feedback_count is 3, so 3 entries should adjust
        let finding = Finding::new("f1", Engine::Mgtg, "f.rs", 1, "test", Severity::Error);
        let config = HitlConfig::default();
        let adjusted = analyzer.adjust_finding(&finding, &config).unwrap();
        assert!(adjusted.validated);
        assert_eq!(adjusted.severity, Severity::Error);
    }

    #[test]
    fn test_adjust_finding_equal_correct_incorrect() {
        let entries = vec![
            FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t"),
            FeedbackEntry::new("e2", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t"),
            FeedbackEntry::new("e3", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Incorrect, "t"),
            FeedbackEntry::new("e4", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Incorrect, "t"),
        ];
        let store = make_store(entries);
        let analyzer = FeedbackAnalyzer::new(&store);
        let finding = Finding::new("f1", Engine::Mgtg, "f.rs", 1, "test", Severity::Error);
        let config = HitlConfig::default();
        let adjusted = analyzer.adjust_finding(&finding, &config).unwrap();
        // confirm_ratio = 0.5 < 0.8, reject_ratio = 0.5 < 0.6 — neither triggers
        assert!(!adjusted.validated);
        assert_eq!(adjusted.severity, Severity::Error);
    }

    #[test]
    fn test_adjust_finding_all_uncertain() {
        let entries = vec![
            FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Uncertain, "t"),
            FeedbackEntry::new("e2", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Uncertain, "t"),
            FeedbackEntry::new("e3", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Uncertain, "t"),
        ];
        let store = make_store(entries);
        let analyzer = FeedbackAnalyzer::new(&store);
        let finding = Finding::new("f1", Engine::Mgtg, "f.rs", 1, "test", Severity::Error);
        let config = HitlConfig::default();
        let adjusted = analyzer.adjust_finding(&finding, &config).unwrap();
        // total_valid = 0 → returns unchanged
        assert!(!adjusted.validated);
        assert_eq!(adjusted.severity, Severity::Error);
    }

    #[test]
    fn test_precision_at_k_zero() {
        let mut store = InMemoryFeedbackStore::new();
        store.store(FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "t")).unwrap();
        let analyzer = FeedbackAnalyzer::new(&store);
        let prec = analyzer.precision_at_k(&Engine::Mgtg, 0).unwrap();
        assert!((prec - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_engine_reliability_no_feedback() {
        let store = InMemoryFeedbackStore::new();
        let analyzer = FeedbackAnalyzer::new(&store);
        let reliability = analyzer.engine_reliability().unwrap();
        assert!(reliability.is_empty());
    }
}
