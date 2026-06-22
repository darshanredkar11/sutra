use std::collections::HashMap;

use crate::types::CommitInfo;

/// Compute Hassan code change entropy for each file.
///
/// For a file `f` across `n` commits:
/// - p_i = lines_changed_in_commit_i / total_lines_changed
/// - H(f) = -sum(p_i * log2(p_i))
///
/// Returns map of file_path -> entropy value (0.0 for single-change files).
pub fn compute_entropy(commits: &[CommitInfo]) -> HashMap<String, f64> {
    // Collect per-file: Vec<(timestamp, lines_changed)>
    let mut file_changes: HashMap<String, Vec<(i64, u32)>> = HashMap::new();

    for commit in commits {
        for change in &commit.files_changed {
            let total = change.lines_added + change.lines_deleted;
            if total == 0 {
                continue;
            }
            file_changes
                .entry(change.file_path.clone())
                .or_default()
                .push((commit.timestamp_ms, total));
        }
    }

    let mut entropy_map = HashMap::new();

    for (file_path, changes) in &file_changes {
        if changes.len() <= 1 {
            entropy_map.insert(file_path.clone(), 0.0);
            continue;
        }

        let total: u32 = changes.iter().map(|(_, c)| c).sum();
        if total == 0 {
            entropy_map.insert(file_path.clone(), 0.0);
            continue;
        }

        let entropy: f64 = changes
            .iter()
            .map(|(_, c)| {
                let p = *c as f64 / total as f64;
                if p <= 0.0 {
                    0.0
                } else {
                    -p * p.log2()
                }
            })
            .sum();

        entropy_map.insert(file_path.clone(), entropy);
    }

    entropy_map
}

/// Compute entropy over a sliding time window (e.g., last 90 days).
pub fn compute_entropy_window(
    commits: &[CommitInfo],
    window_days: i64,
) -> HashMap<String, f64> {
    if commits.is_empty() {
        return HashMap::new();
    }

    let latest = commits.iter().map(|c| c.timestamp_ms).max().unwrap_or(0);
    let cutoff = latest - (window_days * 24 * 60 * 60 * 1000);

    let recent_commits: Vec<&CommitInfo> = commits
        .iter()
        .filter(|c| c.timestamp_ms >= cutoff)
        .collect();

    let owned: Vec<CommitInfo> = recent_commits.into_iter().cloned().collect();
    compute_entropy(&owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChangeType, FileChange};

    fn make_commit(hash: &str, ts_ms: i64, files: &[(&str, u32, u32)]) -> CommitInfo {
        CommitInfo {
            hash: hash.into(),
            author: "t".into(),
            author_email: "t@t".into(),
            timestamp_ms: ts_ms,
            message: "wip".into(),
            is_merge: false,
            files_changed: files
                .iter()
                .map(|(path, added, deleted)| FileChange {
                    file_path: path.to_string(),
                    lines_added: *added,
                    lines_deleted: *deleted,
                    change_type: ChangeType::Modified,
                })
                .collect(),
        }
    }

    #[test]
    fn test_empty_commits() {
        let e = compute_entropy(&[]);
        assert!(e.is_empty());
    }

    #[test]
    fn test_single_change_zero_entropy() {
        let commits = vec![make_commit("c1", 1000, &[("f.py", 10, 0)])];
        let e = compute_entropy(&commits);
        assert!((e["f.py"] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_two_equal_changes() {
        let commits = vec![
            make_commit("c1", 1000, &[("f.py", 5, 0)]),
            make_commit("c2", 2000, &[("f.py", 5, 0)]),
        ];
        let e = compute_entropy(&commits);
        // p1 = p2 = 0.5, H = -(0.5*log2(0.5) + 0.5*log2(0.5)) = -(-0.5 + -0.5) = 1.0
        assert!((e["f.py"] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_three_changes_max_entropy() {
        let commits = vec![
            make_commit("c1", 1000, &[("f.py", 1, 0)]),
            make_commit("c2", 2000, &[("f.py", 1, 0)]),
            make_commit("c3", 3000, &[("f.py", 1, 0)]),
        ];
        let e = compute_entropy(&commits);
        // p_i = 1/3 each
        let h: f64 = -3.0 * (1.0_f64 / 3.0) * (1.0_f64 / 3.0).log2();
        assert!((e["f.py"] - h).abs() < 1e-10);
    }

    #[test]
    fn test_uneven_distribution_lower_entropy() {
        let commits = vec![
            make_commit("c1", 1000, &[("f.py", 90, 0)]),
            make_commit("c2", 2000, &[("f.py", 5, 0)]),
            make_commit("c3", 3000, &[("f.py", 5, 0)]),
        ];
        let e = compute_entropy(&commits);
        let p1: f64 = 90.0 / 100.0;
        let p2: f64 = 5.0 / 100.0;
        let p3: f64 = 5.0 / 100.0;
        let h: f64 = -(p1 * p1.log2() + p2 * p2.log2() + p3 * p3.log2());
        assert!((e["f.py"] - h).abs() < 1e-10);
        // Should be less than max entropy (log2(3) ≈ 1.585)
        assert!(e["f.py"] < 1.5);
    }

    #[test]
    fn test_zero_line_changes_skipped() {
        let commits = vec![
            make_commit("c1", 1000, &[("f.py", 0, 0)]),
        ];
        let e = compute_entropy(&commits);
        assert!(e.is_empty());
    }

    #[test]
    fn test_multiple_files() {
        let commits = vec![
            make_commit("c1", 1000, &[("a.py", 10, 0), ("b.py", 5, 0)]),
            make_commit("c2", 2000, &[("a.py", 10, 0)]),
        ];
        let e = compute_entropy(&commits);
        assert!(e.contains_key("a.py"));
        assert!(e.contains_key("b.py"));
        // b.py has only 1 change => entropy 0
        assert!((e["b.py"] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_window_empty() {
        let e = compute_entropy_window(&[], 90);
        assert!(e.is_empty());
    }

    #[test]
    fn test_window_filters_old() {
        let now = 1000000000000i64;
        let day_ms = 24 * 60 * 60 * 1000;
        let commits = vec![
            make_commit("old", now - 200 * day_ms, &[("f.py", 10, 0)]),
            make_commit("new", now - 10 * day_ms, &[("f.py", 10, 0)]),
        ];
        let e = compute_entropy_window(&commits, 90);
        assert!(e.contains_key("f.py"));
        // Only 1 commit in window => 0 entropy
        assert!((e["f.py"] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_single_change_per_file() {
        let commits = vec![
            make_commit("c1", 1000, &[("a.py", 10, 0)]),
            make_commit("c2", 2000, &[("b.py", 20, 0)]),
            make_commit("c3", 3000, &[("c.py", 5, 5)]),
        ];
        let e = compute_entropy(&commits);
        assert!((e["a.py"] - 0.0).abs() < 1e-10);
        assert!((e["b.py"] - 0.0).abs() < 1e-10);
        assert!((e["c.py"] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_uniform_distribution() {
        let n = 4;
        let commits: Vec<CommitInfo> = (0..n)
            .map(|i| make_commit(&format!("c{}", i), i as i64 * 1000, &[("f.py", 10, 0)]))
            .collect();
        let e = compute_entropy(&commits);
        let expected = (n as f64).log2();
        assert!((e["f.py"] - expected).abs() < 1e-10);
    }

    #[test]
    fn test_window_no_commits_in_window() {
        let e = compute_entropy_window(&[], 90);
        assert!(e.is_empty());
    }

    #[test]
    fn test_window_all_commits_in_window() {
        let now = 5000000i64;
        let commits = vec![
            make_commit("c1", now - 1000, &[("f.py", 5, 0)]),
            make_commit("c2", now - 500, &[("f.py", 5, 0)]),
        ];
        let full = compute_entropy(&commits);
        let windowed = compute_entropy_window(&commits, 365 * 100);
        assert_eq!(full.len(), windowed.len());
        assert!((windowed["f.py"] - full["f.py"]).abs() < 1e-10);
    }
}
