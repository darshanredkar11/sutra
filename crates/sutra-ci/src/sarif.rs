use sutra_schema::v1::{AnalysisResult, Severity};

#[derive(serde::Serialize)]
pub struct SarifLog {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<SarifRun>,
}

#[derive(serde::Serialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invocations: Option<Vec<SarifInvocation>>,
}

#[derive(serde::Serialize)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(serde::Serialize)]
pub struct SarifDriver {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub information_uri: Option<String>,
}

#[derive(serde::Serialize)]
pub struct SarifResult {
    pub rule_id: String,
    pub level: String,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
}

#[derive(serde::Serialize)]
pub struct SarifMessage {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

#[derive(serde::Serialize)]
pub struct SarifLocation {
    pub physical_location: SarifPhysicalLocation,
}

#[derive(serde::Serialize)]
pub struct SarifPhysicalLocation {
    pub artifact_location: SarifArtifactLocation,
    pub region: SarifRegion,
}

#[derive(serde::Serialize)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

#[derive(serde::Serialize)]
pub struct SarifRegion {
    pub start_line: u32,
}

#[derive(serde::Serialize)]
pub struct SarifInvocation {
    pub execution_successful: bool,
}

fn severity_to_sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "note",
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Critical => "error",
    }
}

pub fn to_sarif(result: &AnalysisResult) -> SarifLog {
    let sarif_results: Vec<SarifResult> = result
        .findings
        .iter()
        .map(|f| SarifResult {
            rule_id: f.id.clone(),
            level: severity_to_sarif_level(f.severity).to_string(),
            message: SarifMessage {
                text: format!("[{}] {}", f.engine.as_str(), f.message),
                markdown: Some(format!(
                    "**{}** [{}] {}",
                    f.id, f.engine.as_str(), f.message
                )),
            },
            locations: vec![SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation {
                        uri: f.file_path.clone(),
                    },
                    region: SarifRegion {
                        start_line: f.line,
                    },
                },
            }],
        })
        .collect();

    let invocation = if result.processing_time_ms > 0.0 {
        Some(vec![SarifInvocation {
            execution_successful: true,
        }])
    } else {
        None
    };

    SarifLog {
        schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".into(),
        version: "2.1.0".into(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "Sutra".into(),
                    version: "0.1.0".into(),
                    information_uri: Some("https://sutra.dev".into()),
                },
            },
            results: sarif_results,
            invocations: invocation,
        }],
    }
}

pub fn to_sarif_json(result: &AnalysisResult) -> String {
    serde_json::to_string_pretty(&to_sarif(result)).unwrap_or_else(|_| "{}".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sutra_schema::v1::{Engine, Finding, Recommendation};

    fn sample_result() -> AnalysisResult {
        AnalysisResult {
            request_id: "req-1".into(),
            commit_hash: "abc".into(),
            overall_risk: 0.75,
            findings: vec![
                Finding::new("MGTG-001", Engine::Mgtg, "src/main.rs", 42, "Resource leak", Severity::Error),
                Finding::new("DEP-001", Engine::Dependency, "src/lib.rs", 1, "Circular dependency", Severity::Warning),
                Finding::new("PROC-001", Engine::Process, "src/utils.rs", 1, "High entropy", Severity::Info),
            ],
            recommendations: vec![Recommendation::new("Fix things", 0.9)],
            metrics: None,
            processing_time_ms: 100.0,
            blocked_merge: false,
            jit_features: None,
        }
    }

    #[test]
    fn test_sarif_structure() {
        let result = sample_result();
        let sarif = to_sarif(&result);
        assert_eq!(sarif.version, "2.1.0");
        assert_eq!(sarif.runs.len(), 1);
        assert_eq!(sarif.runs[0].results.len(), 3);
    }

    #[test]
    fn test_sarif_tool_info() {
        let result = sample_result();
        let sarif = to_sarif(&result);
        let driver = &sarif.runs[0].tool.driver;
        assert_eq!(driver.name, "Sutra");
        assert_eq!(driver.version, "0.1.0");
        assert_eq!(driver.information_uri.as_deref(), Some("https://sutra.dev"));
    }

    #[test]
    fn test_sarif_level_mapping() {
        assert_eq!(severity_to_sarif_level(Severity::Critical), "error");
        assert_eq!(severity_to_sarif_level(Severity::Error), "error");
        assert_eq!(severity_to_sarif_level(Severity::Warning), "warning");
        assert_eq!(severity_to_sarif_level(Severity::Info), "note");
    }

    #[test]
    fn test_sarif_result_fields() {
        let result = sample_result();
        let sarif = to_sarif(&result);
        let r = &sarif.runs[0].results[0];
        assert_eq!(r.rule_id, "MGTG-001");
        assert!(r.message.markdown.is_some());
        assert!(r.message.text.contains("mgtg"));
        assert_eq!(r.locations[0].physical_location.artifact_location.uri, "src/main.rs");
        assert_eq!(r.locations[0].physical_location.region.start_line, 42);
    }

    #[test]
    fn test_sarif_invocation_present() {
        let result = sample_result();
        let sarif = to_sarif(&result);
        assert!(sarif.runs[0].invocations.is_some());
        assert!(sarif.runs[0].invocations.as_ref().unwrap()[0].execution_successful);
    }

    #[test]
    fn test_sarif_invocation_absent_when_no_time() {
        let result = AnalysisResult {
            processing_time_ms: 0.0,
            ..sample_result()
        };
        let sarif = to_sarif(&result);
        assert!(sarif.runs[0].invocations.is_none());
    }

    #[test]
    fn test_to_sarif_json_valid() {
        let result = sample_result();
        let json = to_sarif_json(&result);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["version"], "2.1.0");
        assert_eq!(parsed["runs"][0]["results"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_sarif_empty_findings() {
        let result = AnalysisResult {
            findings: vec![],
            ..sample_result()
        };
        let sarif = to_sarif(&result);
        assert!(sarif.runs[0].results.is_empty());
    }

    #[test]
    fn test_sarif_json_empty_result() {
        let result = AnalysisResult::new("req-empty", "abc");
        let sarif = to_sarif(&result);
        assert!(sarif.runs[0].results.is_empty());
    }

    #[test]
    fn test_sarif_serialization_roundtrip() {
        let result = sample_result();
        let json = to_sarif_json(&result);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let serialized_again = serde_json::to_string_pretty(&parsed).unwrap();
        let reparsed: serde_json::Value = serde_json::from_str(&serialized_again).unwrap();
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn test_sarif_thousand_findings() {
        let findings: Vec<Finding> = (0..1000)
            .map(|i| Finding::new(&format!("F{}", i), Engine::Mgtg, "f.rs", i, "msg", Severity::Warning))
            .collect();
        let result = AnalysisResult {
            findings,
            ..sample_result()
        };
        let sarif = to_sarif(&result);
        assert_eq!(sarif.runs[0].results.len(), 1000);
    }

    #[test]
    fn test_sarif_nan_risk_does_not_crash() {
        let result = AnalysisResult {
            overall_risk: f64::NAN,
            processing_time_ms: f64::NAN,
            ..sample_result()
        };
        let sarif = to_sarif(&result);
        assert_eq!(sarif.runs[0].results.len(), 3);
        let json = serde_json::to_string(&sarif).unwrap();
        assert!(!json.is_empty());
    }

    #[test]
    fn test_sarif_infinity_risk_does_not_crash() {
        let result = AnalysisResult {
            overall_risk: f64::INFINITY,
            processing_time_ms: f64::INFINITY,
            ..sample_result()
        };
        let sarif = to_sarif(&result);
        assert_eq!(sarif.runs[0].results.len(), 3);
        let json = serde_json::to_string(&sarif).unwrap();
        assert!(!json.is_empty());
    }

    #[test]
    fn test_sarif_line_zero() {
        let result = AnalysisResult {
            findings: vec![
                Finding::new("F1", Engine::Mgtg, "f.rs", 0, "file-level", Severity::Error),
            ],
            ..sample_result()
        };
        let sarif = to_sarif(&result);
        assert_eq!(sarif.runs[0].results[0].locations[0].physical_location.region.start_line, 0);
    }

    #[test]
    fn test_sarif_line_max() {
        let result = AnalysisResult {
            findings: vec![
                Finding::new("F1", Engine::Mgtg, "f.rs", u32::MAX, "far line", Severity::Warning),
            ],
            ..sample_result()
        };
        let sarif = to_sarif(&result);
        assert_eq!(sarif.runs[0].results[0].locations[0].physical_location.region.start_line, u32::MAX);
    }
}
