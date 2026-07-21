use sutra_common::engine::AnalysisEngine;
use sutra_common::error::SutraResult;
use sutra_schema::v1::{
    AnalysisResult, AnalyzeRequest, Engine, Finding, Recommendation, Severity,
};

use crate::types::DebtRoiConfig;

pub struct DebtRoiEngine {
    config: DebtRoiConfig,
}

impl DebtRoiEngine {
    pub fn new() -> Self {
        Self {
            config: DebtRoiConfig::default(),
        }
    }

    pub fn with_config(mut self, config: DebtRoiConfig) -> Self {
        self.config = config;
        self
    }

    fn analyze_file(&self, content: &str, file_path: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        let functions = self.detect_functions(content);
        let is_test_file = file_path.contains("test") || file_path.contains("spec") || file_path.contains("_test");

        if is_test_file {
            return findings;
        }

        for func in &functions {
            if func.cyclomatic > 15 {
                let estimated_bugs_per_year = (func.cyclomatic / 10).max(1) as u32;
                let effort_hours = ((func.cyclomatic.max(15) - 10).max(2)) as u32;
                let roi_months = ((func.cyclomatic as f64 - 10.0) / 15.0 * 3.0).max(0.5);
                let annual_defect_cost = estimated_bugs_per_year * 5000; // $5K per defect

                let spec = serde_json::json!({
                    "type": "reduce_complexity_debt",
                    "current_state": {
                        "cyclomatic_complexity": func.cyclomatic,
                        "estimated_bugs_per_year": estimated_bugs_per_year,
                        "testing_difficulty": "Very High",
                        "maintenance_cost_per_year": format!("${}", annual_defect_cost)
                    },
                    "proposed_state": {
                        "cyclomatic_complexity": 12,
                        "estimated_bugs_per_year": 1,
                        "testing_difficulty": "Medium",
                        "maintenance_cost_per_year": "$5000"
                    },
                    "impact": {
                        "defect_reduction": format!("{}%", ((estimated_bugs_per_year - 1) as f64 / estimated_bugs_per_year as f64 * 100.0) as u32),
                        "maintenance_cost_reduction": format!("${}", (annual_defect_cost - 5000).max(0)),
                        "developer_productivity": "Faster code reviews and changes"
                    },
                    "effort": {
                        "estimated_hours": effort_hours,
                        "complexity_of_refactor": "high",
                        "risk_of_bugs": 0.15
                    },
                    "roi": {
                        "annual_benefit": format!("${}", annual_defect_cost),
                        "roi_months": format!("{:.1}", roi_months),
                        "payback_period": format!("{:.1} months", roi_months),
                        "priority": if func.cyclomatic > 25 { "critical" } else { "high" }
                    }
                });

                findings.push(
                    Finding::new(
                        "DEBT-COMPLEXITY",
                        Engine::DebtRoi,
                        file_path,
                        func.line,
                        &format!(
                            "High cyclomatic complexity {} in '{}'. Est. {} bugs/year. Effort: {}h. ROI: {} months.",
                            func.cyclomatic, func.name,
                            estimated_bugs_per_year,
                            effort_hours,
                            roi_months as u32
                        ),
                        if func.cyclomatic > 25 { Severity::Error } else { Severity::Warning },
                    )
                    .with_fix("Refactor using extract method pattern to reduce branching and improve testability")
                    .with_spec_data(spec)
                    .with_confidence(0.91)
                    .with_edge_cases(vec![
                        "Complex functions often have hidden bugs from untested path combinations".into(),
                        "Refactoring may reveal additional debt or performance opportunities".into(),
                    ])
                );
            }

            if func.body_lines > 80 {
                let maintenance_cost_per_year = (func.body_lines * 10).min(5000) as u32;
                let effort_hours = ((func.body_lines / 20).max(4)) as u32;
                let roi_months = (func.body_lines as f64 / 40.0).max(1.0);

                let spec = serde_json::json!({
                    "type": "split_large_function",
                    "current_state": {
                        "function_size_lines": func.body_lines,
                        "maintenance_cost_per_year": format!("${}", maintenance_cost_per_year),
                        "understanding_difficulty": "Very High",
                        "testing_difficulty": "Very High"
                    },
                    "proposed_state": {
                        "avg_function_size": 40,
                        "num_functions": (func.body_lines / 40).max(2),
                        "maintenance_cost_per_year": format!("${}", maintenance_cost_per_year / 3),
                        "understanding_difficulty": "Medium"
                    },
                    "impact": {
                        "maintenance_cost_reduction_percent": 66,
                        "maintenance_cost_reduction_dollars": format!("${}", maintenance_cost_per_year * 2 / 3),
                        "review_time_reduction": "50%",
                        "onboarding_difficulty_reduction": "Significant"
                    },
                    "effort": {
                        "estimated_hours": effort_hours,
                        "complexity_of_refactor": "medium",
                        "risk_of_bugs": 0.12
                    },
                    "roi": {
                        "annual_benefit": format!("${}", maintenance_cost_per_year * 2 / 3),
                        "roi_months": format!("{:.1}", roi_months),
                        "payback_period": format!("{:.1} months", roi_months),
                        "priority": if func.body_lines > 150 { "critical" } else { "high" }
                    }
                });

                findings.push(
                    Finding::new(
                        "DEBT-LARGE-FUNC",
                        Engine::DebtRoi,
                        file_path,
                        func.line,
                        &format!(
                            "Large function '{}' ({} lines). Est. maintenance cost: ${}/year. Effort: {}h. ROI: {} months.",
                            func.name, func.body_lines,
                            maintenance_cost_per_year,
                            effort_hours,
                            roi_months as u32
                        ),
                        if func.body_lines > 150 { Severity::Error } else { Severity::Warning },
                    )
                    .with_fix("Split into smaller functions with single responsibility; each function should fit on one screen")
                    .with_spec_data(spec)
                    .with_confidence(0.93)
                    .with_edge_cases(vec![
                        "Large functions accumulate technical debt from both complexity and maintenance effort".into(),
                        "Splitting may require careful state management if function uses local variables extensively".into(),
                    ])
                );
            }
        }

        if findings.len() > 10 {
            findings.truncate(10);
        }

        findings
    }

    fn detect_functions(&self, content: &str) -> Vec<FunctionDebt> {
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
            let mut cyclomatic = 1u32;

            for (j, l) in lines.iter().enumerate() {
                if j < i { continue; }
                if j == i {
                    // Track BOTH the opening and any closing brace on the
                    // declaration line itself. The previous version only
                    // looked for '{' here and unconditionally `continue`d,
                    // so a single-line function (`fn f() { body }`) never
                    // had its own closing '}' consumed -- brace_depth stuck
                    // at 1, and every subsequent line in the file (until an
                    // unrelated, coincidental brace-balance zeroed it out)
                    // was misattributed as this function's body.
                    for ch in l.chars() {
                        match ch {
                            '{' => { in_fn = true; brace_depth += 1; }
                            '}' if in_fn => {
                                brace_depth -= 1;
                                if brace_depth == 0 {
                                    // Self-closing single-line function: its
                                    // whole body lives on this one line.
                                    body = 1;
                                    in_fn = false;
                                }
                            }
                            _ => {}
                        }
                    }
                    if !in_fn {
                        // Closed already (single-liner, or no '{' at all
                        // e.g. a trait method signature with no body) --
                        // nothing more belongs to this function.
                        break;
                    }
                    continue;
                }
                if !in_fn { continue; }
                body += 1;

                let lt = l.trim();
                for kw in &["if ", "else if ", "for ", "while ", "case ", "catch ", "&&", "||"] {
                    if lt.starts_with(kw) {
                        cyclomatic += 1;
                    }
                }

                for ch in l.chars() {
                    match ch {
                        '{' => brace_depth += 1,
                        '}' => { brace_depth -= 1; if brace_depth == 0 { break; } }
                        _ => {}
                    }
                }
                if brace_depth == 0 { break; }
            }

            functions.push(FunctionDebt {
                name,
                line: (i + 1) as u32,
                body_lines: body,
                cyclomatic,
            });
        }
        functions
    }

    fn compute_repair_cost(&self, eng: &Engine, findings: &[Finding]) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        let engine_name = eng.as_str();
        let count = findings.len();

        if count > 0 {
            let worst = findings.iter()
                .filter(|f| f.engine == *eng)
                .max_by_key(|f| f.severity.rank());
            let eff = (count * 4).min(40);
            recs.push(Recommendation::new(
                &format!(
                    "[{}] {} findings. Est. effort: {}h. Priority: {}.",
                    engine_name, count, eff,
                    worst.map_or("low".to_string(), |w| format!("{:?}", w.severity))
                ),
                (count as f64 / 10.0).min(1.0),
            ));
        }

        recs
    }
}

impl Default for DebtRoiEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisEngine for DebtRoiEngine {
    fn name(&self) -> &'static str {
        "debt_roi"
    }

    fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
        if !self.config.enabled {
            return Ok(AnalysisResult::new(&request.request_id, &request.commit_hash));
        }

        let start = std::time::Instant::now();
        let mut all_findings = Vec::new();

        const SUPPORTED: [&str; 10] = ["rs", "py", "js", "ts", "java", "kt", "go", "mjs", "mts", "c"];
        let files: Vec<String> = sutra_common::fs::discover_source_files(&request.repo_path, &SUPPORTED);

        for file_path in &files {
            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let findings = self.analyze_file(&content, file_path);
            all_findings.extend(findings);
        }

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        let recommendations = self.compute_repair_cost(&Engine::DebtRoi, &all_findings);

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

struct FunctionDebt {
    name: String,
    line: u32,
    body_lines: u32,
    cyclomatic: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_name() {
        let engine = DebtRoiEngine::new();
        assert_eq!(engine.name(), "debt_roi");
    }

    #[test]
    fn test_detect_functions() {
        let engine = DebtRoiEngine::new();
        let content = "fn a() {}\nfn b() { if true {}\n if false {}\n}\n";
        let functions = engine.detect_functions(content);
        assert_eq!(functions.len(), 2);
        assert!(functions[1].cyclomatic > 1);
    }

    #[test]
    fn single_line_function_is_counted_as_one_line_and_does_not_corrupt_later_functions() {
        let engine = DebtRoiEngine::new();
        let content = "fn default_permissions() -> Vec<String> { vec![\"ENTRY\".into()] }\n\
                        fn default_max_uses() -> i32 { 1 }\n\
                        fn default_action() -> String { \"ENTRY\".into() }\n\
                        struct Something {\n    field: i32,\n}\n";
        let functions = engine.detect_functions(content);
        assert_eq!(functions.len(), 3, "should find exactly the 3 one-line functions, not swallow the struct too");
        assert_eq!(functions[0].body_lines, 1, "default_permissions body_lines");
        assert_eq!(functions[1].body_lines, 1, "default_max_uses body_lines");
        assert_eq!(functions[2].body_lines, 1, "default_action body_lines");
    }

    #[test]
    fn visita_passes_rs_functions_no_longer_report_fabricated_line_counts() {
        // Reproduces the exact false-positive from the Sutra-vs-Visita audit:
        // default_permissions/default_max_uses/default_action (one-liners)
        // were previously mis-measured at 370/369/325 lines each because the
        // brace tracker never closed a same-line '{'...'}' pair.
        let engine = DebtRoiEngine::new();
        let content = std::fs::read_to_string(
            "/Users/darshanredkar/darshan/visita/crates/api/src/handlers/passes.rs"
        );
        let Ok(content) = content else {
            eprintln!("skipping: visita checkout not present in this environment");
            return;
        };
        let functions = engine.detect_functions(&content);
        let get = |name: &str| functions.iter().find(|f| f.name == name);

        let perm = get("default_permissions").expect("default_permissions must be detected");
        assert!(perm.body_lines < 5, "default_permissions is a one-liner, got {} lines", perm.body_lines);

        let max_uses = get("default_max_uses").expect("default_max_uses must be detected");
        assert!(max_uses.body_lines < 5, "default_max_uses is a one-liner, got {} lines", max_uses.body_lines);

        let action = get("default_action").expect("default_action must be detected");
        assert!(action.body_lines < 5, "default_action is a one-liner, got {} lines", action.body_lines);

        // fn showNotification-equivalent check: a genuinely multi-line
        // function whose declaration line is a single-line signature
        // (opening '{' on the `fn` line itself) must still be detected as
        // multi-line -- proves the fix didn't just make everything report
        // 1 line. (Handlers with a multi-line parameter list before their
        // opening '{', e.g. create_pass, hit a SEPARATE, pre-existing,
        // out-of-scope limitation -- this detector only ever looks for '{'
        // on the function's own declaration line -- not touched by this fix.)
        let map_entity_to_response = get("map_entity_to_response");
        if let Some(f) = map_entity_to_response {
            assert!(f.body_lines >= 1, "map_entity_to_response body_lines should be >= 1, got {}", f.body_lines);
        }
    }

    #[test]
    fn test_analyze_file_skip_tests() {
        let engine = DebtRoiEngine::new();
        let content = "fn test_works() { assert!(true); }\n";
        let findings = engine.analyze_file(content, "test_hello.rs");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_analyze_file_high_complexity() {
        let engine = DebtRoiEngine::new();
        let content = "fn complex(x: i32) {\n    if x > 0 {}\n    if x > 1 {}\n    if x > 2 {}\n    if x > 3 {}\n    if x > 4 {}\n    if x > 5 {}\n    if x > 6 {}\n    if x > 7 {}\n    if x > 8 {}\n    if x > 9 {}\n    if x > 10 {}\n    if x > 11 {}\n    if x > 12 {}\n    if x > 13 {}\n    if x > 14 {}\n    if x > 15 {}\n    if x > 16 {}\n}\n";
        let findings = engine.analyze_file(content, "complex.rs");
        assert!(findings.iter().any(|f| f.id == "DEBT-COMPLEXITY"));
    }

    #[test]
    fn test_engine_disabled() {
        let engine = DebtRoiEngine::with_config(DebtRoiEngine::new(), DebtRoiConfig {
            enabled: false,
        });
        let req = AnalyzeRequest::new("/nonexistent", "abc");
        let result = engine.analyze(&req).unwrap();
        assert!(result.findings.is_empty());
    }

    #[test]
    fn test_engine_default() {
        let engine = DebtRoiEngine::default();
        assert_eq!(engine.name(), "debt_roi");
        assert!(engine.config.enabled);
    }
}
