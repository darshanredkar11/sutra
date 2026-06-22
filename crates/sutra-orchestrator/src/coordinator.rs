use std::collections::HashMap;

use sutra_common::engine::AnalysisEngine;
use sutra_common::error::SutraResult;
use sutra_schema::v1::{AnalysisResult, AnalyzeRequest, Engine, Finding, Recommendation};

pub struct Orchestrator {
    engines: HashMap<Engine, Box<dyn AnalysisEngine>>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            engines: HashMap::new(),
        }
    }

    pub fn register(&mut self, engine: Engine, instance: Box<dyn AnalysisEngine>) {
        self.engines.insert(engine, instance);
    }

    pub fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
        let engines_to_run = if request.engines.is_empty() {
            vec![Engine::Mgtg, Engine::Dependency, Engine::Process]
        } else {
            request.engines.clone()
        };

        let mut all_findings: Vec<Finding> = Vec::new();
        let mut all_recommendations: Vec<Recommendation> = Vec::new();
        let mut total_risk = 0.0f64;
        let mut total_time = 0.0f64;
        let mut merged_metrics: Option<sutra_schema::v1::MetricsSummary> = None;
        let mut blocked = false;

        for engine_type in &engines_to_run {
            let Some(engine) = self.engines.get(engine_type) else {
                continue;
            };

            let engine_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                engine.analyze(request)
            }));

            match engine_result {
                Ok(Ok(result)) => {
                    total_risk = total_risk.max(result.overall_risk);
                    total_time += result.processing_time_ms;
                    all_findings.extend(result.findings);
                    all_recommendations.extend(result.recommendations);
                    if result.blocked_merge {
                        blocked = true;
                    }
                    if let Some(metrics) = result.metrics {
                        let m = merged_metrics.get_or_insert_with(Default::default);
                        m.total_files = m.total_files.max(metrics.total_files);
                        m.total_functions = m.total_functions.max(metrics.total_functions);
                        m.cyclomatic_max = m.cyclomatic_max.max(metrics.cyclomatic_max);
                        m.cognitive_max = m.cognitive_max.max(metrics.cognitive_max);
                        m.nesting_max = m.nesting_max.max(metrics.nesting_max);
                        m.dependency_fan_in_max = m.dependency_fan_in_max.max(metrics.dependency_fan_in_max);
                        m.dependency_fan_out_max = m.dependency_fan_out_max.max(metrics.dependency_fan_out_max);
                        m.circular_dependencies = m.circular_dependencies.max(metrics.circular_dependencies);
                    }
                }
                Ok(Err(e)) => {
                    all_findings.push(Finding::new(
                        &format!("ORCH-{}-ERR", engine_type.as_str()),
                        *engine_type,
                        "N/A",
                        1,
                        &format!("Engine '{}' failed: {}", engine_type.as_str(), e),
                        sutra_schema::v1::Severity::Error,
                    ));
                }
                Err(_) => {
                    all_findings.push(Finding::new(
                        &format!("ORCH-{}-ERR", engine_type.as_str()),
                        *engine_type,
                        "N/A",
                        1,
                        &format!("Engine '{}' panicked", engine_type.as_str()),
                        sutra_schema::v1::Severity::Error,
                    ));
                }
            }
        }

        total_risk = if total_risk.is_nan() { 0.0 } else { total_risk.min(1.0) };

        Ok(AnalysisResult {
            request_id: request.request_id.clone(),
            commit_hash: request.commit_hash.clone(),
            overall_risk: total_risk,
            findings: all_findings,
            recommendations: all_recommendations,
            metrics: merged_metrics,
            processing_time_ms: total_time,
            blocked_merge: blocked,
        })
    }

    pub fn analyze_single(&self, request: &AnalyzeRequest, engine_type: Engine) -> SutraResult<AnalysisResult> {
        let Some(engine) = self.engines.get(&engine_type) else {
            return Err(sutra_common::error::SutraError::engine(
                "orchestrator",
                format!("engine '{}' not registered", engine_type.as_str()),
            ));
        };
        engine.analyze(request)
    }

    pub fn engine_names(&self) -> Vec<&'static str> {
        self.engines.values().map(|e| e.name()).collect()
    }

    pub fn health_check(&self) -> Vec<(Engine, bool)> {
        self.engines
            .keys()
            .map(|e| (*e, true))
            .collect()
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sutra_common::error::SutraResult;
    use sutra_schema::v1::{AnalysisResult, AnalyzeRequest, Engine, Finding, Severity};

    struct MockEngine {
        name: &'static str,
        engine_type: Engine,
    }

    impl MockEngine {
        fn new(name: &'static str, engine_type: Engine) -> Self {
            Self { name, engine_type }
        }
    }

    impl AnalysisEngine for MockEngine {
        fn name(&self) -> &'static str {
            self.name
        }

        fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
            Ok(AnalysisResult {
                request_id: request.request_id.clone(),
                commit_hash: request.commit_hash.clone(),
                overall_risk: 0.5,
                findings: vec![Finding::new(
                    "MOCK-001",
                    self.engine_type,
                    "test.rs",
                    1,
                    "mock finding",
                    Severity::Warning,
                )],
                recommendations: vec![],
                metrics: None,
                processing_time_ms: 10.0,
                blocked_merge: false,
            })
        }
    }

    fn test_orchestrator() -> Orchestrator {
        let mut o = Orchestrator::new();
        o.register(Engine::Mgtg, Box::new(MockEngine::new("mgtg", Engine::Mgtg)));
        o.register(Engine::Dependency, Box::new(MockEngine::new("dependency", Engine::Dependency)));
        o.register(Engine::Process, Box::new(MockEngine::new("process", Engine::Process)));
        o
    }

    #[test]
    fn test_orchestrator_new() {
        let o = Orchestrator::new();
        assert!(o.engine_names().is_empty());
    }

    #[test]
    fn test_register_and_analyze_all() {
        let o = test_orchestrator();
        let req = AnalyzeRequest::new("/repo", "abc");
        let result = o.analyze(&req).unwrap();
        assert_eq!(result.findings.len(), 3);
        assert!((result.overall_risk - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_analyze_specific_engines() {
        let o = test_orchestrator();
        let req = AnalyzeRequest {
            engines: vec![Engine::Mgtg],
            ..AnalyzeRequest::new("/repo", "abc")
        };
        let result = o.analyze(&req).unwrap();
        assert_eq!(result.findings.len(), 1);
    }

    #[test]
    fn test_analyze_single_engine() {
        let o = test_orchestrator();
        let req = AnalyzeRequest::new("/repo", "abc");
        let result = o.analyze_single(&req, Engine::Mgtg).unwrap();
        assert_eq!(result.findings.len(), 1);
    }

    #[test]
    fn test_analyze_single_unregistered() {
        let o = Orchestrator::new();
        let req = AnalyzeRequest::new("/repo", "abc");
        let result = o.analyze_single(&req, Engine::Ml);
        assert!(result.is_err());
    }

    #[test]
    fn test_health_check() {
        let o = test_orchestrator();
        let health = o.health_check();
        assert_eq!(health.len(), 3);
    }

    #[test]
    fn test_engine_names() {
        let o = test_orchestrator();
        let names = o.engine_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"mgtg"));
    }

    #[test]
    fn test_analyze_all_empty_request() {
        let o = test_orchestrator();
        let req = AnalyzeRequest::new("", "abc");
        let result = o.analyze(&req).unwrap();
        assert_eq!(result.findings.len(), 3);
    }

    #[test]
    fn test_analyze_with_engine_failure() {
        let mut o = Orchestrator::new();
        o.register(Engine::Mgtg, Box::new(MockEngine::new("mgtg", Engine::Mgtg)));

        struct FailingEngine;
        impl AnalysisEngine for FailingEngine {
            fn name(&self) -> &'static str { "fail" }
            fn analyze(&self, _: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
                Err(sutra_common::error::SutraError::engine("fail", "broken"))
            }
        }
        o.register(Engine::Process, Box::new(FailingEngine));

        let req = AnalyzeRequest::new("/repo", "abc");
        let result = o.analyze(&req).unwrap();
        assert_eq!(result.findings.len(), 2); // 1 mock + 1 error finding
    }

    #[test]
    fn test_orchestrator_default() {
        let o = Orchestrator::default();
        assert!(o.engine_names().is_empty());
    }

    #[test]
    fn test_analyze_empty_registry() {
        let o = Orchestrator::new();
        let req = AnalyzeRequest::new("/repo", "abc");
        let result = o.analyze(&req).unwrap();
        assert!(result.findings.is_empty());
        assert!((result.overall_risk - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_analyze_with_panicking_engine() {
        struct PanicEngine;
        impl AnalysisEngine for PanicEngine {
            fn name(&self) -> &'static str { "panic" }
            fn analyze(&self, _: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
                panic!("engine panic!");
            }
        }

        let mut o = Orchestrator::new();
        o.register(Engine::Mgtg, Box::new(MockEngine::new("mgtg", Engine::Mgtg)));
        o.register(Engine::Process, Box::new(PanicEngine));

        let req = AnalyzeRequest::new("/repo", "abc");
        let result = o.analyze(&req).unwrap();
        assert_eq!(result.findings.len(), 2);
        assert!(result.findings.iter().any(|f| f.id.starts_with("ORCH-")));
    }

    #[test]
    fn test_analyze_with_nan_risk() {
        struct NanEngine;
        impl AnalysisEngine for NanEngine {
            fn name(&self) -> &'static str { "nan" }
            fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
                Ok(AnalysisResult {
                    overall_risk: f64::NAN,
                    ..AnalysisResult::new(&request.request_id, &request.commit_hash)
                })
            }
        }

        let mut o = Orchestrator::new();
        o.register(Engine::Mgtg, Box::new(NanEngine));

        let req = AnalyzeRequest::new("/repo", "abc");
        let result = o.analyze(&req).unwrap();
        assert!((result.overall_risk - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_register_same_engine_twice() {
        let mut o = Orchestrator::new();
        o.register(Engine::Mgtg, Box::new(MockEngine::new("first", Engine::Mgtg)));
        o.register(Engine::Mgtg, Box::new(MockEngine::new("second", Engine::Mgtg)));
        assert_eq!(o.engine_names().len(), 1);
        assert_eq!(o.engine_names()[0], "second");
    }

    #[test]
    fn test_health_check_empty() {
        let o = Orchestrator::new();
        let health = o.health_check();
        assert!(health.is_empty());
    }
}
