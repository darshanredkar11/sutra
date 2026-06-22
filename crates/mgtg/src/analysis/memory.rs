use crate::analysis::Analyzer;
use crate::findings::finding_id;
use crate::graph::fsm::build_resource_fsms;
use crate::graph::refgraph::build_ref_graph;
use crate::graph::Graphs;
use crate::ir::{Finding, IrNode};

pub struct MemoryAnalyzer;

impl Analyzer for MemoryAnalyzer {
    fn analyze(&self, nodes: &[IrNode], _graphs: &Graphs, file: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        let mut idx = 0usize;

        // 1. Check resource FSMs for leaks
        let fsms = build_resource_fsms(nodes);
        for fsm in fsms {
            if fsm.state == crate::graph::fsm::ResourceState::Leaked {
                findings.push(Finding {
                    id: finding_id("M", idx),
                    category: "memory".to_string(),
                    subtype: "escaped_resource".to_string(),
                    severity: "error".to_string(),
                    file: file.to_string(),
                    line: fsm.alloc_line,
                    column: 0,
                    message: format!(
                        "Resource '{:?}' allocated at line {} may never be released",
                        fsm.kind, fsm.alloc_line
                    ),
                    graph_ref: None,
                });
                idx += 1;
            }
        }

        // 2. Check reference graph for escaped variables
        let ref_graph = build_ref_graph(nodes);
        for (var, _targets) in &ref_graph.edges {
            if ref_graph.can_escape(var) && !is_builtin(var) {
                // Check if it looks like a resource
                let lower = var.to_lowercase();
                if lower.contains("file")
                    || lower.contains("socket")
                    || lower.contains("conn")
                    || lower.contains("handle")
                    || lower.contains("fd")
                {
                    findings.push(Finding {
                        id: finding_id("M", idx + 100),
                        category: "memory".to_string(),
                        subtype: "escaped_reference".to_string(),
                        severity: "warning".to_string(),
                        file: file.to_string(),
                        line: 1,
                        column: 0,
                        message: format!(
                            "Variable '{}' may escape scope and prevent cleanup",
                            var
                        ),
                        graph_ref: None,
                    });
                    idx += 1;
                }
            }
        }

        // 3. Check closure captures
        check_closure_captures(nodes, file, &mut findings, &mut idx);

        findings
    }

    fn name(&self) -> &'static str {
        "memory"
    }
}

fn is_builtin(s: &str) -> bool {
    matches!(
        s,
        "self" | "this" | "_" | "args" | "kwargs" | "console" | "process" | "module" | "exports"
    )
}

fn is_number_str(s: &str) -> bool {
    s.parse::<f64>().is_ok()
}

fn check_closure_captures(
    nodes: &[IrNode],
    file: &str,
    findings: &mut Vec<Finding>,
    idx: &mut usize,
) {
    check_closure_captures_depth(nodes, file, findings, idx, 0);
}

fn check_closure_captures_depth(
    nodes: &[IrNode],
    file: &str,
    findings: &mut Vec<Finding>,
    idx: &mut usize,
    depth: usize,
) {
    for node in nodes {
        match node {
            IrNode::Closure { line, body, .. } => {
                // Scan body for free variables (actual captures)
                let refs = collect_variable_refs(body);
                for cap in refs {
                    if !is_builtin(&cap) {
                        findings.push(Finding {
                            id: finding_id("M", *idx + 200),
                            category: "memory".to_string(),
                            subtype: "closure_capture".to_string(),
                            severity: "info".to_string(),
                            file: file.to_string(),
                            line: *line,
                            column: 0,
                            message: format!(
                                "Variable '{}' captured in closure at line {}",
                                cap, line
                            ),
                            graph_ref: None,
                        });
                        *idx += 1;
                    }
                }
                check_closure_captures_depth(body, file, findings, idx, depth + 1);
            }
            IrNode::Function {
                body, params, name, line, ..
            } => {
                if depth > 0 {
                    let mut refs = collect_variable_refs(body);
                    let locals = collect_local_decls(body);
                    refs.retain(|v| !locals.contains(v) && !params.contains(v) && !is_builtin(v));
                    for cap in refs {
                        let fn_display = if name.is_empty() { "anonymous" } else { name };
                        findings.push(Finding {
                            id: finding_id("M", *idx + 200),
                            category: "memory".to_string(),
                            subtype: "closure_capture".to_string(),
                            severity: "info".to_string(),
                            file: file.to_string(),
                            line: *line,
                            column: 0,
                            message: format!(
                                "Variable '{}' captured by '{}' at line {}",
                                cap, fn_display, line
                            ),
                            graph_ref: None,
                        });
                        *idx += 1;
                    }
                }
                check_closure_captures_depth(body, file, findings, idx, depth + 1);
            }
            IrNode::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                check_closure_captures_depth(then_branch, file, findings, idx, depth);
                check_closure_captures_depth(else_branch, file, findings, idx, depth);
            }
            IrNode::Loop { body, .. } => {
                check_closure_captures_depth(body, file, findings, idx, depth);
            }
            _ => {}
        }
    }
}

/// Collect variable names referenced in a list of IR nodes.
fn collect_variable_refs(nodes: &[IrNode]) -> Vec<String> {
    let mut refs = Vec::new();
    for node in nodes {
        match node {
            IrNode::Assignment { source, target, .. } => {
                let t = target.trim();
                // Strip leading keywords like `let`, `const`, `var`
                let t_clean = if let Some(rest) = t.strip_prefix("let ") { rest }
                    else if let Some(rest) = t.strip_prefix("const ") { rest }
                    else if let Some(rest) = t.strip_prefix("var ") { rest }
                    else { t };
                // Take only the base identifier (before [ or .)
                let t_base = t_clean.trim().split(&['[', '.'][..]).next().unwrap_or(t_clean).trim();
                let t_ident = t_base.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                if !t_ident.is_empty() && !is_builtin(t_ident) && !is_number_str(t_ident) {
                    refs.push(t_ident.to_string());
                }
                for token in source.split_whitespace() {
                    let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    if !clean.is_empty() && !is_keyword(clean) && !is_number_str(clean) {
                        let base = clean.split(['(', '[']).next().unwrap_or(clean);
                        if !base.is_empty() && !is_keyword(base) && !is_number_str(base) {
                            refs.push(base.to_string());
                        }
                    }
                }
            }
            IrNode::Call { name, args, .. } => {
                let base = name.split('.').next().unwrap_or(name);
                if !base.is_empty() && !is_keyword(base) && !is_number_str(base) {
                    refs.push(base.to_string());
                }
                for arg in args {
                    let clean = arg.trim().to_string();
                    if !clean.is_empty() && !is_number_str(&clean) {
                        refs.push(clean);
                    }
                }
            }
            IrNode::Alloc { target, .. } => {
                refs.push(target.clone());
            }
            IrNode::Return { value, .. } => {
                if let Some(val) = value {
                    for token in val.split_whitespace() {
                        let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                        if !clean.is_empty() && !is_keyword(clean) && !is_number_str(clean) {
                            refs.push(clean.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    refs.sort();
    refs.dedup();
    refs
}

/// Collect variable names that are declared locally (assigned) in a body.
/// Only simple identifier targets like `x = ...` or `let x = ...` count as declarations.
/// Indexed assignments like `cache[data] = ...` do NOT declare new variables.
fn collect_local_decls(nodes: &[IrNode]) -> Vec<String> {
    let mut decls = Vec::new();
    for node in nodes {
        match node {
            IrNode::Assignment { target, .. } => {
                let t = target.trim();
                // Only treat as declaration if it's a simple identifier (no `[`, `.`, or spaces)
                if !t.contains('[') && !t.contains('.') {
                    let t_clean = if let Some(rest) = t.strip_prefix("let ") { rest }
                        else if let Some(rest) = t.strip_prefix("const ") { rest }
                        else if let Some(rest) = t.strip_prefix("var ") { rest }
                        else { t };
                    let t_ident = t_clean.trim().trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    if !t_ident.is_empty() {
                        decls.push(t_ident.to_string());
                    }
                }
            }
            IrNode::Alloc { target, .. } => {
                decls.push(target.clone());
            }
            IrNode::Function { .. } | IrNode::Closure { .. } => {
                // Don't recurse into nested functions — their locals are in their own scope
            }
            IrNode::Conditional { then_branch, else_branch, .. } => {
                decls.extend(collect_local_decls(then_branch));
                decls.extend(collect_local_decls(else_branch));
            }
            IrNode::Loop { body, .. } => {
                decls.extend(collect_local_decls(body));
            }
            _ => {}
        }
    }
    decls.sort();
    decls.dedup();
    decls
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "if" | "else" | "elif" | "for" | "while" | "return" | "def"
            | "function" | "var" | "let" | "const" | "class"
            | "import" | "from" | "export" | "async" | "await"
            | "true" | "false" | "null" | "undefined" | "None"
            | "not" | "and" | "or" | "in" | "is" | "new" | "try" | "except" | "finally"
            | "void" | "typeof" | "instanceof" | "delete" | "throw"
    )
}
