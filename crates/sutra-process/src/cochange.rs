use std::collections::HashMap;

use crate::types::{CoChange, CoChangeGraph, CommitInfo};

pub fn build_co_change_graph(commits: &[CommitInfo]) -> CoChangeGraph {
    let mut pair_counts: HashMap<(String, String), u32> = HashMap::new();
    let mut by_file: HashMap<String, Vec<(String, u32)>> = HashMap::new();

    for commit in commits {
        let files: Vec<&str> = commit
            .files_changed
            .iter()
            .map(|f| f.file_path.as_str())
            .collect();

        if files.len() < 2 {
            continue;
        }

        for i in 0..files.len() {
            for j in (i + 1)..files.len() {
                let (a, b) = if files[i] < files[j] {
                    (files[i].to_string(), files[j].to_string())
                } else {
                    (files[j].to_string(), files[i].to_string())
                };

                *pair_counts.entry((a.clone(), b.clone())).or_insert(0) += 1;
            }
        }
    }

    let edges: Vec<CoChange> = pair_counts
        .into_iter()
        .map(|((file_a, file_b), count)| CoChange { file_a, file_b, count })
        .collect();

    for edge in &edges {
        by_file
            .entry(edge.file_a.clone())
            .or_default()
            .push((edge.file_b.clone(), edge.count));
        by_file
            .entry(edge.file_b.clone())
            .or_default()
            .push((edge.file_a.clone(), edge.count));
    }

    for v in by_file.values_mut() {
        v.sort_by(|a, b| b.1.cmp(&a.1));
    }

    CoChangeGraph { edges, by_file }
}

pub fn top_co_changes(graph: &CoChangeGraph, n: usize) -> Vec<CoChange> {
    let mut sorted = graph.edges.clone();
    sorted.sort_by(|a, b| b.count.cmp(&a.count));
    sorted.truncate(n);
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChangeType, FileChange};

    fn make_commit(files: &[&str]) -> CommitInfo {
        CommitInfo {
            hash: "abc".into(),
            author: "t".into(),
            author_email: "t@t".into(),
            timestamp_ms: 1000,
            message: "msg".into(),
            is_merge: false,
            files_changed: files
                .iter()
                .map(|f| FileChange {
                    file_path: f.to_string(),
                    lines_added: 1,
                    lines_deleted: 0,
                    change_type: ChangeType::Modified,
                })
                .collect(),
        }
    }

    #[test]
    fn test_empty_commits() {
        let graph = build_co_change_graph(&[]);
        assert!(graph.edges.is_empty());
        assert!(graph.by_file.is_empty());
    }

    #[test]
    fn test_single_file_no_co_change() {
        let commits = vec![make_commit(&["a.py"])];
        let graph = build_co_change_graph(&commits);
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_simple_co_change() {
        let commits = vec![make_commit(&["a.py", "b.py"])];
        let graph = build_co_change_graph(&commits);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].count, 1);
    }

    #[test]
    fn test_multiple_co_changes() {
        let commits = vec![
            make_commit(&["a.py", "b.py"]),
            make_commit(&["a.py", "b.py"]),
            make_commit(&["a.py", "c.py"]),
        ];
        let graph = build_co_change_graph(&commits);
        assert_eq!(graph.edges.len(), 2);
        let a_b = graph.edges.iter().find(|e| e.file_a == "a.py" && e.file_b == "b.py").unwrap();
        assert_eq!(a_b.count, 2);
        let a_c = graph.edges.iter().find(|e| e.file_a == "a.py" && e.file_b == "c.py").unwrap();
        assert_eq!(a_c.count, 1);
    }

    #[test]
    fn test_ordering_normalized() {
        let commits = vec![make_commit(&["b.py", "a.py"])];
        let graph = build_co_change_graph(&commits);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].file_a, "a.py");
        assert_eq!(graph.edges[0].file_b, "b.py");
    }

    #[test]
    fn test_by_file_lookup() {
        let commits = vec![
            make_commit(&["a.py", "b.py"]),
            make_commit(&["a.py", "c.py"]),
        ];
        let graph = build_co_change_graph(&commits);
        let a_co = graph.by_file.get("a.py").unwrap();
        assert_eq!(a_co.len(), 2);
    }

    #[test]
    fn test_top_co_changes() {
        let commits = vec![
            make_commit(&["a.py", "b.py"]),
            make_commit(&["a.py", "b.py"]),
            make_commit(&["a.py", "b.py"]),
            make_commit(&["a.py", "c.py"]),
            make_commit(&["c.py", "b.py"]),
        ];
        let graph = build_co_change_graph(&commits);
        let top = top_co_changes(&graph, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].count, 3); // a.py-b.py is most frequent
    }

    #[test]
    fn test_two_files_always_together() {
        let n = 5;
        let commits: Vec<CommitInfo> = (0..n)
            .map(|i| {
                let mut c = make_commit(&["x.py", "y.py"]);
                c.hash = format!("c{}", i);
                c
            })
            .collect();
        let graph = build_co_change_graph(&commits);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].count, n);
    }
}
