use std::time::Instant;

use sutra_common::engine::AnalysisEngine;
use sutra_common::error::{SutraError, SutraResult};
use sutra_schema::v1::{
    AnalysisResult, AnalyzeRequest, Engine, Finding, MetricsSummary, Recommendation, Severity,
};

fn convert_severity(s: &str) -> Severity {
    match s {
        "critical" => Severity::Critical,
        "error" => Severity::Error,
        "warning" => Severity::Warning,
        _ => Severity::Info,
    }
}

fn convert_findings(mgtg_files: &[mgtg::ir::AnalysisFile]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for file in mgtg_files {
        for mf in &file.findings {
            findings.push(Finding {
                id: mf.id.clone(),
                engine: Engine::Mgtg,
                file_path: mf.file.clone(),
                line: mf.line as u32,
                message: mf.message.clone(),
                severity: convert_severity(&mf.severity),
                validated: false,
                suggested_fix: None,
            });
        }
    }
    findings
}

fn convert_metrics(mgtg_files: &[mgtg::ir::AnalysisFile]) -> MetricsSummary {
    let mut total_functions = 0u32;
    let mut max_cyclomatic = 0.0f64;
    let mut max_cognitive = 0.0f64;
    let mut max_nesting = 0.0f64;

    for file in mgtg_files {
        total_functions += file.functions.len() as u32;
        max_cyclomatic = max_cyclomatic.max(file.metrics.cyclomatic_max as f64);
        max_cognitive = max_cognitive.max(file.metrics.cognitive_max as f64);
        max_nesting = max_nesting.max(file.metrics.nesting_depth_max as f64);
    }

    MetricsSummary {
        cyclomatic_max: max_cyclomatic,
        cognitive_max: max_cognitive,
        nesting_max: max_nesting,
        total_functions,
        total_files: mgtg_files.len() as u32,
        ..Default::default()
    }
}

fn convert_recommendations(mgtg_files: &[mgtg::ir::AnalysisFile]) -> Vec<Recommendation> {
    let mut recs = Vec::new();

    let error_count: usize = mgtg_files
        .iter()
        .flat_map(|f| &f.findings)
        .filter(|f| f.severity == "error")
        .count();

    let warning_count: usize = mgtg_files
        .iter()
        .flat_map(|f| &f.findings)
        .filter(|f| f.severity == "warning")
        .count();

    if error_count > 0 {
        recs.push(Recommendation::new(
            &format!("Fix {} error(s) to reduce failure risk", error_count),
            0.9,
        ));
    }

    if warning_count > 0 {
        recs.push(Recommendation::new(
            &format!("Address {} warning(s) for improved code quality", warning_count),
            0.6,
        ));
    }

    recs
}

pub struct MgtgEngine;

impl MgtgEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MgtgEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisEngine for MgtgEngine {
    fn name(&self) -> &'static str {
        "mgtg"
    }

    fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
        let start = Instant::now();

        let mgtg_result = mgtg::analyze(&request.repo_path, None)
            .map_err(|e| SutraError::engine("mgtg", e))?;

        let processing_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        let findings = convert_findings(&mgtg_result.files);
        let metrics = convert_metrics(&mgtg_result.files);
        let recommendations = convert_recommendations(&mgtg_result.files);

        let has_errors = findings.iter().any(|f| f.severity == Severity::Error);
        let has_critical = findings.iter().any(|f| f.severity == Severity::Critical);

        Ok(AnalysisResult {
            request_id: request.request_id.clone(),
            commit_hash: request.commit_hash.clone(),
            overall_risk: 1.0 - mgtg_result.summary.overall_health,
            findings,
            recommendations,
            metrics: Some(metrics),
            processing_time_ms,
            blocked_merge: has_critical || (has_errors && mgtg_result.summary.overall_health < 0.3),
            jit_features: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sutra_schema::v1::AnalyzeRequest;

    #[test]
    fn test_engine_name() {
        let engine = MgtgEngine::new();
        assert_eq!(engine.name(), "mgtg");
    }

    #[test]
    fn test_severity_conversion() {
        assert_eq!(convert_severity("error"), Severity::Error);
        assert_eq!(convert_severity("warning"), Severity::Warning);
        assert_eq!(convert_severity("info"), Severity::Info);
        assert_eq!(convert_severity("critical"), Severity::Critical);
        assert_eq!(convert_severity("unknown"), Severity::Info);
    }

    #[test]
    fn test_convert_findings_empty() {
        assert!(convert_findings(&[]).is_empty());
    }

    #[test]
    fn test_convert_metrics_empty() {
        let m = convert_metrics(&[]);
        assert_eq!(m.total_files, 0);
        assert_eq!(m.cyclomatic_max, 0.0);
    }

    #[test]
    fn test_convert_recommendations_empty() {
        assert!(convert_recommendations(&[]).is_empty());
    }

    #[test]
    fn test_analyze_nonexistent_path() {
        let engine = MgtgEngine::new();
        let req = AnalyzeRequest::new("/nonexistent/path/xyz", "abc123");
        let result = engine.analyze(&req);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Path"), "Error should mention path: {}", err);
    }

    #[test]
    fn test_analyze_invalid_path_format() {
        let engine = MgtgEngine::new();
        let req = AnalyzeRequest::new("", "abc123");
        let result = engine.analyze(&req);
        assert!(result.is_err());
    }
}
