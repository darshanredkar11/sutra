use sutra_common::engine::AnalysisEngine;
use sutra_common::error::SutraResult;
use sutra_schema::v1::{
    AnalysisResult, AnalyzeRequest, Engine, Finding, MetricsSummary, Recommendation, Severity,
};

use crate::complexity;
use crate::queueing;
use crate::runtime;
use crate::scoring;
use crate::types::{RseConfig, Runtime, SurvivabilityScore};
use crate::weight;

pub struct RseEngine {
    config: RseConfig,
}

impl RseEngine {
    pub fn new() -> Self {
        Self {
            config: RseConfig::default(),
        }
    }

    pub fn with_config(mut self, config: RseConfig) -> Self {
        self.config = config;
        self
    }

    fn analyze_endpoints(&self, repo_path: &str) -> Vec<(String, String, Runtime, String)> {
        let mut endpoints = Vec::new();
        let mut files: Vec<String> = Vec::new();

        match std::fs::read_dir(repo_path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Ok(sub) = std::fs::read_dir(&path) {
                            for sub_entry in sub.flatten() {
                                let sub_path = sub_entry.path();
                                if sub_path.is_file() {
                                    if let Some(ext) = sub_path.extension().and_then(|e| e.to_str()) {
                                        let supported = ["java", "kt", "kts", "py", "js", "ts", "mjs", "mts", "rs", "go"];
                                        if supported.contains(&ext) {
                                            files.push(sub_path.to_string_lossy().to_string());
                                        }
                                    }
                                }
                            }
                        }
                    } else if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            let supported = ["java", "kt", "kts", "py", "js", "ts", "mjs", "mts", "rs", "go"];
                            if supported.contains(&ext) {
                                files.push(path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
            Err(_) => return endpoints,
        };

        if files.is_empty() {
            return endpoints;
        }

        let runtime = self.config.detect_runtime(&files);

        for file_path in &files {
            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let ext = file_path.rsplit('.').next().unwrap_or("rs");

            let detected = complexity::detect_endpoints(&content, ext);
            for (path, method) in &detected {
                endpoints.push((path.clone(), method.clone(), runtime, file_path.clone()));
                if endpoints.len() >= self.config.max_endpoints {
                    return endpoints;
                }
            }
        }

        if endpoints.is_empty() {
            endpoints.push(("/api/unknown".into(), "POST".into(), runtime, "N/A".into()));
        }

        endpoints
    }

    fn analyze_endpoint(
        &self,
        _path: &str,
        _method: &str,
        runtime: Runtime,
        source: &str,
        file_ext: &str,
    ) -> Result<SurvivabilityScore, String> {
        let weight = weight::estimate_weight_from_file_length(source.len());
        let complexity = complexity::analyze_source_code(source, file_ext);
        let queueing = queueing::compute_queueing(
            runtime,
            &complexity,
            self.config.expected_rps,
            weight.total_bytes,
        );
        let components = runtime::predict_runtime(
            runtime,
            &weight,
            &complexity,
            self.config.expected_rps,
            self.config.memory_limit_mb,
        );
        let score = scoring::compute_survivability(components, queueing, complexity, weight);
        Ok(score)
    }

    fn generate_findings(
        &self,
        endpoint: &str,
        method: &str,
        score: &SurvivabilityScore,
        file_path: &str,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        findings.push(Finding::new(
            "RSE-SURV",
            Engine::RuntimeSurvivability,
            file_path,
            1,
            &format!(
                "{} {} survivability: {:.2} — {}",
                method,
                endpoint,
                score.value,
                if score.value >= 0.8 {
                    "Healthy"
                } else if score.value >= 0.6 {
                    "Warning"
                } else if score.value >= 0.3 {
                    "Critical risk of failure"
                } else {
                    "Guaranteed failure under load"
                }
            ),
            score.severity(),
        ));

        if score.components.cpu_risk > 0.6 {
            findings.push(Finding::new(
                "RSE-CPU",
                Engine::RuntimeSurvivability,
                file_path,
                1,
                &format!(
                    "{} {} CPU saturation risk: {:.2} — complexity {}",
                    method,
                    endpoint,
                    score.components.cpu_risk,
                    score.complexity.time_complexity.label()
                ),
                Severity::Warning,
            ).with_fix("Refactor hot paths. Reduce loop nesting. Add caching."));
        }

        if score.components.memory_risk > 0.6 {
            findings.push(Finding::new(
                "RSE-MEM",
                Engine::RuntimeSurvivability,
                file_path,
                1,
                &format!(
                    "{} {} memory exhaustion risk: {:.2} — {:.1}KB/req",
                    method,
                    endpoint,
                    score.components.memory_risk,
                    score.weight.total_bytes / 1024.0
                ),
                Severity::Warning,
            ).with_fix("Reduce payload size. Implement pagination. Stream large payloads."));
        }

        if score.components.gc_risk > 0.6 {
            findings.push(Finding::new(
                "RSE-GC",
                Engine::RuntimeSurvivability,
                file_path,
                1,
                &format!(
                    "{} {} GC pressure risk: {:.2} — {} allocations detected",
                    method,
                    endpoint,
                    score.components.gc_risk,
                    score.complexity.allocation_count
                ),
                Severity::Warning,
            ).with_fix("Reduce object allocation. Pool/reuse objects. Use value types."));
        }

        if score.components.thread_risk > 0.6 {
            findings.push(Finding::new(
                "RSE-THREAD",
                Engine::RuntimeSurvivability,
                file_path,
                1,
                &format!(
                    "{} {} thread starvation risk: {:.2} — {} expected RPS",
                    method,
                    endpoint,
                    score.components.thread_risk,
                    self.config.expected_rps
                ),
                Severity::Error,
            ).with_fix("Increase thread pool. Add async processing. Scale horizontally."));
        }

        if score.components.latency_risk > 0.6 {
            findings.push(Finding::new(
                "RSE-LATENCY",
                Engine::RuntimeSurvivability,
                file_path,
                1,
                &format!(
                    "{} {} latency growth risk: {:.2} — estimated {:.0}ms",
                    method,
                    endpoint,
                    score.components.latency_risk,
                    score.queueing.response_time_ms
                ),
                Severity::Warning,
            ).with_fix("Add caching. Optimize queries. Reduce blocking operations."));
        }

        if score.queueing.utilization >= 0.8 {
            findings.push(Finding::new(
                "RSE-QUEUE",
                Engine::RuntimeSurvivability,
                file_path,
                1,
                &format!(
                    "{} {} queue saturation: utilization {:.0}% — safe RPS {:.0}, expected {}",
                    method,
                    endpoint,
                    score.queueing.utilization * 100.0,
                    score.queueing.safe_rps,
                    self.config.expected_rps
                ),
                if score.queueing.utilization >= 1.0 {
                    Severity::Critical
                } else {
                    Severity::Error
                },
            ).with_fix("Scale horizontally. Reduce request rate. Optimize endpoint."));
        }

        findings
    }

    fn generate_recommendations(&self, scores: &[SurvivabilityScore]) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        let worst = scores.iter().min_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal));

        if let Some(w) = worst {
            if w.value < 0.6 {
                recs.push(Recommendation::new(
                    &format!(
                        "Critical endpoint has survivability {:.2}. {}",
                        w.value,
                        if w.components.cpu_risk > 0.6 {
                            "Refactor nested iterations and reduce computational complexity."
                        } else if w.components.memory_risk > 0.6 {
                            "Reduce payload sizes and implement pagination."
                        } else if w.components.gc_risk > 0.6 {
                            "Reduce object allocation rates and pool resources."
                        } else if w.queueing.utilization >= 0.8 {
                            "Scale horizontally to handle expected request rate."
                        } else {
                            "Review endpoint architecture and resource configuration."
                        }
                    ),
                    (1.0 - w.value).min(1.0),
                ));
            }
        }

        let avg_survivability = if scores.is_empty() {
            0.0
        } else {
            scores.iter().map(|s| s.value).sum::<f64>() / scores.len() as f64
        };

        if avg_survivability < 0.8 && !scores.is_empty() {
            let max_rps = scores
                .iter()
                .map(|s| scoring::max_safe_rps(s))
                .fold(f64::MAX, f64::min);
            recs.push(Recommendation::new(
                &format!(
                    "Average survivability is {:.2}. Consider reducing expected RPS to {:.0} or scaling infrastructure.",
                    avg_survivability, max_rps
                ),
                0.7,
            ));
        }

        recs
    }
}

impl Default for RseEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisEngine for RseEngine {
    fn name(&self) -> &'static str {
        "rse"
    }

    fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
        if !self.config.enabled {
            return Ok(AnalysisResult::new(&request.request_id, &request.commit_hash));
        }

        let start = std::time::Instant::now();

        let endpoints = self.analyze_endpoints(&request.repo_path);
        if endpoints.is_empty() {
            return Ok(AnalysisResult {
                overall_risk: 0.0,
                findings: vec![Finding::new(
                    "RSE-NO-SRC",
                    Engine::RuntimeSurvivability,
                    "N/A",
                    1,
                    "No supported source files found for analysis",
                    Severity::Info,
                )],
                ..AnalysisResult::new(&request.request_id, &request.commit_hash)
            });
        }

        let mut all_findings = Vec::new();
        let mut scores = Vec::new();

        for (endpoint_path, method, runtime, file_path) in &endpoints {
            let source = match std::fs::read_to_string(file_path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let ext = file_path.rsplit('.').next().unwrap_or("rs");
            let ext = if ext == file_path { "rs" } else { ext };

            match self.analyze_endpoint(endpoint_path, method, *runtime, &source, ext) {
                Ok(score) => {
                    scores.push(score.clone());
                    let ep_findings = self.generate_findings(
                        endpoint_path,
                        method,
                        &score,
                        file_path,
                    );
                    all_findings.extend(ep_findings);
                }
                Err(e) => {
                    all_findings.push(Finding::new(
                        "RSE-ERR",
                        Engine::RuntimeSurvivability,
                        file_path,
                        1,
                        &format!("Failed to analyze endpoint {} {}: {}", method, endpoint_path, e),
                        Severity::Warning,
                    ));
                }
            }
        }

        let recommendations = self.generate_recommendations(&scores);

        let worst_score = scores.iter().min_by(|a, b| {
            a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal)
        });
        let overall_risk = worst_score.map(|s| s.overall_risk()).unwrap_or(0.0);
        let blocked = worst_score.map(|s| s.blocked()).unwrap_or(false);

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        let worst_complexity = scores.iter().map(|s| s.complexity.loop_depth).max().unwrap_or(0) as f64;
        let worst_memory = scores.iter().map(|s| s.weight.total_bytes).fold(0.0f64, f64::max);
        let worst_safe_rps = scores.iter().map(|s| scoring::max_safe_rps(s)).fold(f64::MAX, f64::min);

        Ok(AnalysisResult {
            request_id: request.request_id.clone(),
            commit_hash: request.commit_hash.clone(),
            overall_risk,
            findings: all_findings,
            recommendations,
            metrics: Some(MetricsSummary {
                rse_survivability: worst_score.map(|s| s.value).unwrap_or(0.0),
                rse_complexity_max: worst_complexity,
                rse_memory_per_request: worst_memory,
                rse_safe_rps: if worst_safe_rps == f64::MAX { 0.0 } else { worst_safe_rps },
                ..MetricsSummary::default()
            }),
            processing_time_ms: elapsed,
            blocked_merge: blocked,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RseConfig;

    fn healthy_score() -> SurvivabilityScore {
        SurvivabilityScore {
            value: 0.95,
            components: crate::types::RuntimePrediction {
                cpu_risk: 0.1,
                memory_risk: 0.1,
                gc_risk: 0.1,
                thread_risk: 0.1,
                latency_risk: 0.1,
            },
            queueing: crate::types::QueueingMetrics {
                arrival_rate: 10.0,
                service_rate: 1000.0,
                utilization: 0.01,
                active_requests: 0.01,
                response_time_ms: 1.0,
                safe_rps: 600.0,
            },
            complexity: crate::types::ComplexityProfile {
                time_complexity: crate::types::ComplexityClass::O1,
                loop_depth: 0,
                allocation_count: 0,
                branch_count: 0,
                function_count: 1,
            },
            weight: crate::types::RequestWeight {
                raw_bytes: 64.0,
                expansion_factor: 1.5,
                runtime_bytes: 96.0,
                temp_allocations: 0.0,
                total_bytes: 96.0,
            },
        }
    }

    fn failing_score() -> SurvivabilityScore {
        SurvivabilityScore {
            value: 0.15,
            components: crate::types::RuntimePrediction {
                cpu_risk: 0.9,
                memory_risk: 0.8,
                gc_risk: 0.7,
                thread_risk: 0.7,
                latency_risk: 0.85,
            },
            queueing: crate::types::QueueingMetrics {
                arrival_rate: 10000.0,
                service_rate: 100.0,
                utilization: 0.95,
                active_requests: 19.0,
                response_time_ms: 30000.0,
                safe_rps: 60.0,
            },
            complexity: crate::types::ComplexityProfile {
                time_complexity: crate::types::ComplexityClass::ON3,
                loop_depth: 8,
                allocation_count: 50,
                branch_count: 30,
                function_count: 10,
            },
            weight: crate::types::RequestWeight {
                raw_bytes: 102400.0,
                expansion_factor: 4.0,
                runtime_bytes: 409600.0,
                temp_allocations: 1024.0,
                total_bytes: 410624.0,
            },
        }
    }

    fn score_with(
        cpu: f64, mem: f64, gc: f64, thread: f64, latency: f64,
        util: f64
    ) -> SurvivabilityScore {
        let mut s = healthy_score();
        s.components.cpu_risk = cpu;
        s.components.memory_risk = mem;
        s.components.gc_risk = gc;
        s.components.thread_risk = thread;
        s.components.latency_risk = latency;
        s.queueing.utilization = util;
        // ensure overall survivability tracks worst component
        let worst = cpu.max(mem).max(gc).max(thread).max(latency);
        s.value = (1.0 - worst).max(0.0);
        s
    }

    #[test]
    fn test_engine_name() {
        let engine = RseEngine::new();
        assert_eq!(engine.name(), "rse");
    }

    #[test]
    fn test_engine_disabled() {
        let config = RseConfig {
            enabled: false,
            ..RseConfig::default()
        };
        let engine = RseEngine::new().with_config(config);
        let req = AnalyzeRequest::new("/nonexistent", "abc");
        let result = engine.analyze(&req).unwrap();
        assert!(result.findings.is_empty());
        assert!((result.overall_risk - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_engine_no_source() {
        let engine = RseEngine::new();
        let req = AnalyzeRequest::new("/nonexistent/path", "abc");
        let result = engine.analyze(&req).unwrap();
        assert!(result.findings.is_empty() || result.findings[0].id == "RSE-NO-SRC");
    }

    #[test]
    fn test_rse_config_default_detects_runtime() {
        let config = RseConfig::default();
        assert_eq!(config.detect_runtime(&[]), Runtime::Rust);
        assert_eq!(config.detect_runtime(&["file.java".into()]), Runtime::Jvm);
        assert_eq!(config.detect_runtime(&["file.py".into()]), Runtime::Python);
        assert_eq!(config.detect_runtime(&["file.js".into()]), Runtime::NodeJs);
        assert_eq!(config.detect_runtime(&["file.ts".into()]), Runtime::NodeJs);
        assert_eq!(config.detect_runtime(&["file.rs".into()]), Runtime::Rust);
        assert_eq!(config.detect_runtime(&["file.go".into()]), Runtime::Go);
    }

    #[test]
    fn test_rse_config_explicit_runtime_overrides_detection() {
        let config = RseConfig {
            runtime: Some("python".into()),
            ..RseConfig::default()
        };
        assert_eq!(config.detect_runtime(&["main.rs".into()]), Runtime::Python);
    }

    #[test]
    fn test_rse_config_invalid_runtime_falls_back_to_rust() {
        let config = RseConfig {
            runtime: Some("brainfuck".into()),
            ..RseConfig::default()
        };
        assert_eq!(config.detect_runtime(&[]), Runtime::Rust);
    }

    #[test]
    fn test_generate_findings_healthy_only_surv() {
        let engine = RseEngine::new();
        let score = healthy_score();
        let findings = engine.generate_findings("/health", "GET", &score, "src/main.rs");
        let ids: Vec<&str> = findings.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, vec!["RSE-SURV"]);
    }

    #[test]
    fn test_generate_findings_cpu_risk() {
        let engine = RseEngine::new();
        let score = score_with(0.7, 0.1, 0.1, 0.1, 0.1, 0.01);
        let findings = engine.generate_findings("/cpu", "GET", &score, "src/hot.rs");
        let ids: Vec<&str> = findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"RSE-CPU"));
        assert!(findings.iter().any(|f| f.suggested_fix.is_some()));
    }

    #[test]
    fn test_generate_findings_memory_risk() {
        let engine = RseEngine::new();
        let score = score_with(0.1, 0.7, 0.1, 0.1, 0.1, 0.01);
        let findings = engine.generate_findings("/mem", "POST", &score, "src/big.rs");
        let ids: Vec<&str> = findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"RSE-MEM"));
    }

    #[test]
    fn test_generate_findings_gc_risk() {
        let engine = RseEngine::new();
        let score = score_with(0.1, 0.1, 0.7, 0.1, 0.1, 0.01);
        let findings = engine.generate_findings("/gc", "PUT", &score, "src/alloc.rs");
        let ids: Vec<&str> = findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"RSE-GC"));
    }

    #[test]
    fn test_generate_findings_thread_risk() {
        let engine = RseEngine::new();
        let score = score_with(0.1, 0.1, 0.1, 0.7, 0.1, 0.01);
        let findings = engine.generate_findings("/thread", "DELETE", &score, "src/conc.rs");
        let ids: Vec<&str> = findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"RSE-THREAD"));
    }

    #[test]
    fn test_generate_findings_latency_risk() {
        let engine = RseEngine::new();
        let score = score_with(0.1, 0.1, 0.1, 0.1, 0.7, 0.01);
        let findings = engine.generate_findings("/slow", "GET", &score, "src/slow.rs");
        let ids: Vec<&str> = findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"RSE-LATENCY"));
    }

    #[test]
    fn test_generate_findings_queue_saturation() {
        let engine = RseEngine::new();
        let score = score_with(0.1, 0.1, 0.1, 0.1, 0.1, 0.85);
        let findings = engine.generate_findings("/queue", "GET", &score, "src/queue.rs");
        let ids: Vec<&str> = findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"RSE-QUEUE"));
    }

    #[test]
    fn test_generate_findings_queue_critical_at_or_above_1() {
        let engine = RseEngine::new();
        let score = score_with(0.1, 0.1, 0.1, 0.1, 0.1, 1.0);
        let findings = engine.generate_findings("/overload", "GET", &score, "src/crash.rs");
        let queue = findings.iter().find(|f| f.id == "RSE-QUEUE").unwrap();
        assert_eq!(queue.severity, Severity::Critical);
    }

    #[test]
    fn test_generate_findings_all_at_once() {
        let engine = RseEngine::new();
        let score = failing_score();
        let findings = engine.generate_findings("/all", "POST", &score, "src/all.rs");
        let ids: Vec<&str> = findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"RSE-SURV"));
        assert!(ids.contains(&"RSE-CPU"));
        assert!(ids.contains(&"RSE-MEM"));
        assert!(ids.contains(&"RSE-GC"));
        assert!(ids.contains(&"RSE-THREAD"));
        assert!(ids.contains(&"RSE-LATENCY"));
        assert!(ids.contains(&"RSE-QUEUE"));
    }

    #[test]
    fn test_generate_recommendations_empty() {
        let engine = RseEngine::new();
        let recs = engine.generate_recommendations(&[]);
        assert!(recs.is_empty());
    }

    #[test]
    fn test_generate_recommendations_healthy_no_recs() {
        let engine = RseEngine::new();
        let recs = engine.generate_recommendations(&[healthy_score()]);
        assert!(recs.is_empty());
    }

    #[test]
    fn test_generate_recommendations_failing() {
        let engine = RseEngine::new();
        let recs = engine.generate_recommendations(&[failing_score()]);
        assert!(!recs.is_empty());
        assert!(recs[0].priority <= 1.0);
    }

    #[test]
    fn test_generate_recommendations_mixed_scores() {
        let engine = RseEngine::new();
        let recs = engine.generate_recommendations(&[healthy_score(), failing_score()]);
        assert!(!recs.is_empty());
    }

    #[test]
    fn test_engine_default() {
        let engine = RseEngine::default();
        assert_eq!(engine.name(), "rse");
        assert!(engine.config.enabled);
    }
}
