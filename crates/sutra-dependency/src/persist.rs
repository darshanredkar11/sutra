use rusqlite::{params, Connection};

use crate::types::{DepEdge, DepNode, DependencyGraph, ImportKind};

pub fn create_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = OFF;
        CREATE TABLE IF NOT EXISTS dep_nodes (
            row_id INTEGER PRIMARY KEY AUTOINCREMENT,
            id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            module_name TEXT NOT NULL,
            language TEXT NOT NULL,
            analysis_id TEXT NOT NULL,
            UNIQUE(id, analysis_id)
        );
        CREATE TABLE IF NOT EXISTS dep_edges (
            row_id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id TEXT NOT NULL,
            target_id TEXT NOT NULL,
            line INTEGER NOT NULL,
            kind TEXT NOT NULL,
            analysis_id TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_edges_source ON dep_edges(source_id);
        CREATE INDEX IF NOT EXISTS idx_edges_target ON dep_edges(target_id);
        CREATE INDEX IF NOT EXISTS idx_nodes_analysis ON dep_nodes(analysis_id);
        CREATE INDEX IF NOT EXISTS idx_edges_analysis ON dep_edges(analysis_id);
        ",
    )
    .map_err(|e| format!("failed to create schema: {}", e))?;
    Ok(())
}

pub fn persist_graph(
    conn: &Connection,
    graph: &DependencyGraph,
    analysis_id: &str,
) -> Result<(), String> {
    let mut insert_node = conn
        .prepare("INSERT OR IGNORE INTO dep_nodes (id, file_path, module_name, language, analysis_id) VALUES (?1, ?2, ?3, ?4, ?5)")
        .map_err(|e| format!("prepare node: {}", e))?;

    for node in &graph.nodes {
        insert_node
            .execute(params![
                node.id,
                node.file_path,
                node.module_name,
                node.language,
                analysis_id,
            ])
            .map_err(|e| format!("insert node: {}", e))?;
    }

    let mut insert_edge = conn
        .prepare("INSERT INTO dep_edges (source_id, target_id, line, kind, analysis_id) VALUES (?1, ?2, ?3, ?4, ?5)")
        .map_err(|e| format!("prepare edge: {}", e))?;

    for edge in &graph.edges {
        let kind_str = match edge.kind {
            ImportKind::Static => "static",
            ImportKind::Dynamic => "dynamic",
            ImportKind::ReExport => "reexport",
        };
        insert_edge
            .execute(params![
                edge.source_id,
                edge.target_id,
                edge.line,
                kind_str,
                analysis_id,
            ])
            .map_err(|e| format!("insert edge: {}", e))?;
    }

    Ok(())
}

pub fn load_graph(conn: &Connection, analysis_id: &str) -> Result<DependencyGraph, String> {
    let mut node_stmt = conn
        .prepare("SELECT id, file_path, module_name, language FROM dep_nodes WHERE analysis_id = ?1 ORDER BY row_id")
        .map_err(|e| format!("prepare load nodes: {}", e))?;

    let nodes: Vec<DepNode> = node_stmt
        .query_map(params![analysis_id], |row| {
            Ok(DepNode {
                id: row.get(0)?,
                file_path: row.get(1)?,
                module_name: row.get(2)?,
                language: row.get(3)?,
            })
        })
        .map_err(|e| format!("query nodes: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let mut edge_stmt = conn
        .prepare("SELECT source_id, target_id, line, kind FROM dep_edges WHERE analysis_id = ?1")
        .map_err(|e| format!("prepare load edges: {}", e))?;

    let edges: Vec<DepEdge> = edge_stmt
        .query_map(params![analysis_id], |row| {
            let kind_str: String = row.get(3)?;
            let kind = match kind_str.as_str() {
                "dynamic" => ImportKind::Dynamic,
                "reexport" => ImportKind::ReExport,
                _ => ImportKind::Static,
            };
            Ok(DepEdge {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                line: row.get(2)?,
                kind,
            })
        })
        .map_err(|e| format!("query edges: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(DependencyGraph::new(nodes, edges))
}

pub fn load_latest_analysis(conn: &Connection) -> Result<Option<String>, String> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT analysis_id FROM dep_nodes ORDER BY analysis_id DESC LIMIT 1")
        .map_err(|e| format!("prepare latest: {}", e))?;

    let result: Option<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| format!("query latest: {}", e))?
        .filter_map(|r| r.ok())
        .next();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_graph() -> DependencyGraph {
        DependencyGraph {
            nodes: vec![
                DepNode {
                    id: "src/main.py".into(),
                    file_path: "src/main.py".into(),
                    module_name: "main".into(),
                    language: "python".into(),
                },
                DepNode {
                    id: "src/utils.py".into(),
                    file_path: "src/utils.py".into(),
                    module_name: "utils".into(),
                    language: "python".into(),
                },
            ],
            edges: vec![DepEdge {
                source_id: "src/main.py".into(),
                target_id: "src/utils.py".into(),
                line: 3,
                kind: ImportKind::Static,
            }],
            cycles: vec![],
            fan_in: std::collections::HashMap::new(),
            fan_out: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_create_and_persist() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        let graph = make_test_graph();
        persist_graph(&conn, &graph, "test-001").unwrap();
    }

    #[test]
    fn test_roundtrip() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        let original = make_test_graph();
        persist_graph(&conn, &original, "test-002").unwrap();

        let loaded = load_graph(&conn, "test-002").unwrap();
        assert_eq!(original.nodes.len(), loaded.nodes.len());
        assert_eq!(original.edges.len(), loaded.edges.len());
        assert_eq!(original.nodes[0].id, loaded.nodes[0].id);
        assert_eq!(original.edges[0].source_id, loaded.edges[0].source_id);
        assert_eq!(original.edges[0].kind, loaded.edges[0].kind);
    }

    #[test]
    fn test_load_nonexistent_analysis() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        let graph = load_graph(&conn, "nonexistent").unwrap();
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_multiple_analyses() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        let g1 = make_test_graph();
        persist_graph(&conn, &g1, "analysis-1").unwrap();

        let mut g2 = make_test_graph();
        g2.nodes.push(DepNode {
            id: "src/extra.py".into(),
            file_path: "src/extra.py".into(),
            module_name: "extra".into(),
            language: "python".into(),
        });
        persist_graph(&conn, &g2, "analysis-2").unwrap();

        let loaded_1 = load_graph(&conn, "analysis-1").unwrap();
        let loaded_2 = load_graph(&conn, "analysis-2").unwrap();

        assert_eq!(loaded_1.nodes.len(), 2);
        assert_eq!(loaded_2.nodes.len(), 3);
    }

    #[test]
    fn test_load_latest_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        let latest = load_latest_analysis(&conn).unwrap();
        assert!(latest.is_none());
    }

    #[test]
    fn test_load_latest_with_data() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        let graph = make_test_graph();
        persist_graph(&conn, &graph, "aaa").unwrap();
        persist_graph(&conn, &graph, "bbb").unwrap();
        let latest = load_latest_analysis(&conn).unwrap();
        assert_eq!(latest.as_deref(), Some("bbb"));
    }

    #[test]
    fn test_import_kind_roundtrip() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        let graph = DependencyGraph {
            nodes: vec![
                DepNode {
                    id: "a.js".into(),
                    file_path: "a.js".into(),
                    module_name: "a".into(),
                    language: "javascript".into(),
                },
                DepNode {
                    id: "b.js".into(),
                    file_path: "b.js".into(),
                    module_name: "b".into(),
                    language: "javascript".into(),
                },
            ],
            edges: vec![
                DepEdge {
                    source_id: "a.js".into(),
                    target_id: "b.js".into(),
                    line: 1,
                    kind: ImportKind::Static,
                },
                DepEdge {
                    source_id: "a.js".into(),
                    target_id: "c.js".into(),
                    line: 2,
                    kind: ImportKind::Dynamic,
                },
                DepEdge {
                    source_id: "a.js".into(),
                    target_id: "d.js".into(),
                    line: 3,
                    kind: ImportKind::ReExport,
                },
            ],
            cycles: vec![],
            fan_in: std::collections::HashMap::new(),
            fan_out: std::collections::HashMap::new(),
        };

        // c.js and d.js don't exist as nodes, but persist should still work for edges
        persist_graph(&conn, &graph, "kinds-test").unwrap();
        let loaded = load_graph(&conn, "kinds-test").unwrap();
        assert_eq!(loaded.edges.len(), 3);
    }
}
