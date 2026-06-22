use std::collections::HashSet;

use crate::analysis::Analyzer;
use crate::findings::finding_id;
use crate::graph::Graphs;
use crate::ir::{Finding, IrNode};

pub struct ComputationAnalyzer;

impl Analyzer for ComputationAnalyzer {
    fn analyze(&self, nodes: &[IrNode], _graphs: &Graphs, file: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        let mut function_names = HashSet::new();

        collect_function_names(nodes, &mut function_names);

        let max_loop_nest = max_loop_nesting(nodes);
        if max_loop_nest > 2 {
            findings.push(Finding {
                id: finding_id("P", 0),
                category: "computation".to_string(),
                subtype: "deep_loop_nest".to_string(),
                severity: "info".to_string(),
                file: file.to_string(),
                line: 1,
                column: 0,
                message: format!(
                    "Loop nesting depth is {} (threshold: 2)",
                    max_loop_nest
                ),
                graph_ref: None,
            });
        }

        let recursion_info = find_recursion(nodes, &function_names);
        for (func_name, depth, line) in &recursion_info {
            findings.push(Finding {
                id: finding_id("P", 1),
                category: "computation".to_string(),
                subtype: "recursive_function".to_string(),
                severity: "info".to_string(),
                file: file.to_string(),
                line: *line,
                column: 0,
                message: format!(
                    "Function '{}' is recursive (depth: {})",
                    func_name, depth
                ),
                graph_ref: None,
            });
        }

        findings
    }

    fn name(&self) -> &'static str {
        "computation"
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

pub fn max_loop_nesting(nodes: &[IrNode]) -> usize {
    fn walk(nodes: &[IrNode], depth: usize) -> usize {
        let mut max = depth;
        for node in nodes {
            match node {
                IrNode::Loop { body, .. } => {
                    let d = walk(body, depth + 1);
                    max = max.max(d);
                }
                IrNode::Function { body, .. } | IrNode::Closure { body, .. } => {
                    let d = walk(body, depth);
                    max = max.max(d);
                }
                IrNode::Conditional {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    max = max.max(walk(then_branch, depth));
                    max = max.max(walk(else_branch, depth));
                }
                _ => {}
            }
        }
        max
    }
    walk(nodes, 0)
}

fn find_recursion(
    nodes: &[IrNode],
    function_names: &HashSet<String>,
) -> Vec<(String, usize, usize)> {
    let mut result = Vec::new();

    for node in nodes {
        if let IrNode::Function {
            name,
            body,
            line,
            ..
        } = node
        {
            let depth = find_recursive_calls(body, function_names, name, 0);
            if depth > 0 {
                result.push((name.clone(), depth, *line));
            }
        }
    }

    result
}

fn find_recursive_calls(
    nodes: &[IrNode],
    function_names: &HashSet<String>,
    current_func: &str,
    depth: usize,
) -> usize {
    let mut max_depth = depth;
    for node in nodes {
        match node {
            IrNode::Call { name, .. } => {
                if name == current_func {
                    max_depth = max_depth.max(depth + 1);
                }
            }
            IrNode::Function { body, .. } | IrNode::Closure { body, .. } => {
                let d = find_recursive_calls(body, function_names, current_func, depth);
                max_depth = max_depth.max(d);
            }
            IrNode::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                let d1 = find_recursive_calls(then_branch, function_names, current_func, depth);
                let d2 = find_recursive_calls(else_branch, function_names, current_func, depth);
                max_depth = max_depth.max(d1).max(d2);
            }
            IrNode::Loop { body, .. } => {
                let d = find_recursive_calls(body, function_names, current_func, depth);
                max_depth = max_depth.max(d);
            }
            _ => {}
        }
    }
    max_depth
}
