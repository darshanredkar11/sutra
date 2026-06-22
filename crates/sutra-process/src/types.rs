use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub file_path: String,
    pub lines_added: u32,
    pub lines_deleted: u32,
    pub change_type: ChangeType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub author_email: String,
    pub timestamp_ms: i64,
    pub message: String,
    pub is_merge: bool,
    pub files_changed: Vec<FileChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoChange {
    pub file_a: String,
    pub file_b: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoChangeGraph {
    pub edges: Vec<CoChange>,
    // Maps file -> list of (co_changed_file, count)
    pub by_file: HashMap<String, Vec<(String, u32)>>,
}

impl CoChangeGraph {
    pub fn new() -> Self {
        Self {
            edges: Vec::new(),
            by_file: HashMap::new(),
        }
    }
}

impl Default for CoChangeGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitFeatures {
    pub file_path: String,
    /// NRevs — total revisions
    pub revisions: u32,
    /// NDistinctCommitters
    pub distinct_committers: u32,
    /// NAddedLines
    pub lines_added: u32,
    /// NDeletedLines
    pub lines_deleted: u32,
    /// NMod = added + deleted
    pub total_lines_changed: u32,
    /// Hassan entropy
    pub entropy: f64,
    /// NDir — directories modified in same commits
    pub num_directories: u32,
    /// NFiles — avg files per commit touching this file
    pub avg_files_per_commit: f64,
    /// Age in days since first change
    pub age_days: f64,
    /// WeightedAge
    pub weighted_age_days: f64,
    /// NRevsRecent — commits in last 30 days
    pub recent_commits: u32,
    /// NBugFixes — commits with fix keywords
    pub bug_fix_commits: u32,
    /// top contributor ratio
    pub owner_contribution: f64,
    /// NMinorContributors — contributors with <5% share
    pub minor_contributors: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessAnalysis {
    pub commits: Vec<CommitInfo>,
    pub co_changes: CoChangeGraph,
    pub jit_features: Vec<JitFeatures>,
    pub total_commits: usize,
    pub total_files: usize,
    pub analysis_duration_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_type_serde() {
        for ct in &[ChangeType::Added, ChangeType::Modified, ChangeType::Deleted, ChangeType::Renamed] {
            let json = serde_json::to_string(ct).unwrap();
            let back: ChangeType = serde_json::from_str(&json).unwrap();
            assert_eq!(*ct, back);
        }
    }

    #[test]
    fn test_co_change_graph_new() {
        let g = CoChangeGraph::new();
        assert!(g.edges.is_empty());
        assert!(g.by_file.is_empty());
    }

    #[test]
    fn test_file_change_serde() {
        let fc = FileChange {
            file_path: "src/main.rs".into(),
            lines_added: 10,
            lines_deleted: 2,
            change_type: ChangeType::Modified,
        };
        let json = serde_json::to_string(&fc).unwrap();
        let back: FileChange = serde_json::from_str(&json).unwrap();
        assert_eq!(fc.file_path, back.file_path);
    }

    #[test]
    fn test_jit_features_defaults() {
        let jf = JitFeatures {
            file_path: "test.py".into(),
            revisions: 0,
            distinct_committers: 0,
            lines_added: 0,
            lines_deleted: 0,
            total_lines_changed: 0,
            entropy: 0.0,
            num_directories: 0,
            avg_files_per_commit: 0.0,
            age_days: 0.0,
            weighted_age_days: 0.0,
            recent_commits: 0,
            bug_fix_commits: 0,
            owner_contribution: 0.0,
            minor_contributors: 0,
        };
        assert_eq!(jf.file_path, "test.py");
    }
}
