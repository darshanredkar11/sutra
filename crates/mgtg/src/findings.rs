use crate::ir::{Finding, Metrics};

pub fn finding_id(category: &str, idx: usize) -> String {
    format!("MGTG-{}{:03}", category, idx)
}

pub fn severity_score(severity: &str) -> usize {
    match severity {
        "error" => 3,
        "warning" => 2,
        "info" => 1,
        _ => 0,
    }
}

/// Compute a health score (0.0 – 1.0) based on findings and metrics.
pub fn compute_health_score(metrics: &Metrics, findings: &[Finding]) -> f64 {
    let error_count = findings.iter().filter(|f| f.severity == "error").count();
    let warning_count = findings.iter().filter(|f| f.severity == "warning").count();
    let info_count = findings.iter().filter(|f| f.severity == "info").count();

    let base = 1.0;
    let error_penalty = error_count as f64 * 0.15;
    let warning_penalty = warning_count as f64 * 0.05;
    let info_penalty = info_count as f64 * 0.01;

    let metric_penalty = (metrics.cyclomatic_max.saturating_sub(10) as f64 * 0.02)
        + (metrics.nesting_depth_max.saturating_sub(4) as f64 * 0.03)
        + (metrics.resource_risks as f64 * 0.1);

    let score = base - error_penalty - warning_penalty - info_penalty - metric_penalty;
    score.clamp(0.0, 1.0)
}

pub fn count_severities(findings: &[Finding]) -> (usize, usize, usize) {
    let errors = findings.iter().filter(|f| f.severity == "error").count();
    let warnings = findings.iter().filter(|f| f.severity == "warning").count();
    let info = findings.iter().filter(|f| f.severity == "info").count();
    (errors, warnings, info)
}
