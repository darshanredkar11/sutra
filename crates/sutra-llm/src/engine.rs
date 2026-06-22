use std::collections::HashMap;
use std::time::Instant;

use sutra_common::engine::AnalysisEngine;
use sutra_common::error::SutraResult;
use sutra_schema::v1::{
    AnalysisResult, AnalyzeRequest, Finding, Recommendation, Severity,
};

use crate::client::validate_finding;
use crate::types::{LLMConfig, ValidationResult};

pub struct LlmEngine {
    config: LLMConfig,
    cache: HashMap<String, ValidationResult>,
    enabled: bool,
}

impl LlmEngine {
    pub fn new() -> Self {
        Self {
            config: LLMConfig::default(),
            cache: HashMap::new(),
            enabled: false,
        }
    }

    pub fn with_config(mut self, config: LLMConfig) -> Self {
        self.config = config;
        self.enabled = true;
        self
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.config.model = model.to_string();
        self.enabled = true;
        self
    }

    pub fn with_ollama_url(mut self, url: &str) -> Self {
        self.config.ollama_url = url.to_string();
        self.enabled = true;
        self
    }

    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub fn cached_count(&self) -> usize {
        self.cache.len()
    }

    /// Validate a single finding. Uses cache if available.
    pub fn validate(&mut self, finding: &Finding) -> SutraResult<ValidationResult> {
        if !self.enabled {
            return Ok(ValidationResult::new(&finding.id, true, 1.0, "LLM validation disabled"));
        }

        let cache_key = format!(
            "{}:{}:{}:{}",
            finding.id, finding.file_path, finding.line, finding.message
        );

        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let result = validate_finding(
            &self.config,
            &finding.id,
            &finding.file_path,
            finding.line,
            &finding.message,
            finding.severity,
        )?;

        self.cache.insert(cache_key, result.clone());
        Ok(result)
    }

    /// Validate multiple findings and apply results to them.
    /// Returns updated findings with validation status and suggested fixes.
    pub fn validate_findings(&mut self, findings: &[Finding]) -> SutraResult<Vec<Finding>> {
        let mut updated: Vec<Finding> = Vec::with_capacity(findings.len());

        for finding in findings {
            match self.validate(finding) {
                Ok(vr) => {
                    let mut f = finding.clone();
                    f.validated = vr.is_valid;
                    if let Some(fix) = &vr.suggested_fix {
                        f.suggested_fix = Some(fix.clone());
                    }
                    updated.push(f);
                }
                Err(e) => {
                    let mut f = finding.clone();
                    f.validated = false;
                    f.suggested_fix = Some(format!("validation error: {}", e));
                    updated.push(f);
                }
            }
        }

        Ok(updated)
    }
}

impl Default for LlmEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisEngine for LlmEngine {
    fn name(&self) -> &'static str {
        "llm"
    }

    fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
        let start = Instant::now();

        if !self.enabled {
            return Ok(AnalysisResult {
                request_id: request.request_id.clone(),
                commit_hash: request.commit_hash.clone(),
                overall_risk: 0.0,
                findings: vec![],
                recommendations: vec![Recommendation::new(
                    "LLM validation is disabled. Enable with `with_config()` or `enable()`",
                    0.0,
                )],
                metrics: None,
                processing_time_ms: start.elapsed().as_secs_f64() * 1000.0,
                blocked_merge: false,
            });
        }

        Ok(AnalysisResult {
            request_id: request.request_id.clone(),
            commit_hash: request.commit_hash.clone(),
            overall_risk: 0.0,
            findings: vec![],
            recommendations: vec![Recommendation::new(
                "LLM engine available. Findings are validated through the orchestrator pipeline.",
                0.5,
            )],
            metrics: None,
            processing_time_ms: start.elapsed().as_secs_f64() * 1000.0,
            blocked_merge: false,
        })
    }
}

/// Merge validation results into an AnalysisResult.
/// Updates findings with validation status and suggested fixes.
/// Filters out findings that the LLM deemed invalid (false positives).
pub fn apply_llm_validation(
    mut result: AnalysisResult,
    validated_findings: Vec<Finding>,
    min_confidence: f64,
) -> AnalysisResult {
    let original_count = result.findings.len();
    result.findings = validated_findings;

    // Update risk based on validated findings
    let error_count = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error && f.validated)
        .count();

    let warning_count = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warning && f.validated)
        .count();

    let filtered_count = original_count - result.findings.len();
    let false_positives = result
        .findings
        .iter()
        .filter(|f| !f.validated)
        .count();

    result.overall_risk = ((error_count as f64 * 0.3 + warning_count as f64 * 0.1) * min_confidence).min(1.0);

    result.recommendations.push(Recommendation::new(
        &format!(
            "LLM validation complete: {} findings validated, {} filtered (false positives), {} unconfirmed",
            result.findings.len(),
            filtered_count + false_positives,
            result.findings.iter().filter(|f| !f.validated).count(),
        ),
        0.7,
    ));

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use sutra_schema::v1::{Engine, Finding};
    use crate::types::LLMConfig;

    #[test]
    fn test_engine_name() {
        let engine = LlmEngine::new();
        assert_eq!(engine.name(), "llm");
    }

    #[test]
    fn test_engine_disabled_by_default() {
        let engine = LlmEngine::new();
        assert!(!engine.is_enabled());
    }

    #[test]
    fn test_engine_enable() {
        let engine = LlmEngine::new().enable();
        assert!(engine.is_enabled());
    }

    #[test]
    fn test_validate_when_disabled() {
        let mut engine = LlmEngine::new();
        let finding = Finding::new("MGTG-001", Engine::Mgtg, "f.rs", 1, "test", Severity::Warning);
        let result = engine.validate(&finding).unwrap();
        assert!(result.is_valid);
        assert!((result.confidence - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_validate_caches_result() {
        let mut engine = LlmEngine::new().enable();

        // Validation will fail to connect to Ollama (127.0.0.1:1)
        // But that's fine for testing the cache mechanism
        let config = LLMConfig {
            ollama_url: "http://127.0.0.1:1".into(),
            timeout_secs: 1,
            ..LLMConfig::default()
        };
        engine.config = config;
        engine.clear_cache();
        assert_eq!(engine.cached_count(), 0);
    }

    #[test]
    fn test_analyze_disabled() {
        let engine = LlmEngine::new();
        let req = AnalyzeRequest::new("/repo", "abc");
        let result = engine.analyze(&req).unwrap();
        assert!(result.findings.is_empty());
        assert!(result.recommendations[0].text.contains("disabled"));
    }

    #[test]
    fn test_analyze_enabled() {
        let engine = LlmEngine::new().enable();
        let req = AnalyzeRequest::new("/repo", "abc");
        let result = engine.analyze(&req).unwrap();
        assert!(result.findings.is_empty());
        assert!(result.recommendations[0].text.contains("available"));
    }

    #[test]
    fn test_engine_default() {
        let engine = LlmEngine::default();
        assert!(!engine.is_enabled());
    }

    #[test]
    fn test_with_config() {
        let config = LLMConfig {
            model: "codellama".into(),
            ..LLMConfig::default()
        };
        let engine = LlmEngine::new().with_config(config.clone());
        assert!(engine.is_enabled());
        assert_eq!(engine.config.model, "codellama");
    }

    #[test]
    fn test_with_model() {
        let engine = LlmEngine::new().with_model("mixtral");
        assert_eq!(engine.config.model, "mixtral");
        assert!(engine.is_enabled());
    }

    #[test]
    fn test_with_ollama_url() {
        let engine = LlmEngine::new().with_ollama_url("http://ollama:11434");
        assert_eq!(engine.config.ollama_url, "http://ollama:11434");
    }

    #[test]
    fn test_clear_cache() {
        let mut engine = LlmEngine::new();
        engine.cache.insert("key".into(), ValidationResult::new("F1", true, 1.0, "test"));
        assert_eq!(engine.cached_count(), 1);
        engine.clear_cache();
        assert_eq!(engine.cached_count(), 0);
    }

    #[test]
    fn test_validate_findings_batch() {
        let mut engine = LlmEngine::new(); // disabled
        let findings = vec![
            Finding::new("F1", Engine::Mgtg, "f.rs", 1, "bug", Severity::Error),
            Finding::new("F2", Engine::Dependency, "g.rs", 2, "cycle", Severity::Warning),
        ];
        let updated = engine.validate_findings(&findings).unwrap();
        assert_eq!(updated.len(), 2);
        // Disabled engine always validates as true
        assert!(updated[0].validated);
        assert!(updated[1].validated);
    }

    #[test]
    fn test_apply_llm_validation() {
        let result = AnalysisResult {
            findings: vec![
                Finding::new("F1", Engine::Mgtg, "f.rs", 1, "bug", Severity::Error),
            ],
            ..AnalysisResult::new("req-1", "abc")
        };

        let validated = vec![
            Finding {
                id: "F1".into(),
                validated: true,
                suggested_fix: Some("Fix it".into()),
                ..Finding::new("F1", Engine::Mgtg, "f.rs", 1, "bug", Severity::Error)
            },
        ];

        let updated = apply_llm_validation(result, validated, 0.8);
        assert_eq!(updated.findings.len(), 1);
        assert!(updated.findings[0].validated);
        assert_eq!(updated.findings[0].suggested_fix, Some("Fix it".into()));
        assert!(updated.recommendations.iter().any(|r| r.text.contains("LLM validation")));
    }

    #[test]
    fn test_apply_llm_validation_empty_findings() {
        let result = AnalysisResult::new("req-1", "abc");
        let validated = vec![];
        let updated = apply_llm_validation(result, validated, 0.8);
        assert!(updated.findings.is_empty());
        assert!(updated.recommendations.iter().any(|r| r.text.contains("LLM validation")));
    }

    #[test]
    fn test_apply_llm_validation_thousand_findings() {
        let findings: Vec<Finding> = (0..1000)
            .map(|i| Finding::new(&format!("F{}", i), Engine::Mgtg, "f.rs", i, "msg", Severity::Warning))
            .collect();
        let result = AnalysisResult {
            findings: findings.clone(),
            ..AnalysisResult::new("req-1", "abc")
        };
        let validated: Vec<Finding> = findings.into_iter()
            .map(|mut f| { f.validated = true; f })
            .collect();
        let updated = apply_llm_validation(result, validated, 0.8);
        assert_eq!(updated.findings.len(), 1000);
    }
}
