use sutra_common::engine::AnalysisEngine;
use sutra_common::error::SutraResult;
use sutra_schema::v1::{
    AnalysisResult, AnalyzeRequest, Engine, Finding, Recommendation, Severity,
};

use crate::types::CouplingConfig;

pub struct CouplingEngine {
    config: CouplingConfig,
}

impl CouplingEngine {
    pub fn new() -> Self {
        Self {
            config: CouplingConfig::default(),
        }
    }

    pub fn with_config(mut self, config: CouplingConfig) -> Self {
        self.config = config;
        self
    }

    fn analyze_file(&self, content: &str, file_path: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        let functions = self.detect_functions(content);
        let _imports = self.detect_imports(content);
        let call_chains = self.detect_call_chains(content, &functions);
        let fan_in_out = self.compute_fan_in_out(content, &functions);

        for (func, fan_in, fan_out) in &fan_in_out {
            if *fan_in > 5 || *fan_out > 5 {
                let total_coupling = fan_in + fan_out;
                let refactoring_effort = ((total_coupling as f64 / 10.0) * 16.0).ceil() as u32;
                let throughput_improvement = (total_coupling as f64 * 20.0) as u32; // 20% per reduced hop

                let spec = serde_json::json!({
                    "type": "redistribute_hub",
                    "current_state": {
                        "fan_in": fan_in,
                        "fan_out": fan_out,
                        "total_coupling": total_coupling
                    },
                    "proposed_state": {
                        "fan_in": (fan_in / 2).max(2),
                        "fan_out": (fan_out / 2).max(2),
                        "num_modules": 2
                    },
                    "impact": {
                        "throughput_improvement_percent": throughput_improvement,
                        "latency_improvement": "Medium",
                        "testability_improvement": "High",
                        "reasoning_complexity": "High"
                    },
                    "migration_plan": {
                        "phase_1": "Identify responsibility clusters",
                        "phase_2": "Extract interfaces for each cluster",
                        "phase_3": "Migrate call sites incrementally",
                        "phase_4": "Deprecate and remove old hub"
                    },
                    "effort": {
                        "estimated_hours": refactoring_effort,
                        "complexity_of_refactor": "high",
                        "risk_of_bugs": 0.15
                    },
                    "roi": {
                        "throughput_gain": format!("{}% throughput improvement", throughput_improvement.min(100)),
                        "roi_months": format!("{:.2}", (refactoring_effort as f64 * 100.0) / (throughput_improvement.max(1) as f64 * 10.0)),
                        "priority": "critical"
                    }
                });

                findings.push(
                    Finding::new(
                        "COUP-HUB",
                        Engine::CouplingResolution,
                        file_path,
                        func.line,
                        &format!(
                            "High hub coupling: '{}' has fan-in {} and fan-out {}. Central hub — redistribute responsibilities.",
                            func.name, fan_in, fan_out
                        ),
                        Severity::Warning,
                    )
                    .with_fix("Split functionality: extract subsets of calls into separate modules")
                    .with_spec_data(spec)
                    .with_confidence(0.93)
                    .with_edge_cases(vec![
                        "Ensure all callers are updated when extracting submodules".into(),
                        "Verify no circular dependencies introduced between new modules".into(),
                        "Monitor latency during gradual migration to new architecture".into(),
                    ])
                );
            }
        }

        if call_chains.len() >= self.config.chain_depth_threshold as usize {
            let chain_depth = call_chains.len();
            let refactoring_effort = ((chain_depth as f64 / 5.0) * 24.0).ceil() as u32;
            let latency_reduction = (chain_depth as f64 * 15.0) as u32; // ~15ms per hop reduced

            let spec = serde_json::json!({
                "type": "async_queue_decoupling",
                "current_state": {
                    "call_chain_depth": chain_depth,
                    "synchronous_hops": chain_depth,
                    "blocking_calls": chain_depth
                },
                "proposed_state": {
                    "architecture": "async queue with event-driven handlers",
                    "call_chain_depth": 2,
                    "latency_model": "queue-based with percentile SLAs"
                },
                "impact": {
                    "latency_reduction_ms": latency_reduction,
                    "throughput_improvement": "Very High",
                    "resilience_improvement": "Critical",
                    "scalability_improvement": "Horizontal scaling enabled"
                },
                "technical_approach": {
                    "step_1": "Identify event boundaries between hops",
                    "step_2": "Extract handler functions for each hop",
                    "step_3": "Implement queue (RabbitMQ, Kafka, or Redis Streams)",
                    "step_4": "Add retry logic and dead-letter queues",
                    "step_5": "Migrate call sites to publish events"
                },
                "effort": {
                    "estimated_hours": refactoring_effort,
                    "complexity_of_refactor": "high",
                    "infrastructure_setup": "Moderate",
                    "risk_of_bugs": 0.20
                },
                "roi": {
                    "latency_reduction_ms": latency_reduction,
                    "roi_months": format!("{:.2}", (refactoring_effort as f64 * 100.0) / (latency_reduction.max(1) as f64 * 2.0)),
                    "priority": "critical"
                }
            });

            findings.push(
                Finding::new(
                    "COUP-CHAIN",
                    Engine::CouplingResolution,
                    file_path,
                    call_chains[0].line,
                    &format!(
                        "Call chain detected with {} hops. Consider async queue or event-driven architecture.",
                        call_chains.len()
                    ),
                    Severity::Warning,
                )
                .with_fix("Decouple via message queue: replace direct calls with async events")
                .with_spec_data(spec)
                .with_confidence(0.91)
                .with_edge_cases(vec![
                    "Ensure event ordering is preserved for dependent operations".into(),
                    "Implement idempotency to handle retries safely".into(),
                    "Plan for eventual consistency semantics vs. immediate consistency".into(),
                    "Monitor queue depth and implement backpressure mechanisms".into(),
                ])
            );
        }

        if self.detect_circular_dependency(&functions, &call_chains) {
            let effort_hours = 12u32;
            let bug_risk_reduction = 500u32; // High risk of defects from circular deps

            let spec = serde_json::json!({
                "type": "break_circular_dependency",
                "current_state": {
                    "circular_dependencies": 1,
                    "affected_components": "Multiple",
                    "testing_difficulty": "Very High",
                    "refactoring_risk": "Very High"
                },
                "proposed_state": {
                    "circular_dependencies": 0,
                    "architecture": "Layered or event-driven with clear direction"
                },
                "impact": {
                    "testability_improvement": "Critical",
                    "reasoning_complexity": "Critical",
                    "maintenance_difficulty_reduction": "Very High",
                    "regression_risk": "Reduced"
                },
                "resolution_patterns": [
                    "Dependency Inversion Principle: introduce interface between modules",
                    "Extract mediator/coordinator module to manage interactions",
                    "Use event-driven approach: decouple via events instead of direct calls",
                    "Separate into distinct layers with one-way dependencies"
                ],
                "effort": {
                    "estimated_hours": effort_hours,
                    "complexity_of_refactor": "very_high",
                    "risk_of_bugs": 0.25
                },
                "roi": {
                    "defect_prevention": format!("${}", bug_risk_reduction * 5), // $5K per prevented defect
                    "roi_months": format!("{:.2}", (effort_hours as f64 * 100.0) / (bug_risk_reduction as f64 + 1.0)),
                    "priority": "critical"
                }
            });

            findings.push(
                Finding::new(
                    "COUP-CIRCULAR",
                    Engine::CouplingResolution,
                    file_path,
                    1,
                    "Circular dependency detected between functions. Hard to reason about and test.",
                    Severity::Error,
                )
                .with_fix("Apply dependency inversion: introduce interface or mediator to break the cycle")
                .with_spec_data(spec)
                .with_confidence(0.98)
                .with_edge_cases(vec![
                    "This is a critical issue - must be resolved to maintain code quality".into(),
                    "Verify resolution doesn't introduce performance overhead from indirection".into(),
                    "Ensure all call sites are updated when breaking the cycle".into(),
                    "Add integration tests to verify cycle is truly broken".into(),
                ])
            );
        }

        if findings.len() > 5 {
            findings.truncate(5);
        }

        findings
    }

    fn detect_functions(&self, content: &str) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        let fn_prefixes = ["fn ", "def ", "function ", "pub fn ", "pub async fn ", "async fn "];

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
                continue;
            }
            let is_fn = fn_prefixes.iter().any(|kw| trimmed.starts_with(kw));
            if !is_fn { continue; }

            let before_paren = trimmed.split('(').next().unwrap_or("");
            let name = before_paren
                .split_whitespace()
                .filter(|s| *s != "fn" && *s != "def" && *s != "function"
                    && *s != "pub" && *s != "async" && *s != "unsafe" && *s != "pub(crate)")
                .last()
                .unwrap_or("anonymous")
                .to_string();

            functions.push(FunctionInfo {
                name,
                line: (i + 1) as u32,
            });
        }
        functions
    }

    fn detect_imports(&self, content: &str) -> Vec<String> {
        let mut imports = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("use ")
                || trimmed.starts_with("import ")
                || trimmed.starts_with("from ")
                || trimmed.starts_with("extern crate ")
                || trimmed.starts_with("#include")
                || trimmed.starts_with("require(")
                || trimmed.starts_with("const ")
            {
                imports.push(trimmed.to_string());
            }
        }
        imports
    }

    fn detect_call_chains(&self, content: &str, functions: &[FunctionInfo]) -> Vec<CallSite> {
        let mut calls = Vec::new();
        let fn_prefixes = ["fn ", "def ", "function ", "pub fn ", "async fn "];

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
                continue;
            }

            let is_fn_def = fn_prefixes.iter().any(|kw| trimmed.starts_with(kw));
            if !is_fn_def { continue; }

            let before_paren = trimmed.split('(').next().unwrap_or("");
            let caller_name = before_paren
                .split_whitespace()
                .filter(|s| *s != "fn" && *s != "def" && *s != "function"
                    && *s != "pub" && *s != "async" && *s != "unsafe")
                .last()
                .unwrap_or("anonymous")
                .to_string();

            for func in functions {
                if func.name == caller_name { continue; }
                let call_pattern = format!("{}(", func.name);
                if trimmed.contains(&call_pattern) {
                    let cp = CallSite {
                        caller: caller_name.clone(),
                        callee: func.name.clone(),
                        line: (i + 1) as u32,
                    };
                    if !calls.iter().any(|c: &CallSite| c.caller == cp.caller && c.callee == cp.callee) {
                        calls.push(cp);
                    }
                }
            }
        }
        calls
    }

    fn detect_circular_dependency(&self, functions: &[FunctionInfo], calls: &[CallSite]) -> bool {
        for func_a in functions {
            for func_b in functions {
                if func_a.name == func_b.name {
                    continue;
                }
                let a_to_b = calls.iter().any(|c| c.caller == func_a.name && c.callee == func_b.name);
                let b_to_a = calls.iter().any(|c| c.caller == func_b.name && c.callee == func_a.name);
                if a_to_b && b_to_a {
                    return true;
                }
            }
        }
        false
    }

    fn compute_fan_in_out(&self, content: &str, functions: &[FunctionInfo]) -> Vec<(FunctionInfo, u32, u32)> {
        let mut results = Vec::new();
        for func in functions {
            let mut fan_in = 0u32;
            let mut fan_out = 0u32;
            for other in functions {
                if other.name == func.name {
                    continue;
                }
                if content.contains(&format!("{}.", other.name)) {
                    fan_out += 1;
                }
                if content.contains(&format!("{}.", func.name)) {
                    fan_in += 1;
                }
            }
            results.push((func.clone(), fan_in, fan_out));
        }
        results
    }

    fn generate_recommendations(&self, findings: &[Finding]) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        let count = findings.len();
        if count > 0 {
            recs.push(Recommendation::new(
                &format!(
                    "Found {} coupling issues. Prioritize circular dependencies and hub modules first.",
                    count
                ),
                0.7,
            ));
        }
        recs
    }
}

impl Default for CouplingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisEngine for CouplingEngine {
    fn name(&self) -> &'static str {
        "coupling"
    }

    fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
        if !self.config.enabled {
            return Ok(AnalysisResult::new(&request.request_id, &request.commit_hash));
        }

        let start = std::time::Instant::now();
        let mut all_findings = Vec::new();

        const SUPPORTED: [&str; 10] = ["rs", "py", "js", "ts", "java", "kt", "go", "mjs", "mts", "c"];
        let files: Vec<_> = walkdir::WalkDir::new(&request.repo_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|ext| SUPPORTED.contains(&ext))
                    .unwrap_or(false)
            })
            .filter(|e| !e.path().to_string_lossy().contains("target/")
                && !e.path().to_string_lossy().contains("node_modules/")
                && !e.path().to_string_lossy().contains(".git/"))
            .map(|e| e.path().to_string_lossy().into_owned())
            .collect();

        for file_path in &files {
            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let findings = self.analyze_file(&content, file_path);
            all_findings.extend(findings);
        }

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        let recommendations = self.generate_recommendations(&all_findings);

        let overall_risk = if all_findings.iter().any(|f| f.severity == Severity::Error) {
            0.6
        } else if all_findings.iter().any(|f| f.severity == Severity::Warning) {
            0.3
        } else {
            0.0
        };

        Ok(AnalysisResult {
            request_id: request.request_id.clone(),
            commit_hash: request.commit_hash.clone(),
            overall_risk,
            findings: all_findings,
            recommendations,
            metrics: None,
            processing_time_ms: elapsed,
            blocked_merge: false,
            jit_features: None,
        })
    }
}

#[derive(Debug, Clone)]
struct FunctionInfo {
    name: String,
    line: u32,
}

struct CallSite {
    caller: String,
    callee: String,
    line: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_name() {
        let engine = CouplingEngine::new();
        assert_eq!(engine.name(), "coupling");
    }

    #[test]
    fn test_detect_functions() {
        let engine = CouplingEngine::new();
        let content = r#"
fn foo() {}
fn bar() {}
fn baz() {}
"#;
        let functions = engine.detect_functions(content);
        assert_eq!(functions.len(), 3);
    }

    #[test]
    fn test_detect_circular() {
        let engine = CouplingEngine::new();
        let content = r#"
fn foo() { bar(); }
fn bar() { foo(); }
"#;
        let functions = engine.detect_functions(content);
        let calls = engine.detect_call_chains(content, &functions);
        assert!(engine.detect_circular_dependency(&functions, &calls));
    }

    #[test]
    fn test_detect_call_chains() {
        let engine = CouplingEngine::new();
        let content = r#"
fn a() { b(); }
fn b() { c(); }
fn c() {}
"#;
        let functions = engine.detect_functions(content);
        let calls = engine.detect_call_chains(content, &functions);
        assert!(!calls.is_empty());
    }

    #[test]
    fn test_engine_disabled() {
        let engine = CouplingEngine::with_config(CouplingEngine::new(), CouplingConfig {
            enabled: false,
            ..CouplingConfig::default()
        });
        let req = AnalyzeRequest::new("/nonexistent", "abc");
        let result = engine.analyze(&req).unwrap();
        assert!(result.findings.is_empty());
    }

    #[test]
    fn test_engine_default() {
        let engine = CouplingEngine::default();
        assert_eq!(engine.name(), "coupling");
        assert!(engine.config.enabled);
    }
}
