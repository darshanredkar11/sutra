use std::time::Instant;

use sutra_common::engine::AnalysisEngine;
use sutra_common::error::{SutraError, SutraResult};
use sutra_schema::v1::{
    AnalysisResult, AnalyzeRequest, Engine as SutraEngine, Finding, MetricsSummary,
    Recommendation, Severity,
};

use crate::cochange::build_co_change_graph;
use crate::entropy::compute_entropy;
use crate::gitwalk::{open_repo, walk_commits};
use crate::jitfeatures::extract_jit_features;
use crate::persist;
use crate::types::JitFeatures;

pub struct ProcessEngine {
    max_commits: usize,
    persist_path: Option<String>,
}

impl ProcessEngine {
    pub fn new() -> Self {
        Self {
            max_commits: 10000,
            persist_path: None,
        }
    }

    pub fn with_max_commits(mut self, max: usize) -> Self {
        self.max_commits = max;
        self
    }

    pub fn with_persist(mut self, path: &str) -> Self {
        self.persist_path = Some(path.to_string());
        self
    }
}

impl Default for ProcessEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisEngine for ProcessEngine {
    fn name(&self) -> &'static str {
        "process"
    }

    fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
        let start = Instant::now();
        let repo_path = &request.repo_path;

        let repo = open_repo(repo_path)
            .map_err(|e| SutraError::engine("process", e))?;

        let commits = walk_commits(&repo, self.max_commits)
            .map_err(|e| SutraError::engine("process", e))?;

        if commits.is_empty() {
            return Ok(AnalysisResult {
                request_id: request.request_id.clone(),
                commit_hash: request.commit_hash.clone(),
                overall_risk: 0.0,
                findings: vec![],
                recommendations: vec![],
                metrics: Some(MetricsSummary {
                    total_files: 0,
                    ..Default::default()
                }),
                processing_time_ms: start.elapsed().as_secs_f64() * 1000.0,
                blocked_merge: false,
            });
        }

        let now_ms = commits[0].timestamp_ms;

        let entropy_map = compute_entropy(&commits);
        let co_changes = build_co_change_graph(&commits);
        let jit_features = extract_jit_features(&commits, &entropy_map, now_ms);

        let mut findings: Vec<Finding> = Vec::new();
        let mut recommendations: Vec<Recommendation> = Vec::new();

        // High entropy files
        for f in &jit_features {
            if f.entropy > 3.0 {
                findings.push(Finding::new(
                    &format!("PROC-ENT{:03}", findings.len() + 1),
                    SutraEngine::Process,
                    &f.file_path,
                    1,
                    &format!(
                        "High change entropy ({:.2}) — file is changed in highly distributed patterns across commits",
                        f.entropy
                    ),
                    Severity::Warning,
                ));
            }
        }

        // Frequently changed files (hotspots)
        let high_revision_count = jit_features
            .iter()
            .filter(|f| f.revisions > 20)
            .count();

        for f in &jit_features {
            if f.revisions > 20 {
                findings.push(Finding::new(
                    &format!("PROC-REV{:03}", findings.len() + 1 - (jit_features.iter().filter(|ff| ff.entropy > 3.0 && ff.revisions > 20).count()..).next().unwrap_or(0)),
                    SutraEngine::Process,
                    &f.file_path,
                    1,
                    &format!(
                        "Hotspot: {} revisions — file changes frequently, consider refactoring",
                        f.revisions
                    ),
                    Severity::Warning,
                ));
            }
        }

        // Bug-prone files
        let buggy: Vec<&JitFeatures> = jit_features
            .iter()
            .filter(|f| f.bug_fix_commits >= 3)
            .collect();

        for f in &buggy {
            findings.push(Finding::new(
                &format!("PROC-BUG{:03}", findings.len() + 1 - (jit_features.iter().filter(|ff| ff.revisions > 20 && ff.bug_fix_commits >= 3).count()..).next().unwrap_or(0)),
                SutraEngine::Process,
                &f.file_path,
                1,
                &format!(
                    "Bug-prone: {} bug-fix commits — high defect density",
                    f.bug_fix_commits
                ),
                Severity::Error,
            ));
        }

        // Tight co-change coupling
        let tight_coupling: Vec<_> = co_changes
            .edges
            .iter()
            .filter(|e| e.count as usize > commits.len() / 10)
            .collect();

        for edge in &tight_coupling {
            findings.push(Finding::new(
                &format!("PROC-COUPLE{:03}", findings.len() + 1),
                SutraEngine::Process,
                &edge.file_a,
                1,
                &format!(
                    "Tight coupling: '{}' co-changes with '{}' {} times",
                    edge.file_a, edge.file_b, edge.count
                ),
                Severity::Warning,
            ));
        }

        if !findings.is_empty() {
            let entropy_count = jit_features.iter().filter(|f| f.entropy > 3.0).count();

            if entropy_count > 0 {
                recommendations.push(Recommendation::new(
                    &format!(
                        "Refactor {} high-entropy file(s) — distribute changes more evenly or extract stable interfaces",
                        entropy_count
                    ),
                    0.8,
                ));
            }

            if high_revision_count > 0 {
                recommendations.push(Recommendation::new(
                    &format!("{} hotspot file(s) with >20 revisions — consider splitting or abstracting", high_revision_count),
                    0.7,
                ));
            }
            if !buggy.is_empty() {
                recommendations.push(Recommendation::new(
                    &format!(
                        "Prioritize {} bug-prone file(s) for testing and refactoring",
                        buggy.len()
                    ),
                    0.9,
                ));
            }
            if !tight_coupling.is_empty() {
                recommendations.push(Recommendation::new(
                    &format!(
                        "{} tightly coupled file pair(s) — consider merging or abstracting interface",
                        tight_coupling.len()
                    ),
                    0.6,
                ));
            }
        }

        if let Some(persist_path) = &self.persist_path {
            if let Ok(conn) = rusqlite::Connection::open(persist_path) {
                let aid = format!("proc-{}", request.request_id);
                let _ = persist::create_schema(&conn);
                let _ = persist::persist_commits(&conn, &commits, &aid);
                let _ = persist::persist_co_changes(&conn, &co_changes, &aid);
                let _ = persist::persist_jit_features(&conn, &jit_features, &aid);
            }
        }

        let unique_files: std::collections::HashSet<&str> = jit_features
            .iter()
            .map(|f| f.file_path.as_str())
            .collect();

        let max_entropy = jit_features
            .iter()
            .map(|f| f.entropy)
            .fold(0.0_f64, f64::max);

        let metrics = MetricsSummary {
            total_files: unique_files.len() as u32,
            ..Default::default()
        };

        // Set risk based on findings
        let error_count = findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count();
        let warning_count = findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count();

        let processing_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok(AnalysisResult {
            request_id: request.request_id.clone(),
            commit_hash: request.commit_hash.clone(),
            overall_risk: ((error_count as f64 * 0.3 + warning_count as f64 * 0.1) + max_entropy * 0.1)
                .min(1.0),
            findings,
            recommendations,
            metrics: Some(metrics),
            processing_time_ms,
            blocked_merge: error_count > 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_name() {
        let engine = ProcessEngine::new();
        assert_eq!(engine.name(), "process");
    }

    #[test]
    fn test_analyze_nonexistent_path() {
        let engine = ProcessEngine::new();
        let req = AnalyzeRequest::new("/nonexistent/repo", "abc");
        let result = engine.analyze(&req);
        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_with_config() {
        let engine = ProcessEngine::new().with_max_commits(500);
        assert_eq!(engine.max_commits, 500);
    }

    #[test]
    fn test_analyze_nonexistent_dir_graceful() {
        let engine = ProcessEngine::new();
        let req = AnalyzeRequest::new("/tmp/__sutra_nonexistent_dir_12345__", "abc");
        let result = engine.analyze(&req);
        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_empty_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let engine = ProcessEngine::new();
        let req = AnalyzeRequest::new(dir.path().to_str().unwrap(), "abc");
        let result = engine.analyze(&req);
        assert!(result.is_err());
    }
}
