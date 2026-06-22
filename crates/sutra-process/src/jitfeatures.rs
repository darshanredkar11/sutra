use std::collections::{HashMap, HashSet};

use crate::types::{CommitInfo, JitFeatures};

/// Extract 14 JIT (Just-In-Time) defect prediction features for each file.
pub fn extract_jit_features(
    commits: &[CommitInfo],
    entropy_map: &HashMap<String, f64>,
    now_ms: i64,
) -> Vec<JitFeatures> {
    // Per-file data aggregation
    let mut revisions: HashMap<String, u32> = HashMap::new();
    let mut committers: HashMap<String, HashSet<String>> = HashMap::new();
    let mut lines_added: HashMap<String, u32> = HashMap::new();
    let mut lines_deleted: HashMap<String, u32> = HashMap::new();
    let mut commit_authors: HashMap<String, Vec<(String, u32, u32)>> = HashMap::new();
    let mut commit_timestamps: HashMap<String, Vec<i64>> = HashMap::new();
    let mut affected_dirs: HashMap<String, HashSet<String>> = HashMap::new();
    let mut files_per_commit: HashMap<String, Vec<u32>> = HashMap::new();
    let mut bug_fix_flags: HashMap<String, u32> = HashMap::new();
    let mut first_seen: HashMap<String, i64> = HashMap::new();

    let bug_keywords: &[&str] = &["fix", "bug", "crash", "defect", "error", "hotfix", "patch", "issue", "#"];

    for commit in commits {
        let file_count = commit.files_changed.len() as u32;
        let is_bug_fix = bug_keywords
            .iter()
            .any(|kw| commit.message.to_lowercase().contains(kw));

        for change in &commit.files_changed {
            let fp = &change.file_path;

            *revisions.entry(fp.clone()).or_insert(0) += 1;
            *lines_added.entry(fp.clone()).or_insert(0) += change.lines_added;
            *lines_deleted.entry(fp.clone()).or_insert(0) += change.lines_deleted;

            committers
                .entry(fp.clone())
                .or_default()
                .insert(commit.author.clone());

            commit_authors
                .entry(fp.clone())
                .or_default()
                .push((commit.author.clone(), change.lines_added, change.lines_deleted));

            commit_timestamps
                .entry(fp.clone())
                .or_default()
                .push(commit.timestamp_ms);

            files_per_commit
                .entry(fp.clone())
                .or_default()
                .push(file_count);

            if is_bug_fix {
                *bug_fix_flags.entry(fp.clone()).or_insert(0) += 1;
            }

            first_seen.entry(fp.clone()).or_insert(commit.timestamp_ms);

            // Track directories
            if let Some(parent) = std::path::Path::new(fp).parent() {
                let dir = parent.to_string_lossy().to_string();
                affected_dirs.entry(fp.clone()).or_default().insert(dir);
            }
        }
    }

    let all_file_paths: HashSet<String> = revisions.keys().cloned().collect();

    let mut features = Vec::new();

    for file_path in &all_file_paths {
        let revs = revisions.get(file_path).copied().unwrap_or(0);
        let distinct_committers = committers
            .get(file_path)
            .map(|s| s.len() as u32)
            .unwrap_or(0);
        let added = lines_added.get(file_path).copied().unwrap_or(0);
        let deleted = lines_deleted.get(file_path).copied().unwrap_or(0);
        let total_mod = added + deleted;

        let entropy = entropy_map.get(file_path).copied().unwrap_or(0.0);

        let num_dirs = affected_dirs
            .get(file_path)
            .map(|s| s.len() as u32)
            .unwrap_or(0);

        let fpc = files_per_commit.get(file_path).cloned().unwrap_or_default();
        let avg_files_per_commit = if fpc.is_empty() {
            0.0
        } else {
            fpc.iter().sum::<u32>() as f64 / fpc.len() as f64
        };

        let first_ts = first_seen.get(file_path).copied().unwrap_or(now_ms);
        let age_days = (now_ms - first_ts) as f64 / (24.0 * 60.0 * 60.0 * 1000.0);
        let age_days = age_days.max(0.0);

        // WeightedAge: weight recent changes more
        let timestamps = commit_timestamps.get(file_path).cloned().unwrap_or_default();
        let weighted_age_days = if timestamps.is_empty() {
            0.0
        } else {
            let weighted_sum: f64 = timestamps
                .iter()
                .map(|ts| {
                    let days_ago = (now_ms - ts) as f64 / (24.0 * 60.0 * 60.0 * 1000.0);
                    let weight = (-days_ago / 30.0).exp();
                    weight * days_ago
                })
                .sum();
            let total_weight: f64 = timestamps
                .iter()
                .map(|ts| {
                    let days_ago = (now_ms - ts) as f64 / (24.0 * 60.0 * 60.0 * 1000.0);
                    (-days_ago / 30.0).exp()
                })
                .sum();
            if total_weight > 0.0 {
                weighted_sum / total_weight
            } else {
                0.0
            }
        };

        // Recent commits (last 30 days)
        let recent_cutoff = now_ms - 30 * 24 * 60 * 60 * 1000;
        let recent = timestamps.iter().filter(|ts| **ts >= recent_cutoff).count() as u32;

        let bug_fixes = bug_fix_flags.get(file_path).copied().unwrap_or(0);

        // Owner contribution
        let authors = commit_authors.get(file_path).cloned().unwrap_or_default();
        let total_author_changes: u32 = authors.iter().map(|(_, a, d)| a + d).sum();
        let owner_contribution = if total_author_changes == 0 {
            0.0
        } else {
            let mut author_totals: HashMap<String, u32> = HashMap::new();
            for (name, a, d) in &authors {
                *author_totals.entry(name.clone()).or_insert(0) += a + d;
            }
            let top = author_totals.values().max().copied().unwrap_or(0);
            top as f64 / total_author_changes as f64
        };

        // Minor contributors (< 5% of changes)
        let minor = if total_author_changes == 0 {
            0
        } else {
            let threshold = total_author_changes as f64 * 0.05;
            let mut author_totals: HashMap<String, u32> = HashMap::new();
            for (name, a, d) in &authors {
                *author_totals.entry(name.clone()).or_insert(0) += a + d;
            }
            author_totals
                .values()
                .filter(|&&v| (v as f64) < threshold)
                .count() as u32
        };

        features.push(JitFeatures {
            file_path: file_path.clone(),
            revisions: revs,
            distinct_committers,
            lines_added: added,
            lines_deleted: deleted,
            total_lines_changed: total_mod,
            entropy,
            num_directories: num_dirs,
            avg_files_per_commit,
            age_days,
            weighted_age_days,
            recent_commits: recent,
            bug_fix_commits: bug_fixes,
            owner_contribution,
            minor_contributors: minor,
        });
    }

    features.sort_by(|a, b| b.revisions.cmp(&a.revisions));
    features
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChangeType, FileChange};

    fn commit(
        hash: &str,
        author: &str,
        ts_ms: i64,
        msg: &str,
        files: &[(&str, u32, u32)],
    ) -> CommitInfo {
        CommitInfo {
            hash: hash.into(),
            author: author.into(),
            author_email: "".into(),
            timestamp_ms: ts_ms,
            message: msg.into(),
            is_merge: false,
            files_changed: files
                .iter()
                .map(|(p, a, d)| FileChange {
                    file_path: p.to_string(),
                    lines_added: *a,
                    lines_deleted: *d,
                    change_type: ChangeType::Modified,
                })
                .collect(),
        }
    }

    #[test]
    fn test_empty_commits() {
        let features = extract_jit_features(&[], &HashMap::new(), 1000);
        assert!(features.is_empty());
    }

    #[test]
    fn test_single_file_single_commit() {
        let commits = vec![commit("c1", "alice", 1000, "init", &[("f.py", 10, 2)])];
        let entropy = HashMap::new();
        let features = extract_jit_features(&commits, &entropy, 2000);
        assert_eq!(features.len(), 1);
        assert_eq!(features[0].revisions, 1);
        assert_eq!(features[0].distinct_committers, 1);
        assert_eq!(features[0].lines_added, 10);
        assert_eq!(features[0].lines_deleted, 2);
    }

    #[test]
    fn test_multiple_revisions() {
        let commits = vec![
            commit("c1", "alice", 1000, "add", &[("f.py", 10, 0)]),
            commit("c2", "bob", 2000, "refactor", &[("f.py", 5, 3)]),
            commit("c3", "alice", 3000, "tweak", &[("f.py", 2, 1)]),
        ];
        let entropy = HashMap::new();
        let features = extract_jit_features(&commits, &entropy, 4000);
        assert_eq!(features.len(), 1);
        assert_eq!(features[0].revisions, 3);
        assert_eq!(features[0].distinct_committers, 2);
        assert_eq!(features[0].lines_added, 17);
        assert_eq!(features[0].lines_deleted, 4);
    }

    #[test]
    fn test_bug_fix_detection() {
        let commits = vec![
            commit("c1", "alice", 1000, "fix null pointer", &[("f.py", 2, 1)]),
            commit("c2", "alice", 2000, "refactor", &[("f.py", 5, 0)]),
        ];
        let entropy = HashMap::new();
        let features = extract_jit_features(&commits, &entropy, 3000);
        assert_eq!(features[0].bug_fix_commits, 1);
    }

    #[test]
    fn test_owner_contribution() {
        let commits = vec![
            commit("c1", "alice", 1000, "add", &[("f.py", 80, 0)]),
            commit("c2", "bob", 2000, "add", &[("f.py", 10, 0)]),
            commit("c3", "carol", 3000, "add", &[("f.py", 10, 0)]),
        ];
        let entropy = HashMap::new();
        let features = extract_jit_features(&commits, &entropy, 4000);
        assert!((features[0].owner_contribution - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_minor_contributors() {
        let commits = vec![
            commit("c1", "alice", 1000, "add", &[("f.py", 90, 0)]),
            commit("c2", "bob", 2000, "add", &[("f.py", 5, 0)]),
            commit("c3", "carol", 3000, "add", &[("f.py", 5, 0)]),
            commit("c4", "dave", 4000, "add", &[("f.py", 1, 0)]),
        ];
        let entropy = HashMap::new();
        let features = extract_jit_features(&commits, &entropy, 5000);
        assert_eq!(features[0].minor_contributors, 3);
    }

    #[test]
    fn test_recent_commits() {
        let now = 1000000i64;
        let day_ms = 24 * 60 * 60 * 1000;
        let commits = vec![
            commit("c1", "alice", now - 40 * day_ms, "old", &[("f.py", 5, 0)]),
            commit("c2", "alice", now - 10 * day_ms, "recent", &[("f.py", 3, 0)]),
            commit("c3", "alice", now - 5 * day_ms, "recent2", &[("f.py", 1, 0)]),
        ];
        let entropy = HashMap::new();
        let features = extract_jit_features(&commits, &entropy, now);
        assert_eq!(features[0].recent_commits, 2);
    }

    #[test]
    fn test_age_days() {
        let now = 1000000000000i64;
        let day_ms = 24 * 60 * 60 * 1000;
        let commits = vec![
            commit("c1", "alice", now - 100 * day_ms, "old", &[("f.py", 5, 0)]),
        ];
        let entropy = HashMap::new();
        let features = extract_jit_features(&commits, &entropy, now);
        assert!((features[0].age_days - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_multiple_files_sorted() {
        let commits = vec![
            commit("c1", "alice", 1000, "add", &[("a.py", 10, 0), ("b.py", 5, 0)]),
            commit("c2", "alice", 2000, "add", &[("a.py", 3, 0)]),
        ];
        let entropy = HashMap::new();
        let features = extract_jit_features(&commits, &entropy, 3000);
        assert_eq!(features.len(), 2);
        // Highest revision count first
        assert_eq!(features[0].file_path, "a.py");
        assert_eq!(features[0].revisions, 2);
    }

    #[test]
    fn test_avg_files_per_commit() {
        let commits = vec![
            commit("c1", "alice", 1000, "add", &[("a.py", 10, 0), ("b.py", 5, 0)]),
            commit("c2", "alice", 2000, "add", &[("a.py", 3, 0)]),
        ];
        let entropy = HashMap::new();
        let features = extract_jit_features(&commits, &entropy, 3000);
        // a.py was in commit with 2 files and commit with 1 file => avg = 1.5
        assert!((features[0].avg_files_per_commit - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_entropy_incorporated() {
        let mut entropy = HashMap::new();
        entropy.insert("f.py".into(), 1.5);
        let commits = vec![commit("c1", "alice", 1000, "add", &[("f.py", 10, 0)])];
        let features = extract_jit_features(&commits, &entropy, 2000);
        assert!((features[0].entropy - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_single_commit_age_and_owner() {
        let commits = vec![commit("c1", "alice", 5000, "init", &[("f.py", 10, 2)])];
        let entropy = HashMap::new();
        let features = extract_jit_features(&commits, &entropy, 5000);
        assert!((features[0].age_days - 0.0).abs() < 1e-6);
        assert!((features[0].owner_contribution - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_hundred_authors() {
        let commits: Vec<CommitInfo> = (0..100)
            .map(|i| commit(
                &format!("c{}", i),
                &format!("author_{}", i),
                1000 + i * 100,
                "change",
                &[("f.py", 1, 0)],
            ))
            .collect();
        let entropy = HashMap::new();
        let features = extract_jit_features(&commits, &entropy, 200000);
        assert_eq!(features[0].distinct_committers, 100);
    }

    #[test]
    fn test_bug_fix_all_keywords() {
        let keywords = &["fix", "bug", "crash", "defect", "error", "hotfix", "patch", "issue"];
        let commits: Vec<CommitInfo> = keywords
            .iter()
            .enumerate()
            .map(|(i, kw)| commit(
                &format!("c{}", i),
                "alice",
                1000 + i as i64 * 100,
                kw,
                &[("f.py", 1, 0)],
            ))
            .collect();
        let entropy = HashMap::new();
        let features = extract_jit_features(&commits, &entropy, 200000);
        assert_eq!(features[0].bug_fix_commits, keywords.len() as u32);
    }
}
