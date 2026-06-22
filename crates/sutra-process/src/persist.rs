use rusqlite::{params, Connection};

use crate::types::{ChangeType, CoChangeGraph, CommitInfo, JitFeatures};

pub fn create_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS process_commits (
            row_id INTEGER PRIMARY KEY AUTOINCREMENT,
            hash TEXT NOT NULL,
            author TEXT NOT NULL,
            author_email TEXT NOT NULL,
            timestamp_ms INTEGER NOT NULL,
            message TEXT NOT NULL,
            is_merge INTEGER NOT NULL DEFAULT 0,
            analysis_id TEXT NOT NULL,
            UNIQUE(hash, analysis_id)
        );
        CREATE TABLE IF NOT EXISTS process_file_changes (
            row_id INTEGER PRIMARY KEY AUTOINCREMENT,
            commit_hash TEXT NOT NULL,
            file_path TEXT NOT NULL,
            lines_added INTEGER NOT NULL,
            lines_deleted INTEGER NOT NULL,
            change_type TEXT NOT NULL,
            analysis_id TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS process_co_changes (
            row_id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_a TEXT NOT NULL,
            file_b TEXT NOT NULL,
            co_change_count INTEGER NOT NULL,
            analysis_id TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS process_jit_features (
            row_id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            revisions INTEGER NOT NULL,
            distinct_committers INTEGER NOT NULL,
            lines_added INTEGER NOT NULL,
            lines_deleted INTEGER NOT NULL,
            total_lines_changed INTEGER NOT NULL,
            entropy REAL NOT NULL,
            num_directories INTEGER NOT NULL,
            avg_files_per_commit REAL NOT NULL,
            age_days REAL NOT NULL,
            weighted_age_days REAL NOT NULL,
            recent_commits INTEGER NOT NULL,
            bug_fix_commits INTEGER NOT NULL,
            owner_contribution REAL NOT NULL,
            minor_contributors INTEGER NOT NULL,
            analysis_id TEXT NOT NULL,
            UNIQUE(file_path, analysis_id)
        );
        CREATE INDEX IF NOT EXISTS idx_process_commits_analysis ON process_commits(analysis_id);
        CREATE INDEX IF NOT EXISTS idx_process_changes_analysis ON process_file_changes(analysis_id);
        CREATE INDEX IF NOT EXISTS idx_process_jit_analysis ON process_jit_features(analysis_id);
        ",
    )
    .map_err(|e| format!("create schema: {}", e))?;
    Ok(())
}

pub fn persist_commits(
    conn: &Connection,
    commits: &[CommitInfo],
    analysis_id: &str,
) -> Result<(), String> {
    let mut insert_commit = conn
        .prepare(
            "INSERT OR IGNORE INTO process_commits (hash, author, author_email, timestamp_ms, message, is_merge, analysis_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .map_err(|e| format!("prepare commit: {}", e))?;

    let mut insert_change = conn
        .prepare(
            "INSERT OR IGNORE INTO process_file_changes (commit_hash, file_path, lines_added, lines_deleted, change_type, analysis_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|e| format!("prepare change: {}", e))?;

    for commit in commits {
        insert_commit
            .execute(params![
                commit.hash,
                commit.author,
                commit.author_email,
                commit.timestamp_ms,
                commit.message,
                commit.is_merge as i32,
                analysis_id,
            ])
            .map_err(|e| format!("insert commit: {}", e))?;

        for change in &commit.files_changed {
            let ct = match change.change_type {
                ChangeType::Added => "added",
                ChangeType::Modified => "modified",
                ChangeType::Deleted => "deleted",
                ChangeType::Renamed => "renamed",
            };
            insert_change
                .execute(params![
                    commit.hash,
                    change.file_path,
                    change.lines_added,
                    change.lines_deleted,
                    ct,
                    analysis_id,
                ])
                .map_err(|e| format!("insert change: {}", e))?;
        }
    }

    Ok(())
}

pub fn persist_co_changes(
    conn: &Connection,
    co_changes: &CoChangeGraph,
    analysis_id: &str,
) -> Result<(), String> {
    let mut insert = conn
        .prepare(
            "INSERT OR IGNORE INTO process_co_changes (file_a, file_b, co_change_count, analysis_id) VALUES (?1, ?2, ?3, ?4)",
        )
        .map_err(|e| format!("prepare cochange: {}", e))?;

    for edge in &co_changes.edges {
        insert
            .execute(params![edge.file_a, edge.file_b, edge.count, analysis_id])
            .map_err(|e| format!("insert cochange: {}", e))?;
    }

    Ok(())
}

pub fn persist_jit_features(
    conn: &Connection,
    features: &[JitFeatures],
    analysis_id: &str,
) -> Result<(), String> {
    let mut insert = conn
        .prepare(
            "INSERT OR REPLACE INTO process_jit_features (file_path, revisions, distinct_committers, lines_added, lines_deleted, total_lines_changed, entropy, num_directories, avg_files_per_commit, age_days, weighted_age_days, recent_commits, bug_fix_commits, owner_contribution, minor_contributors, analysis_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        )
        .map_err(|e| format!("prepare jit: {}", e))?;

    for f in features {
        insert
            .execute(params![
                f.file_path,
                f.revisions,
                f.distinct_committers,
                f.lines_added,
                f.lines_deleted,
                f.total_lines_changed,
                f.entropy,
                f.num_directories,
                f.avg_files_per_commit,
                f.age_days,
                f.weighted_age_days,
                f.recent_commits,
                f.bug_fix_commits,
                f.owner_contribution,
                f.minor_contributors,
                analysis_id,
            ])
            .map_err(|e| format!("insert jit: {}", e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChangeType, CoChange, FileChange};

    fn sample_commits() -> Vec<CommitInfo> {
        vec![
            CommitInfo {
                hash: "abc123".into(),
                author: "alice".into(),
                author_email: "alice@test.com".into(),
                timestamp_ms: 1000,
                message: "initial commit".into(),
                is_merge: false,
                files_changed: vec![FileChange {
                    file_path: "src/main.py".into(),
                    lines_added: 20,
                    lines_deleted: 0,
                    change_type: ChangeType::Added,
                }],
            },
            CommitInfo {
                hash: "def456".into(),
                author: "bob".into(),
                author_email: "bob@test.com".into(),
                timestamp_ms: 2000,
                message: "fix bug".into(),
                is_merge: false,
                files_changed: vec![
                    FileChange {
                        file_path: "src/main.py".into(),
                        lines_added: 5,
                        lines_deleted: 2,
                        change_type: ChangeType::Modified,
                    },
                    FileChange {
                        file_path: "src/utils.py".into(),
                        lines_added: 10,
                        lines_deleted: 0,
                        change_type: ChangeType::Added,
                    },
                ],
            },
        ]
    }

    #[test]
    fn test_create_and_persist_commits() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        let commits = sample_commits();
        persist_commits(&conn, &commits, "test-001").unwrap();
    }

    #[test]
    fn test_persist_co_changes() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        let graph = CoChangeGraph {
            edges: vec![CoChange {
                file_a: "a.py".into(),
                file_b: "b.py".into(),
                count: 3,
            }],
            by_file: std::collections::HashMap::new(),
        };
        persist_co_changes(&conn, &graph, "test-002").unwrap();
    }

    #[test]
    fn test_persist_jit_features() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        let features = vec![JitFeatures {
            file_path: "src/main.py".into(),
            revisions: 10,
            distinct_committers: 3,
            lines_added: 100,
            lines_deleted: 20,
            total_lines_changed: 120,
            entropy: 2.5,
            num_directories: 2,
            avg_files_per_commit: 1.5,
            age_days: 30.0,
            weighted_age_days: 15.0,
            recent_commits: 2,
            bug_fix_commits: 1,
            owner_contribution: 0.7,
            minor_contributors: 2,
        }];
        persist_jit_features(&conn, &features, "test-003").unwrap();
    }

    #[test]
    fn test_multiple_analyses() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        persist_commits(&conn, &sample_commits(), "analysis-1").unwrap();

        let mut commits2 = sample_commits();
        let _ = &mut commits2[0].files_changed.push(FileChange {
            file_path: "extra.py".into(),
            lines_added: 1,
            lines_deleted: 0,
            change_type: ChangeType::Added,
        });
        persist_commits(&conn, &commits2, "analysis-2").unwrap();
    }

    #[test]
    fn test_persist_empty_features() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        persist_jit_features(&conn, &[], "empty-test").unwrap();
    }

    #[test]
    fn test_persist_extreme_floats() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        let features = vec![JitFeatures {
            file_path: "extreme.bin".into(),
            revisions: 1,
            distinct_committers: 1,
            lines_added: 0,
            lines_deleted: 0,
            total_lines_changed: 0,
            entropy: 1e300,
            num_directories: 0,
            avg_files_per_commit: -1e300,
            age_days: 1e-300,
            weighted_age_days: 0.0,
            recent_commits: 0,
            bug_fix_commits: 0,
            owner_contribution: 1.0,
            minor_contributors: 0,
        }];
        persist_jit_features(&conn, &features, "extreme-test").unwrap();
        let mut stmt = conn
            .prepare("SELECT entropy, avg_files_per_commit FROM process_jit_features WHERE analysis_id = ?1")
            .unwrap();
        let rows = stmt
            .query_map(params!["extreme-test"], |row| {
                Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert!((rows[0].0 - 1e300).abs() < 1e290);
        assert!((rows[0].1 - -1e300).abs() < 1e290);
    }

    #[test]
    fn test_persist_unicode_paths() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        let path = "日本語/파일/文件.py";
        let features = vec![JitFeatures {
            file_path: path.into(),
            revisions: 5,
            distinct_committers: 2,
            lines_added: 10,
            lines_deleted: 3,
            total_lines_changed: 13,
            entropy: 1.0,
            num_directories: 1,
            avg_files_per_commit: 1.0,
            age_days: 10.0,
            weighted_age_days: 5.0,
            recent_commits: 1,
            bug_fix_commits: 0,
            owner_contribution: 0.8,
            minor_contributors: 1,
        }];
        persist_jit_features(&conn, &features, "unicode-test").unwrap();
        let mut stmt = conn
            .prepare("SELECT file_path FROM process_jit_features WHERE analysis_id = ?1")
            .unwrap();
        let stored: String = stmt
            .query_row(params!["unicode-test"], |row| row.get(0))
            .unwrap();
        assert_eq!(stored, path);
    }
}
