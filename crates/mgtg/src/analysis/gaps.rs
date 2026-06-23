use crate::analysis::Analyzer;
use crate::findings::finding_id;
use crate::graph::Graphs;
use crate::ir::{Finding, IrNode};

pub struct GapsAnalyzer;

impl Analyzer for GapsAnalyzer {
    fn analyze(&self, nodes: &[IrNode], _graphs: &Graphs, file: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        check_gaps(nodes, file, &mut findings, 0);
        findings
    }

    fn name(&self) -> &'static str {
        "gaps"
    }
}

fn check_gaps(nodes: &[IrNode], file: &str, findings: &mut Vec<Finding>, depth: usize) {
    for node in nodes {
        match node {
            IrNode::Conditional {
                then_branch,
                else_branch,
                condition,
                line,
            } => {
                // Check for missing else branch
                if else_branch.is_empty() {
                    // Check if it's not a guard clause (if with return or raise/throw)
                    let terminates_in_then = then_branch
                        .iter()
                        .any(|n| matches!(n, IrNode::Return { .. } | IrNode::Raise { .. }));
                    if !terminates_in_then {
                        findings.push(Finding {
                            id: finding_id("G", 0),
                            category: "gaps".to_string(),
                            subtype: "missing_branch".to_string(),
                            severity: "warning".to_string(),
                            file: file.to_string(),
                            line: *line,
                            column: 0,
                            message: format!(
                                "Conditional 'if {}' at line {} has no else branch",
                                condition, line
                            ),
                            graph_ref: None,
                        });
                    }
                }

                // Check for null-check patterns
                let lower = condition.to_lowercase();
                if lower.contains("null")
                    || lower.contains("undefined")
                    || lower.contains("none")
                    || lower.contains("is none")
                    || lower.contains("is not none")
                {
                    // Check if the null/undefined branch is handled
                    if else_branch.is_empty() || then_branch.is_empty() {
                        findings.push(Finding {
                            id: finding_id("G", 1),
                            category: "gaps".to_string(),
                            subtype: "unhandled_null_path".to_string(),
                            severity: "warning".to_string(),
                            file: file.to_string(),
                            line: *line,
                            column: 0,
                            message: format!(
                                "Potential unhandled null/undefined path at line {}",
                                line
                            ),
                            graph_ref: None,
                        });
                    }
                }

                // Check for deep nesting
                if depth >= 4 {
                    findings.push(Finding {
                        id: finding_id("G", 2),
                        category: "gaps".to_string(),
                        subtype: "deep_conditional_chain".to_string(),
                        severity: "info".to_string(),
                        file: file.to_string(),
                        line: *line,
                        column: 0,
                        message: format!("Deep conditional chain at line {} (depth {})", line, depth),
                        graph_ref: None,
                    });
                }

                check_gaps(then_branch, file, findings, depth + 1);
                check_gaps(else_branch, file, findings, depth + 1);
            }
            IrNode::Loop { body, .. } => {
                check_gaps(body, file, findings, depth);
            }
            IrNode::Function { body, .. } | IrNode::Closure { body, .. } => {
                check_gaps(body, file, findings, depth);
            }
            _ => {}
        }
    }
}
