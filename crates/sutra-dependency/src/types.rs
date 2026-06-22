use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepNode {
    pub id: String,
    pub file_path: String,
    pub module_name: String,
    pub language: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepEdge {
    pub source_id: String,
    pub target_id: String,
    pub line: u32,
    pub kind: ImportKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportKind {
    Static,
    Dynamic,
    ReExport,
}

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub nodes: Vec<DepNode>,
    pub edges: Vec<DepEdge>,
    pub cycles: Vec<Vec<String>>,
    pub fan_in: HashMap<String, usize>,
    pub fan_out: HashMap<String, usize>,
}

impl DependencyGraph {
    pub fn new(nodes: Vec<DepNode>, edges: Vec<DepEdge>) -> Self {
        Self {
            nodes,
            edges,
            cycles: vec![],
            fan_in: HashMap::new(),
            fan_out: HashMap::new(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn cycle_count(&self) -> usize {
        self.cycles.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchLayer {
    pub name: String,
    pub allowed_deps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArchConfig {
    pub layers: Vec<ArchLayer>,
}

impl ArchConfig {
    pub fn from_toml(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("failed to parse architecture config: {}", e))
    }

    pub fn is_allowed(&self, from_layer: &str, to_layer: &str) -> bool {
        let layer = match self.layers.iter().find(|l| l.name == from_layer) {
            Some(l) => l,
            None => return true,
        };
        if to_layer == from_layer {
            return true;
        }
        layer.allowed_deps.iter().any(|d| d == to_layer)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Json,
    Dot,
    Sarif,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedImport {
    pub module: String,
    pub line: u32,
    pub kind: ImportKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dep_graph_defaults() {
        let g = DependencyGraph::new(vec![], vec![]);
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
        assert_eq!(g.cycle_count(), 0);
        assert!(g.fan_in.is_empty());
        assert!(g.fan_out.is_empty());
    }

    #[test]
    fn test_arch_config_empty_allows_all() {
        let config = ArchConfig::default();
        assert!(config.is_allowed("web", "db"));
        assert!(config.is_allowed("unknown", "any"));
    }

    #[test]
    fn test_arch_config_allowed_deps() {
        let config = ArchConfig {
            layers: vec![ArchLayer {
                name: "web".into(),
                allowed_deps: vec!["service".into(), "shared".into()],
            }],
        };
        assert!(config.is_allowed("web", "service"));
        assert!(config.is_allowed("web", "shared"));
        assert!(config.is_allowed("web", "web"));
        assert!(!config.is_allowed("web", "db"));
    }

    #[test]
    fn test_arch_config_unknown_layer_always_allowed() {
        let config = ArchConfig {
            layers: vec![ArchLayer {
                name: "web".into(),
                allowed_deps: vec!["service".into()],
            }],
        };
        assert!(config.is_allowed("unknown_layer", "anything"));
    }

    #[test]
    fn test_arch_config_from_toml() {
        let toml_str = r#"
[[layers]]
name = "web"
allowed_deps = ["service", "shared"]

[[layers]]
name = "service"
allowed_deps = ["db", "shared"]

[[layers]]
name = "db"
allowed_deps = ["shared"]

[[layers]]
name = "shared"
allowed_deps = []
"#;
        let config = ArchConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.layers.len(), 4);
    }

    #[test]
    fn test_arch_config_from_invalid_toml() {
        let result = ArchConfig::from_toml("not valid toml {{{");
        assert!(result.is_err());
    }

    #[test]
    fn test_import_kind_serde() {
        for kind in &[ImportKind::Static, ImportKind::Dynamic, ImportKind::ReExport] {
            let json = serde_json::to_string(kind).unwrap();
            let back: ImportKind = serde_json::from_str(&json).unwrap();
            assert_eq!(*kind, back);
        }
    }

    #[test]
    fn test_dep_node_serde() {
        let node = DepNode {
            id: "src/main.py".into(),
            file_path: "src/main.py".into(),
            module_name: "main".into(),
            language: "python".into(),
        };
        let json = serde_json::to_string(&node).unwrap();
        let back: DepNode = serde_json::from_str(&json).unwrap();
        assert_eq!(node, back);
    }

    #[test]
    fn test_dep_edge_serde() {
        let edge = DepEdge {
            source_id: "src/a.py".into(),
            target_id: "src/b.py".into(),
            line: 5,
            kind: ImportKind::Static,
        };
        let json = serde_json::to_string(&edge).unwrap();
        let back: DepEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(edge, back);
    }
}
