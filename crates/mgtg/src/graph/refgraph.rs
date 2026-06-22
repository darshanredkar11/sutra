use std::collections::{HashMap, HashSet, VecDeque};

use crate::ir::IrNode;

#[derive(Debug, Clone)]
pub struct RefGraph {
    /// Variable → set of variables/objects it references
    pub edges: HashMap<String, HashSet<String>>,
    /// Reverse edges: variable → variables that reference it
    pub reverse: HashMap<String, HashSet<String>>,
    /// Variables that escape their scope (returned, captured, assigned to global)
    pub escaped: HashSet<String>,
}

impl RefGraph {
    pub fn references(&self, var: &str) -> Option<&HashSet<String>> {
        self.edges.get(var)
    }

    pub fn referenced_by(&self, var: &str) -> Option<&HashSet<String>> {
        self.reverse.get(var)
    }

    /// Find all variables transitively reachable from `var`
    pub fn transitive_closure(&self, var: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(var.to_string());

        while let Some(current) = queue.pop_front() {
            if visited.insert(current.clone()) {
                if let Some(targets) = self.edges.get(&current) {
                    for target in targets {
                        queue.push_back(target.clone());
                    }
                }
            }
        }
        visited
    }

    /// Check if a variable can escape scope
    pub fn can_escape(&self, var: &str) -> bool {
        self.escaped.contains(var) || {
            let closure = self.transitive_closure(var);
            closure.iter().any(|v| self.escaped.contains(v))
        }
    }
}

pub fn build_ref_graph(nodes: &[IrNode]) -> RefGraph {
    let mut edges: HashMap<String, HashSet<String>> = HashMap::new();
    let mut reverse: HashMap<String, HashSet<String>> = HashMap::new();
    let mut escaped: HashSet<String> = HashSet::new();

    let mut function_names = HashSet::new();
    collect_function_names(nodes, &mut function_names);

    build_ref_graph_inner(nodes, &mut edges, &mut reverse, &mut escaped, &function_names);

    RefGraph {
        edges,
        reverse,
        escaped,
    }
}

fn collect_function_names(nodes: &[IrNode], names: &mut HashSet<String>) {
    for node in nodes {
        match node {
            IrNode::Function { name, body, .. } => {
                names.insert(name.clone());
                collect_function_names(body, names);
            }
            IrNode::Closure { body, .. } => {
                collect_function_names(body, names);
            }
            IrNode::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                collect_function_names(then_branch, names);
                collect_function_names(else_branch, names);
            }
            IrNode::Loop { body, .. } => {
                collect_function_names(body, names);
            }
            _ => {}
        }
    }
}

fn build_ref_graph_inner(
    nodes: &[IrNode],
    edges: &mut HashMap<String, HashSet<String>>,
    reverse: &mut HashMap<String, HashSet<String>>,
    escaped: &mut HashSet<String>,
    function_names: &HashSet<String>,
) {
    for node in nodes {
        match node {
            IrNode::Assignment { target, source, .. } => {
                // target references source variables
                let refs: Vec<String> = source
                    .split_whitespace()
                    .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric() && c != '_'))
                    .filter(|t| !t.is_empty() && !is_keyword(t) && !is_number(t))
                    .map(|t| t.to_string())
                    .collect();
                for r in refs {
                    edges.entry(target.clone()).or_default().insert(r.clone());
                    reverse.entry(r).or_default().insert(target.clone());
                }
            }
            IrNode::Return { value, .. } => {
                // Returned values escape
                if let Some(val) = value {
                    for token in val.split_whitespace() {
                        let clean =
                            token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                        if !clean.is_empty() && !is_keyword(clean) && !is_number(clean) {
                            escaped.insert(clean.to_string());
                        }
                    }
                }
            }
            IrNode::Closure { body, .. } => {
                // All variable references in closure body escape via closure
                for node in body {
                    match node {
                        IrNode::Assignment { source, target, .. } => {
                            escaped.insert(target.clone());
                            for token in source.split_whitespace() {
                                let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                                if !clean.is_empty() && !is_keyword(clean) && !is_number(clean) {
                                    escaped.insert(clean.to_string());
                                }
                            }
                        }
                        IrNode::Call { name, args, .. } => {
                            let base = name.split('.').next().unwrap_or(name);
                            if !base.is_empty() && !is_keyword(base) && !is_number(base) {
                                escaped.insert(base.to_string());
                            }
                            for arg in args {
                                if !arg.is_empty() && !is_keyword(arg) && !is_number(arg) {
                                    escaped.insert(arg.clone());
                                }
                            }
                        }
                        IrNode::Alloc { target, .. } => {
                            escaped.insert(target.clone());
                        }
                        IrNode::Return { value, .. } => {
                            if let Some(val) = value {
                                for token in val.split_whitespace() {
                                    let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                                    if !clean.is_empty() && !is_keyword(clean) && !is_number(clean) {
                                        escaped.insert(clean.to_string());
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            IrNode::Alloc { target, .. } => {
                // Allocations that are assigned to something that escapes also escape
                edges.entry(target.clone()).or_default();
            }
            _ => {}
        }
    }

    // Recurse into nested structures
    for node in nodes {
        match node {
            IrNode::Function { body, .. } | IrNode::Closure { body, .. } => {
                build_ref_graph_inner(body, edges, reverse, escaped, function_names);
            }
            IrNode::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                build_ref_graph_inner(then_branch, edges, reverse, escaped, function_names);
                build_ref_graph_inner(else_branch, edges, reverse, escaped, function_names);
            }
            IrNode::Loop { body, .. } => {
                build_ref_graph_inner(body, edges, reverse, escaped, function_names);
            }
            _ => {}
        }
    }
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "if" | "else" | "for" | "while" | "return" | "def"
            | "function" | "var" | "let" | "const" | "class"
            | "import" | "from" | "export" | "async" | "await"
            | "true" | "false" | "null" | "undefined" | "None"
            | "not" | "and" | "or" | "in" | "is" | "new"
    )
}

fn is_number(s: &str) -> bool {
    s.parse::<f64>().is_ok()
}
