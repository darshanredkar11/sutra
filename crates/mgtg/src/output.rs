use crate::ir::AnalysisResult;

pub fn format_pretty(result: &AnalysisResult) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "mgtg v{} | Analysis ID: {}\n",
        result.version, result.analysis_id
    ));
    out.push_str(&format!(
        "Files scanned: {} | Findings: {} | Health: {:.2}\n",
        result.summary.total_files,
        result.summary.total_findings,
        result.summary.overall_health
    ));
    out.push_str(&format!(
        "  {} errors  {} warnings  {} info\n\n",
        result.summary.errors, result.summary.warnings, result.summary.info
    ));

    for file in &result.files {
        out.push_str(&format!(
            "📄 {} ({}): health {:.2}\n",
            file.path, file.language, file.health_score
        ));

        if !file.findings.is_empty() {
            for f in &file.findings {
                let icon = match f.severity.as_str() {
                    "error" => "✗",
                    "warning" => "⚠",
                    _ => "ℹ",
                };
                out.push_str(&format!(
                    "  {} [{}] {}:{} — {} ({})\n",
                    icon, f.id, f.file, f.line, f.message, f.subtype
                ));
            }
        }

        out.push_str(&format!(
            "  Metrics: cyclomatic={} cognitive={} nesting={} missing_branches={} resource_risks={}\n\n",
            file.metrics.cyclomatic_max,
            file.metrics.cognitive_max,
            file.metrics.nesting_depth_max,
            file.metrics.missing_branches,
            file.metrics.resource_risks,
        ));
    }

    out
}

pub fn format_json(result: &AnalysisResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".to_string())
}

pub fn format_quiet(result: &AnalysisResult) -> String {
    format!(
        "{} files | {} findings ({} err, {} warn, {} info) | health {:.2}",
        result.summary.total_files,
        result.summary.total_findings,
        result.summary.errors,
        result.summary.warnings,
        result.summary.info,
        result.summary.overall_health
    )
}
