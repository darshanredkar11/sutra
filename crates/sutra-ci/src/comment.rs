use sutra_schema::v1::{AnalysisResult, Severity};

pub fn format_pr_comment(result: &AnalysisResult) -> String {
    let finding_count = result.findings.len();
    let error_count = result
        .findings
        .iter()
        .filter(|f| matches!(f.severity, Severity::Error | Severity::Critical))
        .count();
    let warning_count = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .count();
    let info_count = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Info)
        .count();

    let risk_badge = if result.overall_risk >= 0.8 {
        "🔴 **HIGH**"
    } else if result.overall_risk >= 0.5 {
        "🟡 **MEDIUM**"
    } else {
        "🟢 **LOW**"
    };

    let mut comment = String::new();
    comment.push_str("## Sutra Analysis Report\n\n");
    comment.push_str(&format!(
        "**Risk**: {} ({:.2})\n\n",
        risk_badge, result.overall_risk
    ));
    comment.push_str(&format!(
        "**Findings**: {} total — {} errors, {} warnings, {} info\n\n",
        finding_count, error_count, warning_count, info_count
    ));

    if !result.findings.is_empty() {
        comment.push_str("### Findings\n\n");
        comment.push_str("| ID | Engine | File | Line | Severity | Message |\n");
        comment.push_str("|---|---|---|---|---|---|\n");

        let mut sorted = result.findings.clone();
        sorted.sort_by_key(|f| std::cmp::Reverse(f.severity.rank()));

        for f in sorted {
            let level_icon = match f.severity {
                Severity::Critical | Severity::Error => "🔴",
                Severity::Warning => "🟡",
                Severity::Info => "🔵",
            };
            comment.push_str(&format!(
                "| {} | `{}` | `{}` | {} | {} | {} |\n",
                f.id, f.engine.as_str(), f.file_path, f.line, level_icon, f.message
            ));
        }
        comment.push('\n');
    }

    if !result.recommendations.is_empty() {
        comment.push_str("### Recommendations\n\n");
        for rec in &result.recommendations {
            comment.push_str(&format!(
                "- **[{:.0}% confidence]** {}\n",
                rec.priority * 100.0,
                rec.text
            ));
        }
        comment.push('\n');
    }

    if let Some(metrics) = &result.metrics {
        comment.push_str("### Metrics\n\n");
        comment.push_str(&format!("- Files analyzed: {}\n", metrics.total_files));
        comment.push_str(&format!("- Functions analyzed: {}\n", metrics.total_functions));
        if metrics.cyclomatic_max > 0.0 {
            comment.push_str(&format!("- Max cyclomatic complexity: {}\n", metrics.cyclomatic_max));
        }
        if metrics.circular_dependencies > 0 {
            comment.push_str(&format!("- Circular dependencies: {}\n", metrics.circular_dependencies));
        }
    }

    comment.push_str(&format!(
        "\n---\n_Analyzed in {:.0}ms_",
        result.processing_time_ms
    ));

    comment
}

pub fn format_ci_status(result: &AnalysisResult) -> String {
    if result.blocked_merge || result.overall_risk >= 0.8 {
        format!(
            "Sutra: ❌ FAIL (risk={:.2}, {} findings)",
            result.overall_risk,
            result.findings.len()
        )
    } else if result.overall_risk >= 0.5 {
        format!(
            "Sutra: ⚠️ WARN (risk={:.2}, {} findings)",
            result.overall_risk,
            result.findings.len()
        )
    } else {
        format!(
            "Sutra: ✅ PASS (risk={:.2})",
            result.overall_risk
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sutra_schema::v1::{Engine, Finding, MetricsSummary, Recommendation};

    fn sample_result() -> AnalysisResult {
        AnalysisResult {
            request_id: "req-1".into(),
            commit_hash: "abc".into(),
            overall_risk: 0.75,
            findings: vec![
                Finding::new("M-001", Engine::Mgtg, "src/main.rs", 42, "Resource leak", Severity::Error),
                Finding::new("M-002", Engine::Mgtg, "src/lib.rs", 10, "Unused var", Severity::Warning),
            ],
            recommendations: vec![
                Recommendation::new("Fix resource leak", 0.9),
            ],
            metrics: Some(MetricsSummary {
                total_files: 10,
                total_functions: 50,
                cyclomatic_max: 15.0,
                ..Default::default()
            }),
            processing_time_ms: 123.45,
            blocked_merge: false,
            jit_features: None,
        }
    }

    #[test]
    fn test_format_pr_comment_contains_risk() {
        let comment = format_pr_comment(&sample_result());
        assert!(comment.contains("Sutra Analysis Report"));
        assert!(comment.contains("MEDIUM"));
        assert!(comment.contains("0.75"));
    }

    #[test]
    fn test_format_pr_comment_findings_table() {
        let comment = format_pr_comment(&sample_result());
        assert!(comment.contains("| M-001 |"));
        assert!(comment.contains("| M-002 |"));
        assert!(comment.contains("🔴")); // Error
        assert!(comment.contains("🟡")); // Warning
    }

    #[test]
    fn test_format_pr_comment_recommendations() {
        let comment = format_pr_comment(&sample_result());
        assert!(comment.contains("90%"));
        assert!(comment.contains("Fix resource leak"));
    }

    #[test]
    fn test_format_pr_comment_metrics() {
        let comment = format_pr_comment(&sample_result());
        assert!(comment.contains("Files analyzed: 10"));
        assert!(comment.contains("Functions analyzed: 50"));
        assert!(comment.contains("Max cyclomatic complexity: 15"));
    }

    #[test]
    fn test_format_pr_comment_processing_time() {
        let comment = format_pr_comment(&sample_result());
        assert!(comment.contains("123ms"));
    }

    #[test]
    fn test_format_pr_comment_empty_findings() {
        let result = AnalysisResult {
            findings: vec![],
            recommendations: vec![],
            metrics: None,
            ..sample_result()
        };
        let comment = format_pr_comment(&result);
        assert!(comment.contains("0 total"));
        assert!(!comment.contains("### Findings"));
    }

    #[test]
    fn test_format_ci_status_fail() {
        let result = AnalysisResult {
            overall_risk: 0.9,
            ..sample_result()
        };
        let status = format_ci_status(&result);
        assert!(status.contains("FAIL"));
    }

    #[test]
    fn test_format_ci_status_warn() {
        let result = AnalysisResult {
            overall_risk: 0.6,
            blocked_merge: false,
            ..sample_result()
        };
        let status = format_ci_status(&result);
        assert!(status.contains("WARN"));
    }

    #[test]
    fn test_format_ci_status_pass() {
        let result = AnalysisResult {
            overall_risk: 0.2,
            blocked_merge: false,
            ..sample_result()
        };
        let status = format_ci_status(&result);
        assert!(status.contains("PASS"));
    }

    #[test]
    fn test_format_ci_status_blocked_merge_is_fail() {
        let result = AnalysisResult {
            overall_risk: 0.3,
            blocked_merge: true,
            ..sample_result()
        };
        let status = format_ci_status(&result);
        assert!(status.contains("FAIL"));
    }

    #[test]
    fn test_format_pr_comment_low_risk() {
        let result = AnalysisResult {
            overall_risk: 0.1,
            ..sample_result()
        };
        let comment = format_pr_comment(&result);
        assert!(comment.contains("LOW"));
    }

    #[test]
    fn test_format_pr_comment_high_risk() {
        let result = AnalysisResult {
            overall_risk: 0.9,
            ..sample_result()
        };
        let comment = format_pr_comment(&result);
        assert!(comment.contains("HIGH"));
    }

    #[test]
    fn test_format_pr_comment_no_metrics_section() {
        let result = AnalysisResult {
            metrics: None,
            ..sample_result()
        };
        let comment = format_pr_comment(&result);
        assert!(!comment.contains("### Metrics"));
    }

    #[test]
    fn test_format_pr_comment_all_severities() {
        let result = AnalysisResult {
            findings: vec![
                Finding::new("I-001", Engine::Mgtg, "f.rs", 1, "info", Severity::Info),
                Finding::new("W-001", Engine::Mgtg, "f.rs", 2, "warning", Severity::Warning),
                Finding::new("E-001", Engine::Mgtg, "f.rs", 3, "error", Severity::Error),
                Finding::new("C-001", Engine::Mgtg, "f.rs", 4, "critical", Severity::Critical),
            ],
            ..sample_result()
        };
        let comment = format_pr_comment(&result);
        assert!(comment.contains("I-001"));
        assert!(comment.contains("W-001"));
        assert!(comment.contains("E-001"));
        assert!(comment.contains("C-001"));
    }

    #[test]
    fn test_format_pr_comment_long_message() {
        let long_msg = "A".repeat(10000);
        let result = AnalysisResult {
            findings: vec![
                Finding::new("L-001", Engine::Mgtg, "f.rs", 1, &long_msg, Severity::Warning),
            ],
            ..sample_result()
        };
        let comment = format_pr_comment(&result);
        assert!(comment.contains(&long_msg));
    }

    #[test]
    fn test_format_pr_comment_unicode() {
        let result = AnalysisResult {
            findings: vec![
                Finding::new("U-001", Engine::Mgtg, "f.rs", 1, "🔥 emoji & 中文 & español", Severity::Info),
            ],
            ..sample_result()
        };
        let comment = format_pr_comment(&result);
        assert!(comment.contains("🔥"));
        assert!(comment.contains("中文"));
        assert!(comment.contains("español"));
    }

    #[test]
    fn test_format_pr_comment_hundred_findings() {
        let findings: Vec<Finding> = (0..100)
            .map(|i| Finding::new(&format!("F{:03}", i), Engine::Mgtg, "f.rs", i, "msg", Severity::Warning))
            .collect();
        let result = AnalysisResult {
            findings,
            ..sample_result()
        };
        let comment = format_pr_comment(&result);
        assert!(comment.contains("100 total"));
        assert!(comment.contains("F099"));
    }
}
