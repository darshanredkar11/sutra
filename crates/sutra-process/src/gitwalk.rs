use git2::{DiffOptions, Repository, Sort};

use crate::types::{ChangeType, CommitInfo, FileChange};

pub fn open_repo(path: &str) -> Result<Repository, String> {
    Repository::open(path).map_err(|e| format!("failed to open git repo '{}': {}", path, e))
}

pub fn walk_commits(repo: &Repository, max_commits: usize) -> Result<Vec<CommitInfo>, String> {
    let mut revwalk = repo.revwalk().map_err(|e| format!("revwalk: {}", e))?;
    revwalk
        .set_sorting(Sort::TIME | Sort::TOPOLOGICAL)
        .map_err(|e| format!("sort: {}", e))?;
    revwalk.push_head().map_err(|e| format!("push head: {}", e))?;

    let mut commits = Vec::new();
    for oid_result in revwalk {
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => continue,
        };
        if commits.len() >= max_commits {
            break;
        }
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let timestamp =
            commit.time().seconds() * 1000 + (commit.time().offset_minutes() as i64 * 60 * 1000);
        let author = commit.author();
        let message = commit.message().unwrap_or("").to_string();
        let is_merge = commit.parent_count() > 1;
        let files_changed = extract_changes(repo, &commit);

        commits.push(CommitInfo {
            hash: oid.to_string(),
            author: author.name().unwrap_or("unknown").to_string(),
            author_email: author.email().unwrap_or("").to_string(),
            timestamp_ms: timestamp,
            message,
            is_merge,
            files_changed,
        });
    }
    Ok(commits)
}

pub fn walk_all_commits(repo: &Repository) -> Result<Vec<CommitInfo>, String> {
    walk_commits(repo, usize::MAX)
}

fn extract_changes(repo: &Repository, commit: &git2::Commit) -> Vec<FileChange> {
    let tree = match commit.tree() {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    let parent_tree = if commit.parent_count() > 0 {
        commit.parent(0).ok().and_then(|p| p.tree().ok())
    } else {
        None
    };
    let mut diff_opts = DiffOptions::new();
    diff_opts.ignore_submodules(true);
    let diff = match repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))
    {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    let total_added = diff
        .stats()
        .ok()
        .map(|s| s.insertions() as u32)
        .unwrap_or(0);
    let total_deleted = diff
        .stats()
        .ok()
        .map(|s| s.deletions() as u32)
        .unwrap_or(0);
    let delta_count = diff.deltas().len() as u32;

    let mut changes = Vec::new();
    for delta in diff.deltas() {
        let file_path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        if file_path.is_empty() {
            continue;
        }
        let change_type = match delta.status() {
            git2::Delta::Added => ChangeType::Added,
            git2::Delta::Deleted => ChangeType::Deleted,
            git2::Delta::Renamed | git2::Delta::Copied => ChangeType::Renamed,
            _ => ChangeType::Modified,
        };
        let (added, deleted) = if delta_count > 0 {
            (total_added / delta_count, total_deleted / delta_count)
        } else {
            (0, 0)
        };
        changes.push(FileChange {
            file_path,
            lines_added: added,
            lines_deleted: deleted,
            change_type,
        });
    }
    // If we have only one file, assign exact stats
    if changes.len() == 1 && total_added > 0 {
        changes[0].lines_added = total_added;
        changes[0].lines_deleted = total_deleted;
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        let sig = git2::Signature::now("test", "test@test.com").unwrap();

        let path = dir.path().join("README.md");
        fs::write(&path, b"# Hello\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("README.md")).unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
        drop(tree);

        fs::write(dir.path().join("README.md"), b"# Hello World\n").unwrap();
        fs::write(dir.path().join("main.py"), b"print('hello')\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("README.md")).unwrap();
        index.add_path(std::path::Path::new("main.py")).unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree2 = repo.find_tree(tree_id).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "add main.py", &tree2, &[&parent])
            .unwrap();
        drop(tree2);
        drop(parent);

        (dir, repo)
    }

    #[test]
    fn test_open_repo_valid() {
        let (_dir, repo) = create_test_repo();
        assert!(!repo.path().to_string_lossy().is_empty());
    }

    #[test]
    fn test_open_repo_invalid_path() {
        let result = open_repo("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_walk_commits() {
        let (_dir, repo) = create_test_repo();
        let commits = walk_all_commits(&repo).unwrap();
        assert_eq!(commits.len(), 2);
    }

    #[test]
    fn test_walk_commits_limited() {
        let (_dir, repo) = create_test_repo();
        let commits = walk_commits(&repo, 1).unwrap();
        assert_eq!(commits.len(), 1);
    }

    #[test]
    fn test_commit_hashes_sha1() {
        let (_dir, repo) = create_test_repo();
        let commits = walk_all_commits(&repo).unwrap();
        assert_eq!(commits[0].hash.len(), 40);
    }

    #[test]
    fn test_commit_messages() {
        let (_dir, repo) = create_test_repo();
        let commits = walk_all_commits(&repo).unwrap();
        assert!(commits[0].message.contains("main.py"));
        assert!(commits[1].message.contains("initial"));
    }

    #[test]
    fn test_commit_files() {
        let (_dir, repo) = create_test_repo();
        let commits = walk_all_commits(&repo).unwrap();
        // Second commit touches 2 files
        assert_eq!(commits[0].files_changed.len(), 2);
        // First commit touches 1 file
        assert_eq!(commits[1].files_changed.len(), 1);
    }

    #[test]
    fn test_commit_author() {
        let (_dir, repo) = create_test_repo();
        let commits = walk_all_commits(&repo).unwrap();
        assert_eq!(commits[0].author, "test");
        assert_eq!(commits[0].author_email, "test@test.com");
    }

    #[test]
    fn test_empty_repo_fails() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        // No commits, revwalk should fail
        let result = walk_all_commits(&repo);
        assert!(result.is_err());
    }

    #[test]
    fn test_commit_timestamps() {
        let (_dir, repo) = create_test_repo();
        let commits = walk_all_commits(&repo).unwrap();
        assert!(commits[0].timestamp_ms > 0);
    }
}
