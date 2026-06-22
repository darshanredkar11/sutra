use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Engines ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    Mgtg,
    Process,
    Dependency,
    Ml,
    Hitl,
    RuntimeSurvivability,
}

impl Engine {
    pub const ALL: [Engine; 6] = [Engine::Mgtg, Engine::Process, Engine::Dependency, Engine::Ml, Engine::Hitl, Engine::RuntimeSurvivability];

    pub fn as_str(&self) -> &'static str {
        match self {
            Engine::Mgtg => "mgtg",
            Engine::Process => "process",
            Engine::Dependency => "dependency",
            Engine::Ml => "ml",
            Engine::Hitl => "hitl",
            Engine::RuntimeSurvivability => "rse",
        }
    }

    pub fn from_name(name: &str) -> Option<Engine> {
        match name {
            "mgtg" => Some(Engine::Mgtg),
            "process" => Some(Engine::Process),
            "dependency" => Some(Engine::Dependency),
            "ml" => Some(Engine::Ml),
            "hitl" => Some(Engine::Hitl),
            "rse" | "runtime" => Some(Engine::RuntimeSurvivability),
            _ => None,
        }
    }
}

impl std::fmt::Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Analysis Config ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisConfig {
    #[serde(default)]
    pub skip_llm_validation: bool,
    #[serde(default = "default_min_severity")]
    pub min_severity: f64,
    #[serde(default)]
    pub include_metrics: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            skip_llm_validation: false,
            min_severity: default_min_severity(),
            include_metrics: false,
        }
    }
}

const fn default_min_severity() -> f64 {
    0.0
}

// ── Analyze Request ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeRequest {
    pub repo_path: String,
    pub commit_hash: String,
    #[serde(default)]
    pub engines: Vec<Engine>,
    #[serde(default)]
    pub config: AnalysisConfig,
    pub request_id: String,
}

impl AnalyzeRequest {
    pub fn new(repo_path: &str, commit_hash: &str) -> Self {
        Self {
            repo_path: repo_path.to_owned(),
            commit_hash: commit_hash.to_owned(),
            engines: vec![],
            config: AnalysisConfig::default(),
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

// ── Severity ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

impl Severity {
    pub fn rank(&self) -> u8 {
        match self {
            Severity::Info => 0,
            Severity::Warning => 1,
            Severity::Error => 2,
            Severity::Critical => 3,
        }
    }

    pub fn from_rank(r: u8) -> Option<Self> {
        match r {
            0 => Some(Severity::Info),
            1 => Some(Severity::Warning),
            2 => Some(Severity::Error),
            3 => Some(Severity::Critical),
            _ => None,
        }
    }
}

// ── Finding ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub engine: Engine,
    pub file_path: String,
    pub line: u32,
    pub message: String,
    pub severity: Severity,
    #[serde(default = "default_validated")]
    pub validated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<String>,
}

const fn default_validated() -> bool {
    false
}

impl Finding {
    pub fn new(
        id: &str,
        engine: Engine,
        file_path: &str,
        line: u32,
        message: &str,
        severity: Severity,
    ) -> Self {
        Self {
            id: id.to_owned(),
            engine,
            file_path: file_path.to_owned(),
            line,
            message: message.to_owned(),
            severity,
            validated: false,
            suggested_fix: None,
        }
    }

    pub fn with_fix(mut self, fix: &str) -> Self {
        self.suggested_fix = Some(fix.to_owned());
        self
    }

    pub fn with_validated(mut self, v: bool) -> Self {
        self.validated = v;
        self
    }
}

// ── Metrics Summary ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MetricsSummary {
    #[serde(default)]
    pub cyclomatic_max: f64,
    #[serde(default)]
    pub cognitive_max: f64,
    #[serde(default)]
    pub nesting_max: f64,
    #[serde(default)]
    pub total_functions: u32,
    #[serde(default)]
    pub total_files: u32,
    #[serde(default)]
    pub dependency_fan_in_max: f64,
    #[serde(default)]
    pub dependency_fan_out_max: f64,
    #[serde(default)]
    pub circular_dependencies: u32,
    #[serde(default)]
    pub rse_survivability: f64,
    #[serde(default)]
    pub rse_complexity_max: f64,
    #[serde(default)]
    pub rse_memory_per_request: f64,
    #[serde(default)]
    pub rse_safe_rps: f64,
}

impl MetricsSummary {
    pub fn any_non_default(&self) -> bool {
        self.cyclomatic_max != 0.0
            || self.cognitive_max != 0.0
            || self.nesting_max != 0.0
            || self.total_functions != 0
            || self.total_files != 0
            || self.dependency_fan_in_max != 0.0
            || self.dependency_fan_out_max != 0.0
            || self.circular_dependencies != 0
            || self.rse_survivability != 0.0
            || self.rse_complexity_max != 0.0
            || self.rse_memory_per_request != 0.0
            || self.rse_safe_rps != 0.0
    }
}

// ── Recommendation ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    pub text: String,
    #[serde(default)]
    pub affected_files: Vec<String>,
    #[serde(default)]
    pub priority: f64,
}

impl Recommendation {
    pub fn new(text: &str, priority: f64) -> Self {
        Self {
            text: text.to_owned(),
            affected_files: vec![],
            priority,
        }
    }
}

// ── Analysis Result ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub request_id: String,
    pub commit_hash: String,
    #[serde(default)]
    pub overall_risk: f64,
    #[serde(default)]
    pub findings: Vec<Finding>,
    #[serde(default)]
    pub recommendations: Vec<Recommendation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<MetricsSummary>,
    #[serde(default)]
    pub processing_time_ms: f64,
    #[serde(default)]
    pub blocked_merge: bool,
}

impl AnalysisResult {
    pub fn new(request_id: &str, commit_hash: &str) -> Self {
        Self {
            request_id: request_id.to_owned(),
            commit_hash: commit_hash.to_owned(),
            overall_risk: 0.0,
            findings: vec![],
            recommendations: vec![],
            metrics: None,
            processing_time_ms: 0.0,
            blocked_merge: false,
        }
    }

    pub fn finding_count_by_severity(&self, severity: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == severity).count()
    }

    pub fn highest_severity(&self) -> Option<Severity> {
        self.findings.iter().map(|f| f.severity).max_by_key(|s| s.rank())
    }

    pub fn is_failing(&self) -> bool {
        self.blocked_merge || self.overall_risk >= 0.8
    }
}

// ── Feedback ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackOutcome {
    Correct,
    FalseAlarm,
    Partial,
    Unsure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Feedback {
    pub prediction_id: String,
    pub finding_id: String,
    pub outcome: FeedbackOutcome,
    pub comment: String,
    pub user_id: String,
    pub timestamp_ms: u64,
}

impl Feedback {
    pub fn new(prediction_id: &str, finding_id: &str, outcome: FeedbackOutcome, user_id: &str) -> Self {
        Self {
            prediction_id: prediction_id.to_owned(),
            finding_id: finding_id.to_owned(),
            outcome,
            comment: String::new(),
            user_id: user_id.to_owned(),
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    pub fn with_comment(mut self, comment: &str) -> Self {
        self.comment = comment.to_owned();
        self
    }

    pub fn is_positive(&self) -> bool {
        self.outcome == FeedbackOutcome::Correct
    }

    pub fn is_negative(&self) -> bool {
        self.outcome == FeedbackOutcome::FalseAlarm
    }
}

// ── Feature Map (for ML engine inputs) ───────────────────────────────

pub type FeatureMap = HashMap<String, f64>;

// ── Error type for schema validation ─────────────────────────────────

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SchemaError {
    #[error("invalid severity rank: {0}")]
    InvalidSeverityRank(u8),
    #[error("unknown engine: {0}")]
    UnknownEngine(String),
    #[error("risk score out of bounds: {0} (must be 0.0-1.0)")]
    RiskOutOfBounds(f64),
}

pub fn validate_risk(score: f64) -> Result<f64, SchemaError> {
    if !(0.0..=1.0).contains(&score) {
        return Err(SchemaError::RiskOutOfBounds(score));
    }
    Ok(score)
}

// ── Health Status ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default)]
    pub last_heartbeat_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Engine tests ─────────────────────────────────────────────────

    #[test]
    fn test_engine_all_includes_all() {
        assert_eq!(Engine::ALL.len(), 6);
    }

    #[test]
    fn test_engine_as_str_matches_variant() {
        assert_eq!(Engine::Mgtg.as_str(), "mgtg");
        assert_eq!(Engine::Process.as_str(), "process");
        assert_eq!(Engine::Dependency.as_str(), "dependency");
        assert_eq!(Engine::Ml.as_str(), "ml");
        assert_eq!(Engine::Hitl.as_str(), "hitl");
        assert_eq!(Engine::RuntimeSurvivability.as_str(), "rse");
    }

    #[test]
    fn test_engine_from_name() {
        assert_eq!(Engine::from_name("mgtg"), Some(Engine::Mgtg));
        assert_eq!(Engine::from_name("process"), Some(Engine::Process));
        assert_eq!(Engine::from_name("dependency"), Some(Engine::Dependency));
        assert_eq!(Engine::from_name("ml"), Some(Engine::Ml));
        assert_eq!(Engine::from_name("hitl"), Some(Engine::Hitl));
        assert_eq!(Engine::from_name("rse"), Some(Engine::RuntimeSurvivability));
        assert_eq!(Engine::from_name("runtime"), Some(Engine::RuntimeSurvivability));
        assert_eq!(Engine::from_name("unknown"), None);
    }

    #[test]
    fn test_engine_display() {
        assert_eq!(format!("{}", Engine::Mgtg), "mgtg");
        assert_eq!(format!("{}", Engine::Hitl), "hitl");
    }

    #[test]
    fn test_engine_serde_roundtrip_json() {
        for engine in &Engine::ALL {
            let json = serde_json::to_string(engine).unwrap();
            let back: Engine = serde_json::from_str(&json).unwrap();
            assert_eq!(*engine, back);
        }
    }

    #[test]
    fn test_engine_serde_snake_case() {
        let json = r#""mgtg""#;
        let e: Engine = serde_json::from_str(json).unwrap();
        assert_eq!(e, Engine::Mgtg);

        let json = r#""ml""#;
        let e: Engine = serde_json::from_str(json).unwrap();
        assert_eq!(e, Engine::Ml);
    }

    // ── Severity tests ───────────────────────────────────────────────

    #[test]
    fn test_severity_rank_ordering() {
        assert!(Severity::Info.rank() < Severity::Warning.rank());
        assert!(Severity::Warning.rank() < Severity::Error.rank());
        assert!(Severity::Error.rank() < Severity::Critical.rank());
    }

    #[test]
    fn test_severity_from_rank_roundtrip() {
        for s in &[Severity::Info, Severity::Warning, Severity::Error, Severity::Critical] {
            let rank = s.rank();
            let back = Severity::from_rank(rank).unwrap();
            assert_eq!(*s, back);
        }
    }

    #[test]
    fn test_severity_from_rank_invalid() {
        assert!(Severity::from_rank(4).is_none());
        assert!(Severity::from_rank(255).is_none());
    }

    #[test]
    fn test_severity_serde_roundtrip() {
        for s in &[Severity::Info, Severity::Warning, Severity::Error, Severity::Critical] {
            let json = serde_json::to_string(s).unwrap();
            let back: Severity = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }

    // ── AnalysisConfig tests ─────────────────────────────────────────

    #[test]
    fn test_analysis_config_default() {
        let cfg = AnalysisConfig::default();
        assert!(!cfg.skip_llm_validation);
        assert!((cfg.min_severity - 0.0).abs() < f64::EPSILON);
        assert!(!cfg.include_metrics);
    }

    #[test]
    fn test_analysis_config_serde_defaults_from_missing_fields() {
        let json = r#"{}"#;
        let cfg: AnalysisConfig = serde_json::from_str(json).unwrap();
        assert!(!cfg.skip_llm_validation);
        assert!((cfg.min_severity - 0.0).abs() < f64::EPSILON);
        assert!(!cfg.include_metrics);
    }

    #[test]
    fn test_analysis_config_partial_override() {
        let json = r#"{"skip_llm_validation": true}"#;
        let cfg: AnalysisConfig = serde_json::from_str(json).unwrap();
        assert!(cfg.skip_llm_validation);
        assert!((cfg.min_severity - 0.0).abs() < f64::EPSILON);
        assert!(!cfg.include_metrics);
    }

    #[test]
    fn test_analysis_config_custom_severity() {
        let json = r#"{"min_severity": 0.5}"#;
        let cfg: AnalysisConfig = serde_json::from_str(json).unwrap();
        assert!((cfg.min_severity - 0.5).abs() < f64::EPSILON);
    }

    // ── AnalyzeRequest tests ─────────────────────────────────────────

    #[test]
    fn test_analyze_request_new_generates_uuid() {
        let req1 = AnalyzeRequest::new("/repo", "abc123");
        let req2 = AnalyzeRequest::new("/repo", "abc123");
        assert_eq!(req1.repo_path, "/repo");
        assert_eq!(req1.commit_hash, "abc123");
        assert_ne!(req1.request_id, req2.request_id);
    }

    #[test]
    fn test_analyze_request_default_config() {
        let req = AnalyzeRequest::new("/repo", "abc123");
        assert_eq!(req.engines, vec![]);
        assert!(!req.config.skip_llm_validation);
    }

    #[test]
    fn test_analyze_request_serde_roundtrip() {
        let req = AnalyzeRequest {
            repo_path: "/tmp/test".into(),
            commit_hash: "deadbeef".into(),
            engines: vec![Engine::Mgtg, Engine::Dependency],
            config: AnalysisConfig {
                skip_llm_validation: true,
                min_severity: 0.3,
                include_metrics: true,
            },
            request_id: "test-id".into(),
        };
        let json = serde_json::to_string_pretty(&req).unwrap();
        let back: AnalyzeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, back);
    }

    // ── Finding tests ────────────────────────────────────────────────

    #[test]
    fn test_finding_new_defaults() {
        let f = Finding::new("F001", Engine::Mgtg, "src/main.rs", 42, "Resource leak", Severity::Error);
        assert_eq!(f.id, "F001");
        assert!(!f.validated);
        assert!(f.suggested_fix.is_none());
    }

    #[test]
    fn test_finding_with_fix_and_validated() {
        let f = Finding::new("F001", Engine::Mgtg, "src/main.rs", 42, "Resource leak", Severity::Error)
            .with_fix("Add close() call")
            .with_validated(true);
        assert_eq!(f.suggested_fix, Some("Add close() call".into()));
        assert!(f.validated);
    }

    #[test]
    fn test_finding_serde_roundtrip() {
        let f = Finding {
            id: "MGTG-M001".into(),
            engine: Engine::Mgtg,
            file_path: "src/db.rs".into(),
            line: 88,
            message: "File handle may leak".into(),
            severity: Severity::Critical,
            validated: true,
            suggested_fix: Some("Use `with_open`".into()),
        };
        let json = serde_json::to_string(&f).unwrap();
        let back: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }

    #[test]
    fn test_finding_serde_optional_fix_omitted_when_none() {
        let f = Finding::new("F001", Engine::Mgtg, "f.rs", 1, "msg", Severity::Info);
        let json = serde_json::to_string(&f).unwrap();
        assert!(!json.contains("suggested_fix"));
    }

    #[test]
    fn test_finding_serde_optional_fix_present_when_some() {
        let f = Finding::new("F001", Engine::Mgtg, "f.rs", 1, "msg", Severity::Info)
            .with_fix("fix it");
        let json = serde_json::to_string(&f).unwrap();
        assert!(json.contains("suggested_fix"));
    }

    // ── MetricsSummary tests ─────────────────────────────────────────

    #[test]
    fn test_metrics_summary_default() {
        let m = MetricsSummary {
            cyclomatic_max: 0.0,
            cognitive_max: 0.0,
            nesting_max: 0.0,
            total_functions: 0,
            total_files: 0,
            dependency_fan_in_max: 0.0,
            dependency_fan_out_max: 0.0,
            circular_dependencies: 0,
            rse_survivability: 0.0,
            rse_complexity_max: 0.0,
            rse_memory_per_request: 0.0,
            rse_safe_rps: 0.0,
        };
        assert!(!m.any_non_default());
    }

    #[test]
    fn test_metrics_summary_any_non_default_true() {
        let m = MetricsSummary {
            cyclomatic_max: 15.0,
            ..Default::default()
        };
        assert!(m.any_non_default());

        let m = MetricsSummary {
            total_functions: 100,
            ..Default::default()
        };
        assert!(m.any_non_default());
    }

    #[test]
    fn test_metrics_summary_serde_roundtrip() {
        let m = MetricsSummary {
            cyclomatic_max: 12.5,
            cognitive_max: 25.0,
            nesting_max: 5.0,
            total_functions: 200,
            total_files: 50,
            dependency_fan_in_max: 30.0,
            dependency_fan_out_max: 15.0,
            circular_dependencies: 2,
            rse_survivability: 0.85,
            rse_complexity_max: 3.0,
            rse_memory_per_request: 4096.0,
            rse_safe_rps: 500.0,
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: MetricsSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    // ── AnalysisResult tests ─────────────────────────────────────────

    #[test]
    fn test_analysis_result_new_empty() {
        let r = AnalysisResult::new("req-1", "abc");
        assert_eq!(r.request_id, "req-1");
        assert_eq!(r.commit_hash, "abc");
        assert!(r.findings.is_empty());
        assert!((r.overall_risk - 0.0).abs() < f64::EPSILON);
        assert!(!r.blocked_merge);
    }

    #[test]
    fn test_analysis_result_finding_count_by_severity() {
        let r = AnalysisResult {
            request_id: "r1".into(),
            commit_hash: "abc".into(),
            overall_risk: 0.5,
            findings: vec![
                Finding::new("1", Engine::Mgtg, "f.rs", 1, "a", Severity::Error),
                Finding::new("2", Engine::Mgtg, "f.rs", 2, "b", Severity::Error),
                Finding::new("3", Engine::Mgtg, "f.rs", 3, "c", Severity::Warning),
                Finding::new("4", Engine::Mgtg, "f.rs", 4, "d", Severity::Critical),
            ],
            recommendations: vec![],
            metrics: None,
            processing_time_ms: 100.0,
            blocked_merge: false,
        };
        assert_eq!(r.finding_count_by_severity(Severity::Error), 2);
        assert_eq!(r.finding_count_by_severity(Severity::Warning), 1);
        assert_eq!(r.finding_count_by_severity(Severity::Info), 0);
        assert_eq!(r.finding_count_by_severity(Severity::Critical), 1);
    }

    #[test]
    fn test_analysis_result_highest_severity() {
        let r = AnalysisResult {
            findings: vec![
                Finding::new("1", Engine::Mgtg, "f.rs", 1, "a", Severity::Warning),
                Finding::new("2", Engine::Mgtg, "f.rs", 2, "b", Severity::Error),
            ],
            ..AnalysisResult::new("r1", "abc")
        };
        assert_eq!(r.highest_severity(), Some(Severity::Error));

        let r = AnalysisResult::new("r1", "abc");
        assert_eq!(r.highest_severity(), None);
    }

    #[test]
    fn test_analysis_result_is_failing() {
        let r = AnalysisResult {
            blocked_merge: true,
            ..AnalysisResult::new("r1", "abc")
        };
        assert!(r.is_failing());

        let r = AnalysisResult {
            overall_risk: 0.9,
            ..AnalysisResult::new("r1", "abc")
        };
        assert!(r.is_failing());

        let r = AnalysisResult::new("r1", "abc");
        assert!(!r.is_failing());
    }

    #[test]
    fn test_analysis_result_serde_roundtrip() {
        let r = AnalysisResult {
            request_id: "req-42".into(),
            commit_hash: "deadbeef".into(),
            overall_risk: 0.75,
            findings: vec![
                Finding::new("F1", Engine::Mgtg, "src/main.rs", 10, "leak", Severity::Error)
                    .with_validated(true),
            ],
            recommendations: vec![Recommendation::new("Fix the leak", 0.9)],
            metrics: Some(MetricsSummary {
                cyclomatic_max: 15.0,
                ..Default::default()
            }),
            processing_time_ms: 1234.56,
            blocked_merge: false,
        };
        let json = serde_json::to_string_pretty(&r).unwrap();
        let back: AnalysisResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn test_analysis_result_metrics_omitted_when_none() {
        let r = AnalysisResult::new("r1", "abc");
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("metrics"));
    }

    #[test]
    fn test_analysis_result_metrics_present_when_some() {
        let r = AnalysisResult {
            metrics: Some(MetricsSummary::default()),
            ..AnalysisResult::new("r1", "abc")
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("metrics"));
    }

    // ── Recommendation tests ─────────────────────────────────────────

    #[test]
    fn test_recommendation_new() {
        let rec = Recommendation::new("Refactor this", 0.8);
        assert_eq!(rec.text, "Refactor this");
        assert!((rec.priority - 0.8).abs() < f64::EPSILON);
        assert!(rec.affected_files.is_empty());
    }

    #[test]
    fn test_recommendation_serde_roundtrip() {
        let rec = Recommendation {
            text: "Extract module".into(),
            affected_files: vec!["src/a.rs".into(), "src/b.rs".into()],
            priority: 0.95,
        };
        let json = serde_json::to_string(&rec).unwrap();
        let back: Recommendation = serde_json::from_str(&json).unwrap();
        assert_eq!(rec, back);
    }

    // ── Feedback tests ───────────────────────────────────────────────

    #[test]
    fn test_feedback_new() {
        let fb = Feedback::new("pred-1", "finding-1", FeedbackOutcome::Correct, "user-42");
        assert_eq!(fb.prediction_id, "pred-1");
        assert_eq!(fb.finding_id, "finding-1");
        assert_eq!(fb.outcome, FeedbackOutcome::Correct);
        assert!(fb.comment.is_empty());
        assert!(fb.timestamp_ms > 0);
    }

    #[test]
    fn test_feedback_with_comment() {
        let fb = Feedback::new("p1", "f1", FeedbackOutcome::FalseAlarm, "u1")
            .with_comment("Not a real bug");
        assert_eq!(fb.comment, "Not a real bug");
    }

    #[test]
    fn test_feedback_is_positive_and_negative() {
        let fb = Feedback::new("p1", "f1", FeedbackOutcome::Correct, "u1");
        assert!(fb.is_positive());
        assert!(!fb.is_negative());

        let fb = Feedback::new("p1", "f1", FeedbackOutcome::FalseAlarm, "u1");
        assert!(fb.is_negative());
        assert!(!fb.is_positive());
    }

    #[test]
    fn test_feedback_outcome_serde() {
        for outcome in &[FeedbackOutcome::Correct, FeedbackOutcome::FalseAlarm, FeedbackOutcome::Partial, FeedbackOutcome::Unsure] {
            let json = serde_json::to_string(outcome).unwrap();
            let back: FeedbackOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(*outcome, back);
        }
    }

    #[test]
    fn test_feedback_serde_roundtrip() {
        let fb = Feedback {
            prediction_id: "pred-1".into(),
            finding_id: "finding-1".into(),
            outcome: FeedbackOutcome::Partial,
            comment: "Sort of correct".into(),
            user_id: "user-abc".into(),
            timestamp_ms: 1700000000000,
        };
        let json = serde_json::to_string(&fb).unwrap();
        let back: Feedback = serde_json::from_str(&json).unwrap();
        assert_eq!(fb, back);
    }

    // ── Validation tests ─────────────────────────────────────────────

    #[test]
    fn test_validate_risk_valid() {
        assert!((validate_risk(0.0).unwrap() - 0.0).abs() < f64::EPSILON);
        assert!((validate_risk(0.5).unwrap() - 0.5).abs() < f64::EPSILON);
        assert!((validate_risk(1.0).unwrap() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_validate_risk_invalid_negative() {
        assert!(matches!(validate_risk(-0.1), Err(SchemaError::RiskOutOfBounds(_))));
    }

    #[test]
    fn test_validate_risk_invalid_above_one() {
        assert!(matches!(validate_risk(1.1), Err(SchemaError::RiskOutOfBounds(_))));
    }

    // ── Edge case tests ──────────────────────────────────────────────

    #[test]
    fn test_finding_with_empty_message() {
        let f = Finding::new("F1", Engine::Mgtg, "f.rs", 1, "", Severity::Info);
        assert_eq!(f.message, "");
    }

    #[test]
    fn test_analysis_result_with_thousand_findings() {
        let findings: Vec<Finding> = (0..1000)
            .map(|i| Finding::new(&format!("F{}", i), Engine::Mgtg, "f.rs", i, "msg", Severity::Info))
            .collect();
        let r = AnalysisResult {
            findings,
            ..AnalysisResult::new("r1", "abc")
        };
        assert_eq!(r.findings.len(), 1000);
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.len() > 1000);
    }

    #[test]
    fn test_severity_serde_unknown_variant_fails() {
        let result: Result<Severity, _> = serde_json::from_str(r#""unknown""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_serde_unknown_variant_fails() {
        let result: Result<Engine, _> = serde_json::from_str(r#""nonexistent""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_feature_map_type_alias() {
        let mut map: FeatureMap = HashMap::new();
        map.insert("cyclomatic_max".into(), 15.0);
        map.insert("cognitive_max".into(), 30.0);
        assert_eq!(map.len(), 2);
        let json = serde_json::to_string(&map).unwrap();
        let back: FeatureMap = serde_json::from_str(&json).unwrap();
        assert_eq!(map, back);
    }

    #[test]
    fn test_component_health_serde() {
        let h = ComponentHealth {
            name: "mgtg".into(),
            status: HealthStatus::Healthy,
            message: None,
            last_heartbeat_ms: 1000,
        };
        let json = serde_json::to_string(&h).unwrap();
        let back: ComponentHealth = serde_json::from_str(&json).unwrap();
        assert_eq!(h, back);

        let h2 = ComponentHealth {
            name: "ml".into(),
            status: HealthStatus::Degraded,
            message: Some("Model not loaded".into()),
            last_heartbeat_ms: 0,
        };
        let json = serde_json::to_string(&h2).unwrap();
        let back2: ComponentHealth = serde_json::from_str(&json).unwrap();
        assert_eq!(h2, back2);
    }

    #[test]
    fn test_analysis_config_deserialize_from_toml() {
        let toml_str = r#"
skip_llm_validation = true
min_severity = 0.2
include_metrics = true
"#;
        let cfg: AnalysisConfig = toml::from_str(toml_str).unwrap();
        assert!(cfg.skip_llm_validation);
        assert!((cfg.min_severity - 0.2).abs() < f64::EPSILON);
        assert!(cfg.include_metrics);
    }

    #[test]
    fn test_default_impl_for_metrics() {
        let m = MetricsSummary::default();
        assert!(!m.any_non_default());
    }

    #[test]
    fn test_default_impl_for_config() {
        let c = AnalysisConfig::default();
        assert!(!c.skip_llm_validation);
        assert!((c.min_severity - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_engine_eq_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Engine::Mgtg);
        set.insert(Engine::Mgtg);
        assert_eq!(set.len(), 1);
        set.insert(Engine::Ml);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_severity_eq_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Severity::Error);
        set.insert(Severity::Error);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_analysis_result_deserialize_with_missing_fields() {
        let json = r#"{"request_id":"r1","commit_hash":"abc"}"#;
        let r: AnalysisResult = serde_json::from_str(json).unwrap();
        assert_eq!(r.request_id, "r1");
        assert_eq!(r.commit_hash, "abc");
        assert!((r.overall_risk - 0.0).abs() < f64::EPSILON);
        assert!(r.findings.is_empty());
        assert!(!r.blocked_merge);
    }

    // ── Proptest strategies and helpers ───────────────────────────────

    use proptest::prelude::{any, prop, Strategy};

    fn f64_approx_eq(a: f64, b: f64) -> bool {
        if a == b {
            return true;
        }
        let diff = (a - b).abs();
        let max_magnitude = a.abs().max(b.abs()).max(1.0);
        diff / max_magnitude < 1e-12 || diff < 1e-12
    }

    fn recommendation_approx_eq(a: &Recommendation, b: &Recommendation) -> bool {
        a.text == b.text
            && a.affected_files == b.affected_files
            && f64_approx_eq(a.priority, b.priority)
    }

    fn metrics_approx_eq(a: &MetricsSummary, b: &MetricsSummary) -> bool {
        f64_approx_eq(a.cyclomatic_max, b.cyclomatic_max)
            && f64_approx_eq(a.cognitive_max, b.cognitive_max)
            && f64_approx_eq(a.nesting_max, b.nesting_max)
            && a.total_functions == b.total_functions
            && a.total_files == b.total_files
            && f64_approx_eq(a.dependency_fan_in_max, b.dependency_fan_in_max)
            && f64_approx_eq(a.dependency_fan_out_max, b.dependency_fan_out_max)
            && a.circular_dependencies == b.circular_dependencies
    }

    fn analysis_result_approx_eq(a: &AnalysisResult, b: &AnalysisResult) -> bool {
        a.request_id == b.request_id
            && a.commit_hash == b.commit_hash
            && f64_approx_eq(a.overall_risk, b.overall_risk)
            && a.findings == b.findings
            && a.recommendations.len() == b.recommendations.len()
            && a.recommendations.iter().zip(b.recommendations.iter()).all(|(ra, rb)| recommendation_approx_eq(ra, rb))
            && match (&a.metrics, &b.metrics) {
                (Some(ma), Some(mb)) => metrics_approx_eq(ma, mb),
                (None, None) => true,
                _ => false,
            }
            && f64_approx_eq(a.processing_time_ms, b.processing_time_ms)
            && a.blocked_merge == b.blocked_merge
    }

    fn engine_strategy() -> impl Strategy<Value = Engine> {
        prop::sample::select(vec![
            Engine::Mgtg,
            Engine::Process,
            Engine::Dependency,
            Engine::Ml,
            Engine::Hitl,
            Engine::RuntimeSurvivability,
        ])
    }

    fn severity_strategy() -> impl Strategy<Value = Severity> {
        prop::sample::select(vec![
            Severity::Info,
            Severity::Warning,
            Severity::Error,
            Severity::Critical,
        ])
    }

    fn finding_strategy() -> impl Strategy<Value = Finding> {
        (
            "[a-zA-Z0-9_-]{0,20}",
            engine_strategy(),
            "[a-zA-Z0-9_/.-]{0,60}",
            any::<u32>(),
            "[a-zA-Z0-9_ -]{0,60}",
            severity_strategy(),
            any::<bool>(),
            prop::option::of("[a-zA-Z0-9_ -]{0,60}"),
        )
            .prop_map(
                |(id, engine, file_path, line, message, severity, validated, suggested_fix)| Finding {
                    id,
                    engine,
                    file_path,
                    line,
                    message,
                    severity,
                    validated,
                    suggested_fix,
                },
            )
    }

    fn recommendation_strategy() -> impl Strategy<Value = Recommendation> {
        (
            "[a-zA-Z0-9_ -]{0,60}",
            prop::collection::vec("[a-zA-Z0-9_/.-]{1,20}", 0..5),
            -1e6f64..1e6f64,
        )
            .prop_map(|(text, affected_files, priority)| Recommendation {
                text,
                affected_files,
                priority,
            })
    }

    fn metrics_summary_strategy() -> impl Strategy<Value = MetricsSummary> {
        (
            0.0f64..1e6,
            0.0f64..1e6,
            0.0f64..1e4,
            any::<u32>(),
            any::<u32>(),
            0.0f64..1e4,
            0.0f64..1e4,
            any::<u32>(),
            0.0f64..1.0,
            0.0f64..1e4,
            0.0f64..1e9,
            0.0f64..1e5,
        )
            .prop_map(
                |(cmax, cogmax, nmax, tfunc, tfiles, dfan_in, dfan_out, circ, rse_surv, rse_cplx, rse_mem, rse_rps)| MetricsSummary {
                    cyclomatic_max: cmax,
                    cognitive_max: cogmax,
                    nesting_max: nmax,
                    total_functions: tfunc,
                    total_files: tfiles,
                    dependency_fan_in_max: dfan_in,
                    dependency_fan_out_max: dfan_out,
                    circular_dependencies: circ,
                    rse_survivability: rse_surv,
                    rse_complexity_max: rse_cplx,
                    rse_memory_per_request: rse_mem,
                    rse_safe_rps: rse_rps,
                },
            )
    }

    fn analysis_result_strategy() -> impl Strategy<Value = AnalysisResult> {
        (
            "[a-f0-9-]{0,36}",
            "[a-f0-9]{6,40}",
            0.0f64..=1.0,
            prop::collection::vec(finding_strategy(), 0..10),
            prop::collection::vec(recommendation_strategy(), 0..5),
            prop::option::of(metrics_summary_strategy()),
            0.0f64..100_000.0,
            any::<bool>(),
        )
            .prop_map(
                |(
                    request_id,
                    commit_hash,
                    overall_risk,
                    findings,
                    recommendations,
                    metrics,
                    processing_time_ms,
                    blocked_merge,
                )| {
                    AnalysisResult {
                        request_id,
                        commit_hash,
                        overall_risk,
                        findings,
                        recommendations,
                        metrics,
                        processing_time_ms,
                        blocked_merge,
                    }
                },
            )
    }

    #[test]
    fn proptest_analysis_result_serde_json() {
        proptest::proptest!(|(result in analysis_result_strategy())| {
            let json = serde_json::to_string(&result).unwrap();
            let back: AnalysisResult = serde_json::from_str(&json).unwrap();
            assert!(analysis_result_approx_eq(&result, &back),
                "JSON roundtrip mismatch: left={:?} right={:?}", result, back);
        });
    }

    #[test]
    fn proptest_analysis_result_serde_yaml() {
        proptest::proptest!(|(result in analysis_result_strategy())| {
            let yaml = serde_yaml::to_string(&result).unwrap();
            let back: AnalysisResult = serde_yaml::from_str(&yaml).unwrap();
            assert!(analysis_result_approx_eq(&result, &back),
                "YAML roundtrip mismatch: left={:?} right={:?}", result, back);
        });
    }

    #[test]
    fn proptest_analysis_result_serde_toml() {
        proptest::proptest!(|(result in analysis_result_strategy())| {
            let toml_str = toml::to_string(&result).unwrap();
            let back: AnalysisResult = toml::from_str(&toml_str).unwrap();
            assert!(analysis_result_approx_eq(&result, &back),
                "TOML roundtrip mismatch: left={:?} right={:?}", result, back);
        });
    }

    #[test]
    fn proptest_severity_rank_roundtrip() {
        proptest::proptest!(|(r in 0u8..=3u8)| {
            let s = Severity::from_rank(r).unwrap();
            assert_eq!(s.rank(), r);
        });
    }

    // ── Edge case: NaN / Infinity risk ────────────────────────────────

    #[test]
    fn test_validate_risk_nan() {
        let result = validate_risk(f64::NAN);
        assert!(result.is_err());
        assert!(matches!(result, Err(SchemaError::RiskOutOfBounds(_))));
    }

    #[test]
    fn test_validate_risk_neg_infinity() {
        let result = validate_risk(f64::NEG_INFINITY);
        assert!(result.is_err());
        assert!(matches!(result, Err(SchemaError::RiskOutOfBounds(_))));
    }

    #[test]
    fn test_validate_risk_pos_infinity() {
        let result = validate_risk(f64::INFINITY);
        assert!(result.is_err());
        assert!(matches!(result, Err(SchemaError::RiskOutOfBounds(_))));
    }

    // ── Edge case: Finding with empty id ──────────────────────────────

    #[test]
    fn test_finding_empty_id_serializes() {
        let f = Finding {
            id: "".into(),
            engine: Engine::Mgtg,
            file_path: "f.rs".into(),
            line: 1,
            message: "test".into(),
            severity: Severity::Info,
            validated: false,
            suggested_fix: None,
        };
        let json = serde_json::to_string(&f).unwrap();
        assert!(json.contains(r#""id":""#));
        let back: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }

    // ── Edge case: Unicode strings ────────────────────────────────────

    #[test]
    fn test_finding_unicode_roundtrip() {
        let f = Finding {
            id: "F-Ω-💥".into(),
            engine: Engine::Mgtg,
            file_path: "src/文件.rs".into(),
            line: 42,
            message: "مرحباً بالعالم ← test".into(),
            severity: Severity::Info,
            validated: false,
            suggested_fix: Some("أصلح هذا".into()),
        };
        let json = serde_json::to_string(&f).unwrap();
        let back: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);

        let yaml = serde_yaml::to_string(&f).unwrap();
        let back: Finding = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(f, back);
    }

    // ── Edge case: Extreme AnalyzeRequest ─────────────────────────────

    #[test]
    fn test_analyze_request_extreme_values() {
        let req = AnalyzeRequest {
            repo_path: "z".repeat(10_000),
            commit_hash: "a".repeat(1000),
            engines: Engine::ALL.to_vec(),
            config: AnalysisConfig {
                skip_llm_validation: true,
                min_severity: 1.0,
                include_metrics: true,
            },
            request_id: "b".repeat(1000),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: AnalyzeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, back);
    }

    // ── Edge case: MetricsSummary with max values ─────────────────────

    #[test]
    fn test_metrics_summary_max_values() {
        let m = MetricsSummary {
            cyclomatic_max: f64::MAX,
            cognitive_max: f64::MAX,
            nesting_max: f64::MAX,
            total_functions: u32::MAX,
            total_files: u32::MAX,
            dependency_fan_in_max: f64::MAX,
            dependency_fan_out_max: f64::MAX,
            circular_dependencies: u32::MAX,
            rse_survivability: 1.0,
            rse_complexity_max: f64::MAX,
            rse_memory_per_request: f64::MAX,
            rse_safe_rps: f64::MAX,
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: MetricsSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);

        let yaml = serde_yaml::to_string(&m).unwrap();
        let back: MetricsSummary = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(m, back);
    }

    // ── Edge case: AnalysisResult with 0 findings ─────────────────────

    #[test]
    fn test_analysis_result_empty_findings_roundtrip() {
        let r = AnalysisResult {
            request_id: "req-empty".into(),
            commit_hash: "abc123".into(),
            overall_risk: 0.5,
            findings: vec![],
            recommendations: vec![],
            metrics: Some(MetricsSummary::default()),
            processing_time_ms: 100.0,
            blocked_merge: false,
        };
        assert!(r.findings.is_empty());
        let json = serde_json::to_string(&r).unwrap();
        let back: AnalysisResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    // ── Edge case: Engine::from_name case sensitivity ─────────────────

    #[test]
    fn test_engine_from_name_case_sensitivity() {
        assert_eq!(Engine::from_name("Mgtg"), None);
        assert_eq!(Engine::from_name("MGTG"), None);
        assert_eq!(Engine::from_name("Process"), None);
        assert_eq!(Engine::from_name("PROCESS"), None);
        assert_eq!(Engine::from_name("mgtg"), Some(Engine::Mgtg));
        assert_eq!(Engine::from_name("process"), Some(Engine::Process));
    }

    // ── Edge case: Engine serde unknown variant ───────────────────────

    #[test]
    fn test_engine_serde_unknown_variant_yaml() {
        let yaml = "unknown_engine\n";
        let result: Result<Engine, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }
}
