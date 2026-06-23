use crate::types::{ArchConfig, DependencyGraph};

#[derive(Debug, Clone)]
pub struct ArchViolation {
    pub from_node: String,
    pub to_node: String,
    pub from_layer: String,
    pub to_layer: String,
    pub message: String,
}

pub struct ArchEngine {
    config: ArchConfig,
    layer_map: Vec<(String, Vec<String>)>,
}

impl ArchEngine {
    pub fn new(config: ArchConfig) -> Self {
        let layer_map: Vec<(String, Vec<String>)> = config
            .layers
            .iter()
            .map(|l| (l.name.clone(), l.allowed_deps.clone()))
            .collect();
        Self { config, layer_map }
    }

    pub fn from_toml(content: &str) -> Result<Self, String> {
        let config = ArchConfig::from_toml(content)?;
        Ok(Self::new(config))
    }

    pub fn detect_layer(&self, module_path: &str) -> Option<String> {
        // ponytail: improved layer detection — match path hierarchy, not just substring
        // Examples: "api/users.rs" matches "api" layer, "api.controllers" matches "api" layer
        // but "api_deprecated.rs" should NOT match "api" layer
        for (layer_name, _) in &self.layer_map {
            // Match layer as first component with clear boundary (slash or dot)
            if module_path == layer_name
                || module_path.starts_with(&format!("{}/", layer_name))
                || module_path.starts_with(&format!("src/{}/", layer_name))
                || module_path.starts_with(&format!("{}.", layer_name))
            {
                return Some(layer_name.clone());
            }
        }
        None
    }

    pub fn validate(&self, graph: &DependencyGraph) -> Vec<ArchViolation> {
        let mut violations = Vec::new();

        if self.layer_map.is_empty() {
            return violations;
        }

        for edge in &graph.edges {
            let from_node = match graph.nodes.iter().find(|n| n.id == edge.source_id) {
                Some(n) => n,
                None => continue,
            };
            let to_node = match graph.nodes.iter().find(|n| n.id == edge.target_id) {
                Some(n) => n,
                None => continue,
            };

            let from_layer = self.detect_layer(&from_node.module_name);
            let to_layer = self.detect_layer(&to_node.module_name);

            if let (Some(fl), Some(tl)) = (&from_layer, &to_layer) {
                if fl != tl && !self.config.is_allowed(fl, tl) {
                    violations.push(ArchViolation {
                        from_node: from_node.id.clone(),
                        to_node: to_node.id.clone(),
                        from_layer: fl.clone(),
                        to_layer: tl.clone(),
                        message: format!(
                            "Layer '{}' is not allowed to depend on layer '{}'",
                            fl, tl
                        ),
                    });
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DepNode, DepEdge, ImportKind};

    #[test]
    fn test_detect_layer_by_path() {
        let config = ArchConfig::from_toml(
            r#"
[[layers]]
name = "web"
allowed_deps = ["service"]

[[layers]]
name = "service"
allowed_deps = ["db"]

[[layers]]
name = "db"
allowed_deps = []
"#,
        )
        .unwrap();
        let engine = ArchEngine::new(config);
        assert_eq!(engine.detect_layer("web.controllers"), Some("web".into()));
        assert_eq!(engine.detect_layer("service.models"), Some("service".into()));
        assert_eq!(engine.detect_layer("db.repository"), Some("db".into()));
        assert_eq!(engine.detect_layer("external.lib"), None);
    }

    #[test]
    fn test_validate_no_violations() {
        let config = ArchConfig::from_toml(
            r#"
[[layers]]
name = "web"
allowed_deps = ["service"]

[[layers]]
name = "service"
allowed_deps = ["db"]
"#,
        )
        .unwrap();
        let engine = ArchEngine::new(config);
        let graph = DependencyGraph {
            nodes: vec![
                DepNode {
                    id: "web/a.js".into(),
                    file_path: "web/a.js".into(),
                    module_name: "web.controllers".into(),
                    language: "javascript".into(),
                },
                DepNode {
                    id: "service/b.js".into(),
                    file_path: "service/b.js".into(),
                    module_name: "service.handler".into(),
                    language: "javascript".into(),
                },
                DepNode {
                    id: "db/c.js".into(),
                    file_path: "db/c.js".into(),
                    module_name: "db.models".into(),
                    language: "javascript".into(),
                },
            ],
            edges: vec![
                DepEdge {
                    source_id: "web/a.js".into(),
                    target_id: "service/b.js".into(),
                    line: 1,
                    kind: ImportKind::Static,
                },
                DepEdge {
                    source_id: "service/b.js".into(),
                    target_id: "db/c.js".into(),
                    line: 2,
                    kind: ImportKind::Static,
                },
            ],
            cycles: vec![],
            fan_in: std::collections::HashMap::new(),
            fan_out: std::collections::HashMap::new(),
        };
        let violations = engine.validate(&graph);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_validate_with_violation() {
        let config = ArchConfig::from_toml(
            r#"
[[layers]]
name = "web"
allowed_deps = ["service"]

[[layers]]
name = "db"
allowed_deps = []
"#,
        )
        .unwrap();
        let engine = ArchEngine::new(config);
        let graph = DependencyGraph {
            nodes: vec![
                DepNode {
                    id: "web/app.js".into(),
                    file_path: "web/app.js".into(),
                    module_name: "web.app".into(),
                    language: "javascript".into(),
                },
                DepNode {
                    id: "db/models.js".into(),
                    file_path: "db/models.js".into(),
                    module_name: "db.models".into(),
                    language: "javascript".into(),
                },
            ],
            edges: vec![DepEdge {
                source_id: "db/models.js".into(),
                target_id: "web/app.js".into(),
                line: 5,
                kind: ImportKind::Static,
            }],
            cycles: vec![],
            fan_in: std::collections::HashMap::new(),
            fan_out: std::collections::HashMap::new(),
        };
        let violations = engine.validate(&graph);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("not allowed"));
    }

    #[test]
    fn test_validate_empty_config_no_violations() {
        let engine = ArchEngine::new(ArchConfig::default());
        let graph = DependencyGraph::new(vec![], vec![]);
        let violations = engine.validate(&graph);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_validate_unknown_layer_allowed() {
        let config = ArchConfig::from_toml(
            r#"
[[layers]]
name = "web"
allowed_deps = ["service"]
"#,
        )
        .unwrap();
        let engine = ArchEngine::new(config);
        let graph = DependencyGraph {
            nodes: vec![
                DepNode {
                    id: "unknown/x.py".into(),
                    file_path: "unknown/x.py".into(),
                    module_name: "external.lib".into(),
                    language: "python".into(),
                },
                DepNode {
                    id: "web/app.py".into(),
                    file_path: "web/app.py".into(),
                    module_name: "web.app".into(),
                    language: "python".into(),
                },
            ],
            edges: vec![DepEdge {
                source_id: "unknown/x.py".into(),
                target_id: "web/app.py".into(),
                line: 1,
                kind: ImportKind::Static,
            }],
            cycles: vec![],
            fan_in: std::collections::HashMap::new(),
            fan_out: std::collections::HashMap::new(),
        };
        let violations = engine.validate(&graph);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_engine_from_toml() {
        let engine = ArchEngine::from_toml(
            r#"
[[layers]]
name = "web"
allowed_deps = ["service"]
"#,
        )
        .unwrap();
        assert_eq!(engine.layer_map.len(), 1);
    }

    #[test]
    fn test_engine_from_invalid_toml() {
        let result = ArchEngine::from_toml("not valid");
        assert!(result.is_err());
    }

    #[test]
    fn test_all_layers_allowed_no_violations() {
        let config = ArchConfig::from_toml(
            r#"
[[layers]]
name = "web"
allowed_deps = ["service", "db"]

[[layers]]
name = "service"
allowed_deps = ["web", "db"]

[[layers]]
name = "db"
allowed_deps = ["web", "service"]
"#,
        )
        .unwrap();
        let engine = ArchEngine::new(config);
        let graph = DependencyGraph {
            nodes: vec![
                DepNode {
                    id: "web/app.js".into(),
                    file_path: "web/app.js".into(),
                    module_name: "web.app".into(),
                    language: "javascript".into(),
                },
                DepNode {
                    id: "service/handler.js".into(),
                    file_path: "service/handler.js".into(),
                    module_name: "service.handler".into(),
                    language: "javascript".into(),
                },
                DepNode {
                    id: "db/models.js".into(),
                    file_path: "db/models.js".into(),
                    module_name: "db.models".into(),
                    language: "javascript".into(),
                },
            ],
            edges: vec![
                DepEdge {
                    source_id: "web/app.js".into(),
                    target_id: "db/models.js".into(),
                    line: 1,
                    kind: ImportKind::Static,
                },
                DepEdge {
                    source_id: "service/handler.js".into(),
                    target_id: "web/app.js".into(),
                    line: 2,
                    kind: ImportKind::Static,
                },
            ],
            cycles: vec![],
            fan_in: std::collections::HashMap::new(),
            fan_out: std::collections::HashMap::new(),
        };
        let violations = engine.validate(&graph);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_files_not_matching_any_layer() {
        let config = ArchConfig::from_toml(
            r#"
[[layers]]
name = "web"
allowed_deps = ["db"]

[[layers]]
name = "db"
allowed_deps = []
"#,
        )
        .unwrap();
        let engine = ArchEngine::new(config);
        let graph = DependencyGraph {
            nodes: vec![
                DepNode {
                    id: "external/lib.py".into(),
                    file_path: "external/lib.py".into(),
                    module_name: "external.lib".into(),
                    language: "python".into(),
                },
                DepNode {
                    id: "vendor/thing.py".into(),
                    file_path: "vendor/thing.py".into(),
                    module_name: "vendor.thing".into(),
                    language: "python".into(),
                },
            ],
            edges: vec![DepEdge {
                source_id: "external/lib.py".into(),
                target_id: "vendor/thing.py".into(),
                line: 1,
                kind: ImportKind::Static,
            }],
            cycles: vec![],
            fan_in: std::collections::HashMap::new(),
            fan_out: std::collections::HashMap::new(),
        };
        let violations = engine.validate(&graph);
        assert!(violations.is_empty());
    }
}
