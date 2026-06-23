use crate::ir::{IrNode, LoopKind};
use super::Parser;

pub struct RustParser;

impl Parser for RustParser {
    fn parse(source: &str) -> Vec<IrNode> {
        let mut nodes = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        let mut in_block_comment = false;

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }
            if trimmed.starts_with("/*") {
                in_block_comment = true;
                if trimmed.contains("*/") { in_block_comment = false; }
                continue;
            }
            if in_block_comment {
                if trimmed.contains("*/") { in_block_comment = false; }
                continue;
            }

            // Detect function definitions
            let is_fn = trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("pub(crate) fn ")
                || trimmed.starts_with("unsafe fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("pub async fn ")
                || trimmed.starts_with("pub unsafe fn ");

            if is_fn && trimmed.contains('(') && !trimmed.contains("impl") {
                let name = trimmed
                    .split("fn ")
                    .nth(1)
                    .and_then(|s| s.split('(').next())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let params: Vec<String> = Vec::new();

                let mut body = Vec::new();
                for j in (i + 1)..lines.len() {
                    let l = lines[j].trim();
                    let body_line = j + 1;

                    if l.starts_with("if ") || l.starts_with("} else {") || l.starts_with("}else{")
                        || l.starts_with("} else if ") || l.starts_with("}else if ")
                        || (l.starts_with("if") && l.len() > 2 && !l.starts_with("ifdef"))
                    {
                        body.push(IrNode::Conditional {
                            condition: l.to_string(),
                            then_branch: vec![],
                            else_branch: vec![],
                            line: body_line,
                        });
                    }
                    if l.starts_with("match ") {
                        body.push(IrNode::Conditional {
                            condition: l.to_string(),
                            then_branch: vec![],
                            else_branch: vec![],
                            line: body_line,
                        });
                    }
                    if l.starts_with("loop ") || l.starts_with("while ") || l.starts_with("for ")
                        || l.starts_with("} loop ") || l.starts_with("} while ") || l.starts_with("} for ")
                    {
                        let kind = if l.starts_with("for ") || l.starts_with("} for ") {
                            LoopKind::For
                        } else if l.starts_with("while ") || l.starts_with("} while ") {
                            LoopKind::While
                        } else {
                            LoopKind::DoWhile
                        };
                        body.push(IrNode::Loop {
                            kind,
                            condition: l.to_string(),
                            body: vec![],
                            line: body_line,
                        });
                    }

                    if l == "}" || l == "};" || l.starts_with("} //") || l.starts_with("}/*") {
                        break;
                    }
                }

                nodes.push(IrNode::Function {
                    name,
                    params,
                    body,
                    line: line_num,
                });
            }

            // Top-level conditionals and loops (not inside a function)
            let in_fn = nodes.iter().any(|n| matches!(n, IrNode::Function { .. }));
            if !in_fn {
                if trimmed.starts_with("if ") || trimmed.starts_with("match ") {
                    nodes.push(IrNode::Conditional {
                        condition: trimmed.to_string(),
                        then_branch: vec![],
                        else_branch: vec![],
                        line: line_num,
                    });
                }
                if trimmed.starts_with("loop ") || trimmed.starts_with("while ") || trimmed.starts_with("for ") {
                    nodes.push(IrNode::Loop {
                        kind: if trimmed.starts_with("for ") { LoopKind::For }
                              else if trimmed.starts_with("while ") { LoopKind::While }
                              else { LoopKind::DoWhile },
                        condition: trimmed.to_string(),
                        body: vec![],
                        line: line_num,
                    });
                }
            }
        }

        nodes
    }

    fn language_name() -> &'static str {
        "rust"
    }
}
