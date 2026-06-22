use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use sutra_schema::v1::{Engine, Severity};

/// Human verdict on a finding.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FeedbackVerdict {
    Correct,
    Incorrect,
    Uncertain,
}

impl FeedbackVerdict {
    pub fn as_weight(&self) -> f64 {
        match self {
            FeedbackVerdict::Correct => 1.0,
            FeedbackVerdict::Incorrect => -1.0,
            FeedbackVerdict::Uncertain => 0.0,
        }
    }
}

/// A single piece of human feedback on a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntry {
    pub id: String,
    pub finding_id: String,
    pub engine: Engine,
    pub file_path: String,
    pub line: u32,
    pub verdict: FeedbackVerdict,
    pub actual_severity: Option<Severity>,
    pub notes: String,
    pub timestamp: i64,
    pub reviewer: String,
}

impl FeedbackEntry {
    pub fn new(
        id: &str,
        finding_id: &str,
        engine: Engine,
        file_path: &str,
        line: u32,
        verdict: FeedbackVerdict,
        reviewer: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            finding_id: finding_id.to_string(),
            engine,
            file_path: file_path.to_string(),
            line,
            verdict,
            actual_severity: None,
            notes: String::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            reviewer: reviewer.to_string(),
        }
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.actual_severity = Some(severity);
        self
    }

    pub fn with_notes(mut self, notes: &str) -> Self {
        self.notes = notes.to_string();
        self
    }
}

/// Configuration for the HITL engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitlConfig {
    pub min_feedback_count: u32,
    pub adjust_confidence: bool,
    pub auto_confirm_threshold: f64,
    pub auto_reject_threshold: f64,
    pub enabled: bool,
}

impl Default for HitlConfig {
    fn default() -> Self {
        Self {
            min_feedback_count: 3,
            adjust_confidence: true,
            auto_confirm_threshold: 0.8,
            auto_reject_threshold: 0.6,
            enabled: true,
        }
    }
}

/// Computed metrics from a feedback store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeedbackMetrics {
    pub total_entries: usize,
    pub correct_count: usize,
    pub incorrect_count: usize,
    pub uncertain_count: usize,
    pub precision_by_engine: HashMap<String, f64>,
    pub total_findings_with_feedback: usize,
    pub coverage: f64,
}

impl FeedbackMetrics {
    pub fn new() -> Self {
        Self {
            total_entries: 0,
            correct_count: 0,
            incorrect_count: 0,
            uncertain_count: 0,
            precision_by_engine: HashMap::new(),
            total_findings_with_feedback: 0,
            coverage: 0.0,
        }
    }

    pub fn precision(&self) -> f64 {
        let total_valid = self.correct_count + self.incorrect_count;
        if total_valid == 0 {
            return 0.0;
        }
        self.correct_count as f64 / total_valid as f64
    }

    pub fn incorrect_rate(&self) -> f64 {
        let total_valid = self.correct_count + self.incorrect_count;
        if total_valid == 0 {
            return 0.0;
        }
        self.incorrect_count as f64 / total_valid as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_verdict_weight() {
        assert_eq!(FeedbackVerdict::Correct.as_weight(), 1.0);
        assert_eq!(FeedbackVerdict::Incorrect.as_weight(), -1.0);
        assert_eq!(FeedbackVerdict::Uncertain.as_weight(), 0.0);
    }

    #[test]
    fn test_feedback_entry_new() {
        let e = FeedbackEntry::new(
            "f1", "find-001", Engine::Mgtg, "src/main.rs", 42,
            FeedbackVerdict::Correct, "alice",
        );
        assert_eq!(e.id, "f1");
        assert_eq!(e.finding_id, "find-001");
        assert_eq!(e.engine, Engine::Mgtg);
        assert_eq!(e.file_path, "src/main.rs");
        assert_eq!(e.line, 42);
        assert_eq!(e.verdict, FeedbackVerdict::Correct);
        assert_eq!(e.reviewer, "alice");
        assert_eq!(e.actual_severity, None);
        assert!(e.notes.is_empty());
        assert!(e.timestamp > 0);
    }

    #[test]
    fn test_feedback_entry_with_severity() {
        let e = FeedbackEntry::new("f1", "f2", Engine::Dependency, "a.rs", 1, FeedbackVerdict::Correct, "bob")
            .with_severity(Severity::Critical);
        assert_eq!(e.actual_severity, Some(Severity::Critical));
    }

    #[test]
    fn test_feedback_entry_with_notes() {
        let e = FeedbackEntry::new("f1", "f2", Engine::Process, "a.rs", 1, FeedbackVerdict::Incorrect, "bob")
            .with_notes("false positive, this is intentional");
        assert_eq!(e.notes, "false positive, this is intentional");
    }

    #[test]
    fn test_feedback_entry_serde_roundtrip() {
        let e = FeedbackEntry::new("f1", "find-001", Engine::Mgtg, "src/main.rs", 10, FeedbackVerdict::Correct, "alice")
            .with_severity(Severity::Warning);
        let json = serde_json::to_string(&e).unwrap();
        let de: FeedbackEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(de.id, e.id);
        assert_eq!(de.finding_id, e.finding_id);
        assert_eq!(de.engine, e.engine);
        assert_eq!(de.verdict, e.verdict);
        assert_eq!(de.actual_severity, e.actual_severity);
    }

    #[test]
    fn test_hitl_config_default() {
        let c = HitlConfig::default();
        assert_eq!(c.min_feedback_count, 3);
        assert!(c.adjust_confidence);
        assert_eq!(c.auto_confirm_threshold, 0.8);
        assert_eq!(c.auto_reject_threshold, 0.6);
        assert!(c.enabled);
    }

    #[test]
    fn test_feedback_metrics_empty() {
        let m = FeedbackMetrics::new();
        assert_eq!(m.total_entries, 0);
        assert_eq!(m.precision(), 0.0);
        assert_eq!(m.incorrect_rate(), 0.0);
    }

    #[test]
    fn test_feedback_metrics_precision() {
        let mut m = FeedbackMetrics::new();
        m.correct_count = 8;
        m.incorrect_count = 2;
        m.total_entries = 12;
        assert!((m.precision() - 0.8).abs() < 1e-9);
        assert!((m.incorrect_rate() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn test_feedback_metrics_precision_all_correct() {
        let mut m = FeedbackMetrics::new();
        m.correct_count = 10;
        m.incorrect_count = 0;
        assert!((m.precision() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_feedback_metrics_precision_all_incorrect() {
        let mut m = FeedbackMetrics::new();
        m.correct_count = 0;
        m.incorrect_count = 10;
        assert!((m.precision() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_feedback_metrics_uncertain_not_counted() {
        let mut m = FeedbackMetrics::new();
        m.correct_count = 5;
        m.incorrect_count = 5;
        m.uncertain_count = 10;
        assert_eq!(m.total_entries, 0); // not incremented by test setup
        assert!((m.precision() - 0.5).abs() < 1e-9);
        assert!((m.incorrect_rate() - 0.5).abs() < 1e-9);
    }
}
