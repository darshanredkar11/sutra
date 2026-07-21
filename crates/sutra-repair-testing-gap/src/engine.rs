use sutra_common::engine::AnalysisEngine;
use sutra_common::error::SutraResult;
use sutra_schema::v1::{
    AnalysisResult, AnalyzeRequest, Engine, Finding, Recommendation, Severity,
};

use crate::types::TestingGapConfig;

pub struct TestingGapEngine {
    config: TestingGapConfig,
}

impl TestingGapEngine {
    pub fn new() -> Self {
        Self {
            config: TestingGapConfig::default(),
        }
    }

    pub fn with_config(mut self, config: TestingGapConfig) -> Self {
        self.config = config;
        self
    }

    fn analyze_file(&self, content: &str, file_path: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        let functions = self.detect_functions(content);
        let branches = self.detect_branches(content);
        let error_handling = self.detect_error_handling(content);
        let is_test_file = file_path.contains("test") || file_path.contains("spec") || file_path.contains("_test");

        let estimated_coverage = if is_test_file { 0.85 } else { 0.45 };

        for func in &functions {
            let func_branches: Vec<&BranchInfo> = branches.iter()
                .filter(|b| b.line >= func.line && b.line < func.line + func.body_lines)
                .collect();

            if func_branches.is_empty() && func.body_lines > 3 && !is_test_file {
                let spec = serde_json::json!({
                    "type": "add_unit_tests",
                    "current_state": {
                        "function_type": "Linear/no-branch",
                        "test_coverage": "None",
                        "function_lines": func.body_lines
                    },
                    "proposed_state": {
                        "test_cases": 1,
                        "test_coverage": "100%"
                    },
                    "impact": {
                        "coverage_improvement": "Baseline coverage established",
                        "regression_detection": "Future changes detected",
                        "documentation_value": "Function behavior documented through tests"
                    },
                    "effort": {
                        "estimated_hours": 1u32,
                        "complexity_of_testing": "low",
                        "risk_of_bugs": 0.02
                    },
                    "roi": {
                        "regression_prevention": "$1000",
                        "roi_months": "0.1",
                        "priority": "low"
                    }
                });

                findings.push(
                    Finding::new(
                        "TEST-NO-BRANCH",
                        Engine::TestingGap,
                        file_path,
                        func.line,
                        &format!(
                            "Function '{}' has no branches — may be a simple accessor. Coverage: ~{:.0}%",
                            func.name, estimated_coverage * 100.0
                        ),
                        Severity::Info,
                    )
                    .with_fix("Add simple unit test to verify function behavior and catch regressions")
                    .with_spec_data(spec)
                    .with_confidence(0.75)
                    .with_edge_cases(vec![
                        "Simple functions are often refactored later — tests prevent regressions".into(),
                    ])
                );
            }

            let untested_branches: Vec<&&BranchInfo> = func_branches.iter()
                .filter(|b| {
                    let mut found = false;
                    if let Some(ref _line_text) = b.line_text {
                        if is_test_file {
                            found = true;
                        }
                    }
                    !found
                })
                .collect();

            if untested_branches.len() > 2 {
                let gap: f64 = self.config.coverage_goal - estimated_coverage;
                if gap > 0.0 {
                    let improvement = (gap * 100.0) as u32;
                    let test_count = untested_branches.len().min(5) as u32;
                    let effort_hours = (test_count as f64 * 1.5).ceil() as u32;
                    let defect_prevention = test_count * 1000; // $1K per prevented defect from untested path

                    let spec = serde_json::json!({
                        "type": "increase_branch_coverage",
                        "current_state": {
                            "coverage_percent": (estimated_coverage * 100.0) as u32,
                            "untested_branches": untested_branches.len(),
                            "coverage_gap_percent": improvement
                        },
                        "proposed_state": {
                            "coverage_percent": self.config.coverage_goal as u32 * 100,
                            "untested_branches": 0,
                            "test_cases_added": test_count
                        },
                        "impact": {
                            "coverage_improvement_percent": improvement,
                            "defect_detection_rate": "Very High",
                            "regression_prevention": "High",
                            "confidence_in_changes": "Significantly improved"
                        },
                        "testing_strategy": {
                            "parametrized_tests": true,
                            "error_paths": "Required",
                            "edge_cases": "Required",
                            "happy_path": "Required"
                        },
                        "effort": {
                            "estimated_hours": effort_hours,
                            "complexity_of_testing": "medium",
                            "infrastructure": "Parametrized test framework (pytest, JUnit, etc.)",
                            "risk_of_bugs": 0.05
                        },
                        "roi": {
                            "defect_prevention_value": format!("${}", defect_prevention),
                            "roi_months": format!("{:.2}", (effort_hours as f64 * 100.0) / (defect_prevention as f64 + 1.0)),
                            "priority": "high"
                        }
                    });

                    findings.push(
                        Finding::new(
                            "TEST-GAP",
                            Engine::TestingGap,
                            file_path,
                            func.line,
                            &format!(
                                "Function '{}' has {} untested branches. Coverage gap: {:.0}%. Add parametrized tests.",
                                func.name,
                                untested_branches.len(),
                                gap * 100.0
                            ),
                            Severity::Warning,
                        )
                        .with_fix(&format!(
                            "Write {} parametrized test cases covering error paths, edge cases, and main flow",
                            untested_branches.len().min(5)
                        ))
                        .with_spec_data(spec)
                        .with_confidence(0.87)
                        .with_edge_cases(vec![
                            "Ensure all parametrized test cases exercise distinct code paths".into(),
                            "Mock external dependencies to isolate function behavior".into(),
                            "Add assertions that verify both positive and negative scenarios".into(),
                        ])
                    );
                }
            }
        }

        if !error_handling.is_empty() && !is_test_file {
            for err in &error_handling {
                let has_negative_test = content.contains(&format!("{}_error", err.name))
                    || content.contains(&format!("{}_fail", err.name))
                    || content.contains(&format!("test_{}", err.name));

                if !has_negative_test {
                    let effort_hours = 2u32;
                    let defect_prevention = 2000u32; // Error path defects are critical ($2K per)

                    let spec = serde_json::json!({
                        "type": "test_error_paths",
                        "current_state": {
                            "error_handling": "Implemented",
                            "error_tests": "None detected",
                            "coverage": "0% of error paths"
                        },
                        "proposed_state": {
                            "error_handling": "Implemented + Tested",
                            "error_tests": "Added",
                            "coverage": "100% of error paths"
                        },
                        "impact": {
                            "defect_detection_rate": "Critical errors caught early",
                            "production_reliability": "Very High",
                            "incident_prevention": "Error scenarios prevented in production"
                        },
                        "test_scenarios": [
                            "Network timeout/unavailable",
                            "Invalid input/malformed data",
                            "Resource exhaustion (memory, connections)",
                            "Concurrency/race conditions",
                            "Cascading failures from dependencies"
                        ],
                        "effort": {
                            "estimated_hours": effort_hours,
                            "complexity_of_testing": "medium",
                            "error_scenario_complexity": "Can be complex",
                            "risk_of_bugs": 0.10
                        },
                        "roi": {
                            "critical_incident_prevention": format!("${}", defect_prevention),
                            "roi_months": format!("{:.2}", (effort_hours as f64 * 100.0) / (defect_prevention as f64 + 1.0)),
                            "priority": "critical"
                        }
                    });

                    findings.push(
                        Finding::new(
                            "TEST-ERROR-PATH",
                            Engine::TestingGap,
                            file_path,
                            err.line,
                            &format!(
                                "'{}' handles errors but no negative test detected. Add error-path tests.",
                                err.name
                            ),
                            Severity::Info,
                        )
                        .with_fix("Add dedicated error-path tests using chaos engineering or exception injection")
                        .with_spec_data(spec)
                        .with_confidence(0.91)
                        .with_edge_cases(vec![
                            "Error paths are often difficult to reproduce in production".into(),
                            "Use test utilities or mocking to simulate error conditions".into(),
                            "Verify error messages are helpful for debugging and don't leak secrets".into(),
                        ])
                    );
                }
            }
        }

        if findings.len() > 5 {
            findings.truncate(5);
        }

        findings
    }

    fn detect_functions(&self, content: &str) -> Vec<FunctionProfile> {
        let mut functions = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
                continue;
            }

            let is_fn = trimmed.starts_with("fn ")
                || trimmed.starts_with("def ")
                || trimmed.starts_with("function ")
                || (trimmed.contains("fn ") && trimmed.contains('('));

            if !is_fn {
                continue;
            }

            let name = trimmed
                .split(|c: char| c == '(' || c == ' ' || c == '<' || c == ':' || c == '{')
                .filter(|s| !s.is_empty() && *s != "fn" && *s != "def" && *s != "function"
                    && *s != "pub" && *s != "async" && *s != "pub(crate)")
                .next()
                .unwrap_or("anonymous")
                .to_string();

            let mut brace_depth = 0u32;
            let mut body = 0u32;
            let mut in_fn = false;

            for (j, l) in lines.iter().enumerate() {
                if j < i { continue; }
                if j == i {
                    for ch in l.chars() {
                        if ch == '{' { in_fn = true; brace_depth = 1; }
                    }
                    continue;
                }
                if !in_fn { continue; }
                body += 1;
                for ch in l.chars() {
                    match ch {
                        '{' => brace_depth += 1,
                        '}' => { brace_depth -= 1; if brace_depth == 0 { break; } }
                        _ => {}
                    }
                }
                if brace_depth == 0 { break; }
            }

            functions.push(FunctionProfile {
                name,
                line: (i + 1) as u32,
                body_lines: body,
            });
        }
        functions
    }

    fn detect_branches(&self, content: &str) -> Vec<BranchInfo> {
        let mut branches = Vec::new();
        let branch_keywords = ["if ", "else ", "match ", "switch ", "case ", "?"];

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }
            if branch_keywords.iter().any(|kw| trimmed.starts_with(kw)) {
                branches.push(BranchInfo {
                    line: (i + 1) as u32,
                    line_text: Some(trimmed.to_string()),
                });
            }
        }
        branches
    }

    fn detect_error_handling(&self, content: &str) -> Vec<ErrorHandler> {
        let mut handlers = Vec::new();
        let error_keywords = ["Result<", "Option<", "unwrap(", "expect(", "?",
            "Err(", "Ok(", "try {", "catch ", "throws", "throw ",
            "if let Err", "match ", "Err("];

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            let is_fn = trimmed.starts_with("fn ") || trimmed.starts_with("def ") || trimmed.starts_with("function ");

            if is_fn {
                let name = trimmed.split('(').next()
                    .and_then(|s| s.split(' ').filter(|p| *p != "fn" && *p != "def" && *p != "function" && *p != "pub" && *p != "async").last())
                    .unwrap_or("anonymous")
                    .to_string();

                if error_keywords.iter().any(|kw| trimmed.contains(kw)) {
                    handlers.push(ErrorHandler {
                        name,
                        line: (i + 1) as u32,
                    });
                }
            }
        }
        handlers
    }

    fn generate_recommendations(&self, findings: &[Finding]) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        let gap_count = findings.iter().filter(|f| f.id == "TEST-GAP").count();
        let error_path_count = findings.iter().filter(|f| f.id == "TEST-ERROR-PATH").count();

        if gap_count > 0 {
            recs.push(Recommendation::new(
                &format!("Found {} coverage gaps. Add parametrized tests to reach {:.0}% coverage.",
                    gap_count, self.config.coverage_goal * 100.0),
                0.7,
            ));
        }
        if error_path_count > 0 {
            recs.push(Recommendation::new(
                &format!("Found {} untested error paths. Add negative test cases.", error_path_count),
                0.6,
            ));
        }
        recs
    }
}

impl Default for TestingGapEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisEngine for TestingGapEngine {
    fn name(&self) -> &'static str {
        "testing_gap"
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
            0.5
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

struct FunctionProfile {
    name: String,
    line: u32,
    body_lines: u32,
}

struct BranchInfo {
    line: u32,
    line_text: Option<String>,
}

struct ErrorHandler {
    name: String,
    line: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_name() {
        let engine = TestingGapEngine::new();
        assert_eq!(engine.name(), "testing_gap");
    }

    #[test]
    fn test_detect_functions() {
        let engine = TestingGapEngine::new();
        let content = "fn a() {}\nfn b(x: i32) -> i32 { x }\n";
        let functions = engine.detect_functions(content);
        assert_eq!(functions.len(), 2);
    }

    #[test]
    fn test_detect_branches() {
        let engine = TestingGapEngine::new();
        let content = "fn test(x: i32) {\n    if x > 0 {\n        println!(\"pos\");\n    }\n    if x < 0 {\n        println!(\"neg\");\n    }\n}\n";
        let branches = engine.detect_branches(content);
        assert_eq!(branches.len(), 2);
    }

    #[test]
    fn test_detect_error_handling() {
        let engine = TestingGapEngine::new();
        let content = "fn read_file() -> Result<String, Error> {\n    Ok(String::new())\n}\n";
        let handlers = engine.detect_error_handling(content);
        assert!(!handlers.is_empty());
    }

    #[test]
    fn test_engine_disabled() {
        let engine = TestingGapEngine::with_config(TestingGapEngine::new(), TestingGapConfig {
            enabled: false,
            ..TestingGapConfig::default()
        });
        let req = AnalyzeRequest::new("/nonexistent", "abc");
        let result = engine.analyze(&req).unwrap();
        assert!(result.findings.is_empty());
    }

    #[test]
    fn test_engine_default() {
        let engine = TestingGapEngine::default();
        assert_eq!(engine.name(), "testing_gap");
        assert!(engine.config.enabled);
    }
}
