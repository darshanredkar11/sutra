use sutra_common::engine::AnalysisEngine;
use sutra_common::error::SutraResult;
use sutra_schema::v1::{AnalysisResult, AnalyzeRequest, Engine, Finding, Recommendation, Severity};

use crate::analyzer::FeedbackAnalyzer;
use crate::store::{FeedbackStore, InMemoryFeedbackStore};
use crate::types::{FeedbackEntry, HitlConfig};

pub struct HitlEngine {
    store: Box<dyn FeedbackStore>,
    config: HitlConfig,
    /// Path to auto-persist feedback to, e.g. `~/.sutra/hitl-feedback.json`.
    /// Only set by `new()` -- `with_store()` (used by tests and anyone
    /// wanting a pure in-memory engine) never touches disk.
    persist_path: Option<String>,
}

impl HitlEngine {
    pub fn new() -> Self {
        let mut store = InMemoryFeedbackStore::new();
        let persist_path = std::env::var("HOME")
            .ok()
            .map(|home| format!("{}/.sutra/hitl-feedback.json", home));

        // ponytail: auto-load feedback from ~/.sutra/hitl-feedback.json if it
        // exists. Loaded via load_bulk (no per-entry duplicate-scan +
        // full-file rewrite) -- replaying N saved entries through store()
        // one at a time was O(n^2) and turned a few thousand entries into a
        // multi-minute stall on every single engine construction.
        if let Some(path) = &persist_path {
            if let Ok(json) = std::fs::read_to_string(path) {
                if let Ok(entries) = serde_json::from_str::<Vec<FeedbackEntry>>(&json) {
                    store.load_bulk(entries);
                }
            }
        }
        Self {
            store: Box::new(store),
            config: HitlConfig::default(),
            persist_path,
        }
    }

    pub fn with_store(store: Box<dyn FeedbackStore>) -> Self {
        Self {
            store,
            config: HitlConfig::default(),
            persist_path: None,
        }
    }

    pub fn with_config(mut self, config: HitlConfig) -> Self {
        self.config = config;
        self
    }

    fn persist(&self) -> SutraResult<()> {
        let Some(path) = &self.persist_path else {
            return Ok(());
        };
        let entries = self.store.get_all()?;
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, serde_json::to_string_pretty(&entries).unwrap_or_default());
        Ok(())
    }

    pub fn store_feedback(&mut self, entry: FeedbackEntry) -> SutraResult<()> {
        self.store.store(entry)?;
        self.persist()
    }

    pub fn store_feedback_batch(&mut self, entries: Vec<FeedbackEntry>) -> SutraResult<()> {
        for entry in entries {
            self.store.store(entry)?;
        }
        self.persist()
    }

    pub fn adjust_findings(&self, findings: &[Finding]) -> SutraResult<Vec<Finding>> {
        let analyzer = FeedbackAnalyzer::new(self.store.as_ref());
        analyzer.adjust_findings(findings, &self.config)
    }

    pub fn engine_reliability(&self) -> SutraResult<Vec<(Engine, f64)>> {
        let analyzer = FeedbackAnalyzer::new(self.store.as_ref());
        analyzer.engine_reliability()
    }

    pub fn metrics(&self) -> SutraResult<crate::types::FeedbackMetrics> {
        self.store.metrics()
    }

    pub fn config(&self) -> &HitlConfig {
        &self.config
    }

    fn analyzer(&self) -> FeedbackAnalyzer<'_> {
        FeedbackAnalyzer::new(self.store.as_ref())
    }
}

impl Default for HitlEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisEngine for HitlEngine {
    fn name(&self) -> &'static str {
        "hitl"
    }

    fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
        if !self.config.enabled {
            return Ok(AnalysisResult::new(&request.request_id, &request.commit_hash));
        }

        let metrics = self.store.metrics()?;
        let analyzer = self.analyzer();

        let mut findings = Vec::new();

        if metrics.total_entries > 0 {
            let precision = metrics.precision();

            findings.push(Finding::new(
                "HITL-001",
                Engine::Hitl,
                "feedback",
                0,
                &format!(
                    "Feedback system precision: {:.1}% based on {} entries ({} findings reviewed)",
                    precision * 100.0,
                    metrics.total_entries,
                    metrics.total_findings_with_feedback,
                ),
                if precision < 0.5 {
                    Severity::Warning
                } else {
                    Severity::Info
                },
            ));

            if metrics.total_findings_with_feedback > 10 {
                let coverage = metrics.total_entries as f64
                    / metrics.total_findings_with_feedback.max(1) as f64;
                if coverage < 5.0 {
                    findings.push(Finding::new(
                        "HITL-002",
                        Engine::Hitl,
                        "feedback",
                        0,
                        &format!(
                            "Low feedback density: {:.1} entries per finding",
                            coverage,
                        ),
                        Severity::Info,
                    ));
                }
            }

            for (engine_name, engine_precision) in &metrics.precision_by_engine {
                if *engine_precision < 0.5 {
                    findings.push(Finding::new(
                        "HITL-003",
                        Engine::Hitl,
                        "feedback",
                        0,
                        &format!(
                            "Low precision for engine '{}': {:.1}% — review findings manually",
                            engine_name,
                            engine_precision * 100.0,
                        ),
                        Severity::Warning,
                    ));
                }
            }
        }

        let mut recommendations = Vec::new();
        if metrics.total_entries == 0 {
            recommendations.push(Recommendation::new(
                "No human feedback recorded. Start reviewing findings to improve system precision.",
                0.9,
            ));
        } else if metrics.total_findings_with_feedback < 10 {
            recommendations.push(Recommendation::new(
                &format!(
                    "Only {} findings have feedback. Review more to improve coverage.",
                    metrics.total_findings_with_feedback,
                ),
                0.7,
            ));
        }

        if let Ok(reliability) = analyzer.engine_reliability() {
            if let Some((best_engine, best_precision)) = reliability.first() {
                recommendations.push(Recommendation::new(
                    &format!(
                        "Most reliable engine: {} ({:.0}% precision). Prioritize its findings.",
                        best_engine,
                        best_precision * 100.0,
                    ),
                    0.6,
                ));
            }
        }

        let result = AnalysisResult {
            request_id: request.request_id.clone(),
            commit_hash: request.commit_hash.clone(),
            overall_risk: 0.0,
            findings,
            recommendations,
            metrics: None,
            processing_time_ms: 0.0,
            blocked_merge: false,
            jit_features: None,
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FeedbackEntry, FeedbackVerdict};
    use sutra_schema::v1::{Finding, Severity};

    /// Fresh, disk-free engine for tests. `hermetic_engine()` auto-loads
    /// and auto-persists to the real `~/.sutra/hitl-feedback.json` on this
    /// machine, which made every test here order-dependent on and mutate
    /// shared real state (duplicate-id collisions across runs, unbounded
    /// file growth). Tests must use this instead.
    fn hermetic_engine() -> HitlEngine {
        HitlEngine::with_store(Box::new(InMemoryFeedbackStore::new()))
    }

    #[test]
    fn test_engine_disabled() {
        let engine = hermetic_engine().with_config(HitlConfig {
            enabled: false,
            ..HitlConfig::default()
        });
        let req = AnalyzeRequest::new("/repo", "abc");
        let result = engine.analyze(&req).unwrap();
        assert!(result.findings.is_empty());
        assert!(result.recommendations.is_empty());
    }

    #[test]
    fn test_engine_no_feedback() {
        let engine = hermetic_engine();
        let req = AnalyzeRequest::new("/repo", "abc");
        let result = engine.analyze(&req).unwrap();

        assert!(result.findings.is_empty());
        assert_eq!(result.recommendations.len(), 1);
        assert!(result.recommendations[0].text.contains("No human feedback recorded"));
    }

    #[test]
    fn test_engine_with_feedback() {
        let mut engine = hermetic_engine();
        engine.store_feedback(FeedbackEntry::new(
            "e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "tester",
        )).unwrap();
        engine.store_feedback(FeedbackEntry::new(
            "e2", "f2", Engine::Mgtg, "f.rs", 2, FeedbackVerdict::Incorrect, "tester",
        )).unwrap();
        engine.store_feedback(FeedbackEntry::new(
            "e3", "f3", Engine::Process, "f.rs", 3, FeedbackVerdict::Correct, "tester",
        )).unwrap();

        let req = AnalyzeRequest::new("/repo", "abc");
        let result = engine.analyze(&req).unwrap();

        assert!(!result.findings.is_empty());
        assert!(!result.recommendations.is_empty());
        // HITL-001 should report precision
        assert!(result.findings.iter().any(|f| f.id == "HITL-001"));
    }

    #[test]
    fn test_engine_name() {
        let engine = hermetic_engine();
        assert_eq!(engine.name(), "hitl");
    }

    #[test]
    fn test_adjust_findings() {
        let mut engine = hermetic_engine();
        for i in 0..3 {
            engine.store_feedback(FeedbackEntry::new(
                &format!("e{}", i), "f1", Engine::Mgtg, "f.rs", 1,
                FeedbackVerdict::Correct, "tester",
            )).unwrap();
        }

        let findings = vec![
            Finding::new("f1", Engine::Mgtg, "f.rs", 1, "test", Severity::Error),
        ];

        let adjusted = engine.adjust_findings(&findings).unwrap();
        assert_eq!(adjusted.len(), 1);
        assert!(adjusted[0].validated);
    }

    #[test]
    fn test_store_feedback_batch() {
        let mut engine = hermetic_engine();
        let entries = vec![
            FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "tester"),
            FeedbackEntry::new("e2", "f2", Engine::Mgtg, "f.rs", 2, FeedbackVerdict::Incorrect, "tester"),
            FeedbackEntry::new("e3", "f3", Engine::Process, "f.rs", 3, FeedbackVerdict::Uncertain, "tester"),
        ];
        engine.store_feedback_batch(entries).unwrap();

        let m = engine.metrics().unwrap();
        assert_eq!(m.total_entries, 3);
    }

    #[test]
    fn test_engine_reliability() {
        let mut engine = hermetic_engine();
        engine.store_feedback(FeedbackEntry::new(
            "e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "tester",
        )).unwrap();
        engine.store_feedback(FeedbackEntry::new(
            "e2", "f2", Engine::Process, "f.rs", 2, FeedbackVerdict::Incorrect, "tester",
        )).unwrap();

        let reliability = engine.engine_reliability().unwrap();
        assert_eq!(reliability.len(), 2);
    }

    #[test]
    fn test_engine_with_store() {
        let store = Box::new(InMemoryFeedbackStore::new());
        let engine = HitlEngine::with_store(store);
        assert_eq!(engine.name(), "hitl");
    }

    #[test]
    fn test_engine_config() {
        let config = HitlConfig {
            min_feedback_count: 5,
            ..HitlConfig::default()
        };
        let engine = hermetic_engine().with_config(config.clone());
        assert_eq!(engine.config().min_feedback_count, 5);
    }

    #[test]
    fn test_engine_default() {
        let engine: HitlEngine = Default::default();
        assert!(engine.config().enabled);
    }

    #[test]
    fn test_engine_analyze_with_low_precision_warning() {
        let mut engine = hermetic_engine();
        for i in 0..5 {
            engine.store_feedback(FeedbackEntry::new(
                &format!("e{}", i), &format!("f{}", i), Engine::Mgtg, "f.rs", 1,
                FeedbackVerdict::Incorrect, "tester",
            )).unwrap();
        }
        engine.store_feedback(FeedbackEntry::new(
            "e5", "f5", Engine::Mgtg, "f.rs", 1,
            FeedbackVerdict::Correct, "tester",
        )).unwrap();

        let req = AnalyzeRequest::new("/repo", "abc");
        let result = engine.analyze(&req).unwrap();

        let has_low_precision = result.findings.iter().any(|f| f.id == "HITL-003");
        assert!(has_low_precision);
    }

    // ── Edge case tests ───────────────────────────────────────────────

    #[test]
    fn test_store_feedback_duplicate_id() {
        let mut engine = hermetic_engine();
        engine.store_feedback(FeedbackEntry::new(
            "e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "tester",
        )).unwrap();
        let err = engine.store_feedback(FeedbackEntry::new(
            "e1", "f2", Engine::Process, "f.rs", 2, FeedbackVerdict::Incorrect, "tester",
        )).unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn test_store_feedback_batch_mixed_valid_invalid() {
        let mut engine = hermetic_engine();
        let entries = vec![
            FeedbackEntry::new("e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "tester"),
            FeedbackEntry::new("", "f2", Engine::Process, "f.rs", 2, FeedbackVerdict::Incorrect, "tester"),
            FeedbackEntry::new("e3", "f3", Engine::Mgtg, "f.rs", 3, FeedbackVerdict::Uncertain, "tester"),
        ];
        let err = engine.store_feedback_batch(entries).unwrap_err();
        assert!(err.to_string().contains("cannot be empty"));
        // The first entry was stored before the error was encountered
        assert_eq!(engine.metrics().unwrap().total_entries, 1);
    }

    #[test]
    fn test_engine_analyze_with_1000_feedback_entries() {
        let mut engine = hermetic_engine();
        for i in 0..1000 {
            engine.store_feedback(FeedbackEntry::new(
                &format!("e{}", i), &format!("f{}", i % 50), Engine::Mgtg, "f.rs", 1,
                if i % 3 == 0 { FeedbackVerdict::Correct } else { FeedbackVerdict::Incorrect },
                "tester",
            )).unwrap();
        }
        let req = AnalyzeRequest::new("/repo", "abc");
        let result = engine.analyze(&req).unwrap();
        assert!(!result.findings.is_empty());
        assert!(result.findings.iter().any(|f| f.id == "HITL-001"));
    }

    #[test]
    fn test_engine_disabled_still_stores() {
        let mut engine = hermetic_engine().with_config(HitlConfig {
            enabled: false,
            ..HitlConfig::default()
        });
        engine.store_feedback(FeedbackEntry::new(
            "e1", "f1", Engine::Mgtg, "f.rs", 1, FeedbackVerdict::Correct, "tester",
        )).unwrap();
        engine.store_feedback(FeedbackEntry::new(
            "e2", "f2", Engine::Mgtg, "f.rs", 2, FeedbackVerdict::Incorrect, "tester",
        )).unwrap();

        // Store should have entries
        assert_eq!(engine.metrics().unwrap().total_entries, 2);

        // Analyze should return empty since engine is disabled
        let req = AnalyzeRequest::new("/repo", "abc");
        let result = engine.analyze(&req).unwrap();
        assert!(result.findings.is_empty());
        assert!(result.recommendations.is_empty());
    }
}
