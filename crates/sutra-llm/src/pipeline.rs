use sutra_common::error::SutraResult;
use sutra_orchestrator::coordinator::Orchestrator;
use sutra_schema::v1::{AnalysisResult, AnalyzeRequest, Engine, Recommendation};

use crate::engine::LlmEngine;
use crate::types::LLMConfig;

/// Run the full analysis pipeline: all engines + LLM validation.
/// Skips the LLM engine from the initial run since it's used for post-processing.
pub fn analyze_with_llm(
    orchestrator: &Orchestrator,
    request: &AnalyzeRequest,
    llm_config: Option<LLMConfig>,
    min_confidence: f64,
) -> SutraResult<AnalysisResult> {
    let mut llm_engine = match llm_config {
        Some(config) => LlmEngine::new().with_config(config),
        None => LlmEngine::new(),
    };

    let mut updated_engines = request.engines.clone();
    updated_engines.retain(|e| *e != Engine::Ml);

    let mut llm_request = request.clone();
    llm_request.engines = updated_engines;

    let mut result = orchestrator.analyze(&llm_request)?;

    if llm_engine.is_enabled() && !result.findings.is_empty() {
        let validated = llm_engine.validate_findings(&result.findings)?;
        result.findings = validated;

        result.recommendations.push(Recommendation::new(
            &format!(
                "LLM validation applied: {} findings evaluated, {} valid, {} invalid",
                result.findings.len(),
                result.findings.iter().filter(|f| f.validated).count(),
                result.findings.iter().filter(|f| !f.validated).count(),
            ),
            0.8,
        ));

        let error_count = result
            .findings
            .iter()
            .filter(|f| f.severity == sutra_schema::v1::Severity::Error && f.validated)
            .count();
        let warning_count = result
            .findings
            .iter()
            .filter(|f| f.severity == sutra_schema::v1::Severity::Warning && f.validated)
            .count();

        result.overall_risk =
            ((error_count as f64 * 0.3 + warning_count as f64 * 0.1) * min_confidence).min(1.0);
        result.blocked_merge = error_count > 0;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sutra_common::engine::AnalysisEngine;
    use sutra_common::error::SutraResult;
    use sutra_orchestrator::coordinator::Orchestrator;
    use sutra_schema::v1::{Engine, Finding, Severity};

    struct MockEngine;

    impl AnalysisEngine for MockEngine {
        fn name(&self) -> &'static str {
            "mock"
        }
        fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
            Ok(AnalysisResult {
                request_id: request.request_id.clone(),
                commit_hash: request.commit_hash.clone(),
                overall_risk: 0.5,
                findings: vec![
                    Finding::new("M-001", Engine::Mgtg, "f.rs", 1, "test", Severity::Warning),
                ],
                recommendations: vec![],
                metrics: None,
                processing_time_ms: 5.0,
                blocked_merge: false,
                jit_features: None,
            })
        }
    }

    #[test]
    fn test_analyze_with_llm_disabled() {
        let mut o = Orchestrator::new();
        o.register(Engine::Mgtg, Box::new(MockEngine));

        let req = AnalyzeRequest::new("/repo", "abc");
        let result = analyze_with_llm(&o, &req, None, 0.8).unwrap();

        assert_eq!(result.findings.len(), 1);
        // LLM disabled, so finding keeps default validated=false
        assert!(!result.findings[0].validated);
    }

    #[test]
    fn test_analyze_with_llm_empty_findings() {
        struct EmptyEngine;
        impl AnalysisEngine for EmptyEngine {
            fn name(&self) -> &'static str { "empty" }
            fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
                Ok(AnalysisResult::new(&request.request_id, &request.commit_hash))
            }
        }

        let mut o = Orchestrator::new();
        o.register(Engine::Mgtg, Box::new(EmptyEngine));

        let config = LLMConfig {
            ollama_url: "http://127.0.0.1:1".into(),
            timeout_secs: 1,
            ..LLMConfig::default()
        };

        let req = AnalyzeRequest::new("/repo", "abc");
        let result = analyze_with_llm(&o, &req, Some(config), 0.8).unwrap();

        assert!(result.findings.is_empty());
    }

    #[test]
    fn test_analyze_with_llm_skips_ml_engine() {
        struct MlMock;
        impl AnalysisEngine for MlMock {
            fn name(&self) -> &'static str { "ml" }
            fn analyze(&self, _: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
                Ok(AnalysisResult {
                    findings: vec![Finding::new("ML-001", Engine::Ml, "f.rs", 1, "ml", Severity::Info)],
                    ..AnalysisResult::new("req", "abc")
                })
            }
        }

        let mut o = Orchestrator::new();
        o.register(Engine::Mgtg, Box::new(MockEngine));
        o.register(Engine::Ml, Box::new(MlMock));

        let req = AnalyzeRequest {
            engines: vec![Engine::Mgtg, Engine::Ml],
            ..AnalyzeRequest::new("/repo", "abc")
        };

        let result = analyze_with_llm(&o, &req, None, 0.8).unwrap();
        // ML findings should NOT be included since ML engine is skipped
        assert_eq!(result.findings.len(), 1);
    }
}
