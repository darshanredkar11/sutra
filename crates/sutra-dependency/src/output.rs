use serde::Serialize;

use crate::types::{DependencyGraph, ImportKind};

pub fn format_json(graph: &DependencyGraph) -> Result<String, String> {
    let report = JsonReport {
        nodes: graph.nodes.clone(),
        edges: graph.edges.clone(),
        cycles: graph.cycles.clone(),
        fan_in: graph.fan_in.clone(),
        fan_out: graph.fan_out.clone(),
        stats: JsonStats {
            node_count: graph.nodes.len(),
            edge_count: graph.edges.len(),
            cycle_count: graph.cycles.len(),
        },
    };
    serde_json::to_string_pretty(&report).map_err(|e| format!("json serialization: {}", e))
}

#[derive(Serialize)]
struct JsonReport {
    nodes: Vec<crate::types::DepNode>,
    edges: Vec<crate::types::DepEdge>,
    cycles: Vec<Vec<String>>,
    fan_in: std::collections::HashMap<String, usize>,
    fan_out: std::collections::HashMap<String, usize>,
    stats: JsonStats,
}

#[derive(Serialize)]
struct JsonStats {
    node_count: usize,
    edge_count: usize,
    cycle_count: usize,
}

pub fn format_dot(graph: &DependencyGraph) -> String {
    let mut out = String::from("digraph DependencyGraph {\n");
    out.push_str("    rankdir=LR;\n");
    out.push_str("    node [shape=box, style=rounded];\n\n");

    for node in &graph.nodes {
        let label = node
            .file_path
            .rsplit('/')
            .next()
            .unwrap_or(&node.file_path)
            .replace('"', "\\\"");
        out.push_str(&format!(
            "    \"{}\" [label=\"{}\"];\n",
            node.id.replace('"', "\\\""),
            label
        ));
    }

    out.push('\n');

    for edge in &graph.edges {
        let style = match edge.kind {
            ImportKind::Dynamic => "dashed",
            ImportKind::ReExport => "dotted",
            ImportKind::Static => "solid",
        };
        let color = match edge.kind {
            ImportKind::Dynamic => "orange",
            ImportKind::ReExport => "blue",
            ImportKind::Static => "black",
        };
        out.push_str(&format!(
            "    \"{}\" -> \"{}\" [style={}, color={}];\n",
            edge.source_id.replace('"', "\\\""),
            edge.target_id.replace('"', "\\\""),
            style,
            color,
        ));
    }

    if !graph.cycles.is_empty() {
        out.push('\n');
        out.push_str("    // Cycles detected:\n");
        for (i, cycle) in graph.cycles.iter().enumerate() {
            let cycle_nodes: Vec<&str> = cycle
                .iter()
                .map(|n| {
                    n.rsplit('/')
                        .next()
                        .unwrap_or(n)
                })
                .collect();
            out.push_str(&format!("    // Cycle {}: {}\n", i + 1, cycle_nodes.join(" -> ")));
        }
    }

    out.push_str("}\n");
    out
}

pub fn format_sarif(
    graph: &DependencyGraph,
    tool_name: &str,
    tool_version: &str,
) -> Result<String, String> {
    let mut results = Vec::new();

    for (i, cycle) in graph.cycles.iter().enumerate() {
        let cycle_path: Vec<&str> = cycle.iter().map(|n| {
            n.rsplit('/').next().unwrap_or(n)
        }).collect();
        let message = format!(
            "Circular dependency detected: {}",
            cycle_path.join(" -> ")
        );

        let locations: Vec<SarifLocation> = cycle
            .iter()
            .map(|node_id| {
                let _node = graph.nodes.iter().find(|n| n.id == *node_id);
                SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation {
                            uri: node_id.clone(),
                        },
                        region: SarifRegion {
                            start_line: 1,
                        },
                    },
                }
            })
            .collect();

        results.push(SarifResult {
            rule_id: format!("SUTRA-DEP{:03}", i + 1),
            level: "warning".to_string(),
            message: SarifMessage {
                text: message,
            },
            locations,
        });
    }

    let sarif = SarifOutput {
        schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".into(),
        version: "2.1.0".into(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: tool_name.to_string(),
                    version: tool_version.to_string(),
                },
            },
            results,
        }],
    };

    serde_json::to_string_pretty(&sarif).map_err(|e| format!("sarif serialization: {}", e))
}

#[derive(Serialize)]
struct SarifOutput {
    #[serde(rename = "$schema")]
    schema: String,
    version: String,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
struct SarifDriver {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    level: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize)]
struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: SarifArtifactLocation,
    region: SarifRegion,
}

#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Serialize)]
struct SarifRegion {
    #[serde(rename = "startLine")]
    start_line: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DepEdge, DepNode, ImportKind};
    use std::collections::HashMap;

    fn sample_graph() -> DependencyGraph {
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
                DepNode {
                    id: "src/models.py".into(),
                    file_path: "src/models.py".into(),
                    module_name: "models".into(),
                    language: "python".into(),
                },
            ],
            edges: vec![
                DepEdge {
                    source_id: "src/main.py".into(),
                    target_id: "src/utils.py".into(),
                    line: 3,
                    kind: ImportKind::Static,
                },
                DepEdge {
                    source_id: "src/utils.py".into(),
                    target_id: "src/models.py".into(),
                    line: 1,
                    kind: ImportKind::ReExport,
                },
            ],
            cycles: vec![],
            fan_in: HashMap::new(),
            fan_out: HashMap::new(),
        }
    }

    #[test]
    fn test_json_output() {
        let graph = sample_graph();
        let json = format_json(&graph).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["stats"]["node_count"], 3);
        assert_eq!(parsed["stats"]["edge_count"], 2);
        assert!(parsed["nodes"].is_array());
        assert!(parsed["edges"].is_array());
    }

    #[test]
    fn test_json_with_cycles() {
        let mut graph = sample_graph();
        graph.cycles = vec![vec!["a".into(), "b".into()]];
        let json = format_json(&graph).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["cycles"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_dot_output() {
        let graph = sample_graph();
        let dot = format_dot(&graph);
        assert!(dot.starts_with("digraph"));
        assert!(dot.contains("src/main.py"));
        assert!(dot.contains("->"));
        assert!(dot.ends_with("}\n"));
    }

    #[test]
    fn test_dot_static_style() {
        let graph = sample_graph();
        let dot = format_dot(&graph);
        assert!(dot.contains("style=solid"));
    }

    #[test]
    fn test_dot_with_cycles() {
        let mut graph = sample_graph();
        graph.cycles = vec![vec!["src/main.py".into(), "src/utils.py".into()]];
        let dot = format_dot(&graph);
        assert!(dot.contains("Cycle"));
    }

    #[test]
    fn test_sarif_no_cycles() {
        let graph = sample_graph();
        let sarif = format_sarif(&graph, "sutra", "0.1.0").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&sarif).unwrap();
        assert_eq!(parsed["runs"][0]["results"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_sarif_with_cycles() {
        let mut graph = sample_graph();
        graph.cycles = vec![vec!["src/main.py".into(), "src/utils.py".into()]];
        let sarif = format_sarif(&graph, "sutra", "0.1.0").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&sarif).unwrap();
        assert_eq!(parsed["runs"][0]["results"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["runs"][0]["results"][0]["level"], "warning");
    }

    #[test]
    fn test_sarif_schema() {
        let graph = DependencyGraph::new(vec![], vec![]);
        let sarif = format_sarif(&graph, "sutra-test", "1.0.0").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&sarif).unwrap();
        assert_eq!(parsed["version"], "2.1.0");
        assert_eq!(parsed["runs"][0]["tool"]["driver"]["name"], "sutra-test");
    }

    #[test]
    fn test_dot_empty_graph() {
        let graph = DependencyGraph::new(vec![], vec![]);
        let dot = format_dot(&graph);
        assert!(dot.contains("digraph"));
        assert!(dot.ends_with("}\n"));
    }

    #[test]
    fn test_dot_dynamic_style() {
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
            edges: vec![DepEdge {
                source_id: "a.js".into(),
                target_id: "b.js".into(),
                line: 1,
                kind: ImportKind::Dynamic,
            }],
            cycles: vec![],
            fan_in: HashMap::new(),
            fan_out: HashMap::new(),
        };
        let dot = format_dot(&graph);
        assert!(dot.contains("style=dashed"));
        assert!(dot.contains("color=orange"));
    }
}
