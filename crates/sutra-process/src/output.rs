use std::io::Write;

use crate::types::{CoChangeGraph, JitFeatures, ProcessAnalysis};

pub fn export_jit_json(features: &[JitFeatures]) -> Result<String, String> {
    serde_json::to_string_pretty(features).map_err(|e| format!("json: {}", e))
}

pub fn export_co_changes_json(graph: &CoChangeGraph) -> Result<String, String> {
    serde_json::to_string_pretty(&graph.edges).map_err(|e| format!("json: {}", e))
}

pub fn export_analysis_json(analysis: &ProcessAnalysis) -> Result<String, String> {
    serde_json::to_string_pretty(analysis).map_err(|e| format!("json: {}", e))
}

pub fn export_jit_csv<W: Write>(writer: W, features: &[JitFeatures]) -> Result<(), String> {
    let mut wtr = csv::Writer::from_writer(writer);

    wtr.write_record([
        "file_path",
        "revisions",
        "distinct_committers",
        "lines_added",
        "lines_deleted",
        "total_lines_changed",
        "entropy",
        "num_directories",
        "avg_files_per_commit",
        "age_days",
        "weighted_age_days",
        "recent_commits",
        "bug_fix_commits",
        "owner_contribution",
        "minor_contributors",
    ])
    .map_err(|e| format!("csv header: {}", e))?;

    for f in features {
        wtr.write_record([
            &f.file_path,
            &f.revisions.to_string(),
            &f.distinct_committers.to_string(),
            &f.lines_added.to_string(),
            &f.lines_deleted.to_string(),
            &f.total_lines_changed.to_string(),
            &f.entropy.to_string(),
            &f.num_directories.to_string(),
            &f.avg_files_per_commit.to_string(),
            &f.age_days.to_string(),
            &f.weighted_age_days.to_string(),
            &f.recent_commits.to_string(),
            &f.bug_fix_commits.to_string(),
            &f.owner_contribution.to_string(),
            &f.minor_contributors.to_string(),
        ])
        .map_err(|e| format!("csv row: {}", e))?;
    }

    wtr.flush().map_err(|e| format!("csv flush: {}", e))?;
    Ok(())
}

pub fn export_jit_csv_string(features: &[JitFeatures]) -> Result<String, String> {
    let mut buf = Vec::new();
    export_jit_csv(&mut buf, features)?;
    String::from_utf8(buf).map_err(|e| format!("utf8: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CoChange, JitFeatures};

    fn sample_features() -> Vec<JitFeatures> {
        vec![
            JitFeatures {
                file_path: "src/main.py".into(),
                revisions: 15,
                distinct_committers: 4,
                lines_added: 200,
                lines_deleted: 50,
                total_lines_changed: 250,
                entropy: 3.2,
                num_directories: 3,
                avg_files_per_commit: 2.1,
                age_days: 120.0,
                weighted_age_days: 45.0,
                recent_commits: 5,
                bug_fix_commits: 3,
                owner_contribution: 0.65,
                minor_contributors: 2,
            },
            JitFeatures {
                file_path: "src/utils.py".into(),
                revisions: 5,
                distinct_committers: 2,
                lines_added: 80,
                lines_deleted: 10,
                total_lines_changed: 90,
                entropy: 1.8,
                num_directories: 1,
                avg_files_per_commit: 1.2,
                age_days: 60.0,
                weighted_age_days: 20.0,
                recent_commits: 1,
                bug_fix_commits: 0,
                owner_contribution: 0.9,
                minor_contributors: 0,
            },
        ]
    }

    #[test]
    fn test_json_jit_features() {
        let json = export_jit_json(&sample_features()).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["file_path"], "src/main.py");
    }

    #[test]
    fn test_json_co_changes() {
        let graph = CoChangeGraph {
            edges: vec![CoChange {
                file_a: "a.py".into(),
                file_b: "b.py".into(),
                count: 5,
            }],
            by_file: std::collections::HashMap::new(),
        };
        let json = export_co_changes_json(&graph).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["file_a"], "a.py");
    }

    #[test]
    fn test_csv_output() {
        let csv = export_jit_csv_string(&sample_features()).unwrap();
        assert!(csv.contains("file_path"));
        assert!(csv.contains("src/main.py"));
        assert!(csv.contains("src/utils.py"));
        assert!(csv.contains("15")); // revisions
        assert!(csv.contains("3.2")); // entropy
    }

    #[test]
    fn test_csv_header() {
        let csv = export_jit_csv_string(&[]).unwrap();
        assert!(csv.contains("file_path"));
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1); // header only
    }

    #[test]
    fn test_json_empty() {
        let json = export_jit_json(&[]).unwrap();
        assert_eq!(json, "[]");
    }
}
