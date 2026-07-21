use crate::ir::{IrNode, LoopKind};
use super::Parser;

pub struct RustParser;

/// Index (0-based) of the line containing the closing `}` that matches the
/// opening `{` first seen on `start_idx`. Brace-counts the whole line's
/// text, so single-line blocks (`if x { return y; }`) resolve on `start_idx`
/// itself. This is a heuristic (doesn't understand strings/chars containing
/// braces) but good enough for guard-clause detection below.
fn find_block_end(lines: &[&str], start_idx: usize) -> usize {
    let mut depth = 0i32;
    let mut started = false;
    for (k, l) in lines.iter().enumerate().skip(start_idx) {
        for ch in l.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    started = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if started && depth <= 0 {
            return k;
        }
    }
    lines.len().saturating_sub(1)
}

/// Whether the block spanning `lines[start_idx..=end_idx]` contains a
/// control-flow terminator (`return`, `continue`, `break`, `panic!`,
/// `bail!`, or a bare `Err(...)` tail expression) -- the signature of a
/// Rust guard clause (`if cond { return Err(..); }`) that doesn't need a
/// matching `else` because the `if` branch exits the function.
fn block_has_terminator(lines: &[&str], start_idx: usize, end_idx: usize) -> bool {
    let end = end_idx.min(lines.len().saturating_sub(1));
    lines[start_idx..=end].iter().any(|l| {
        let t = l.trim();
        t.starts_with("return")
            || t.starts_with("continue")
            || t.starts_with("break")
            || t.contains("panic!(")
            || t.contains("bail!(")
            || t.starts_with("Err(")
    })
}

/// Strip a leading keyword (`if`, `match`, `} else if`, `}else if`, ...)
/// and a trailing opening brace from a raw source line, leaving just the
/// condition/scrutinee text -- matching the contract the JS/Python
/// tree-sitter parsers already follow (`extract_condition` returns the
/// bare expression, not the whole statement). Callers that prepend their
/// own "if "/"match " when formatting messages would otherwise double up
/// ("Conditional 'if if request.method() != ...'").
fn strip_keyword(l: &str, keywords: &[&str]) -> String {
    let mut rest = l;
    for kw in keywords {
        if let Some(r) = rest.strip_prefix(kw) {
            rest = r;
            break;
        }
    }
    rest.trim().trim_end_matches('{').trim().to_string()
}

/// Mark the most recently pushed sibling `Conditional` as having a non-empty
/// else branch. Used when a `} else {` / `} else if` continuation line is
/// seen: that line is NOT a new, independent conditional (the previous
/// line-based parser pushed it as one, generating a phantom "no else
/// branch" finding against the else clause itself) -- it's evidence that
/// the *preceding* if/else-if now has an else.
fn attach_else(body: &mut [IrNode], line: usize) {
    if let Some(IrNode::Conditional { else_branch, .. }) =
        body.iter_mut().rev().find(|n| matches!(n, IrNode::Conditional { .. }))
    {
        if else_branch.is_empty() {
            else_branch.push(IrNode::Unresolved { text: "else".to_string(), line });
        }
    }
}

fn push_conditional(body: &mut Vec<IrNode>, lines: &[&str], j: usize, condition: String, line: usize) {
    let end_idx = find_block_end(lines, j);
    let then_branch = if block_has_terminator(lines, j, end_idx) {
        vec![IrNode::Return { value: None, line }]
    } else {
        vec![]
    };
    body.push(IrNode::Conditional {
        condition,
        then_branch,
        else_branch: vec![],
        line,
    });
}

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

                    if l.starts_with("} else if ") || l.starts_with("}else if ") {
                        attach_else(&mut body, body_line);
                        let cond = strip_keyword(l, &["} else if ", "}else if "]);
                        push_conditional(&mut body, &lines, j, cond, body_line);
                    } else if l.starts_with("} else {") || l.starts_with("}else{")
                        || l == "} else" || l == "}else"
                    {
                        attach_else(&mut body, body_line);
                    } else if l.starts_with("if ")
                        || (l.starts_with("if") && l.len() > 2 && !l.starts_with("ifdef"))
                    {
                        let cond = strip_keyword(l, &["if ", "if"]);
                        push_conditional(&mut body, &lines, j, cond, body_line);
                    } else if l.starts_with("match ") {
                        // `match` is exhaustive by construction in Rust (the
                        // compiler enforces it); "has no else branch" is a
                        // category error for it, not a real gap. Still
                        // recorded as a Conditional (other analyzers use
                        // these as branch points for complexity), but with a
                        // "match " condition prefix gaps.rs recognizes and
                        // skips for the no-else-branch check.
                        let cond = strip_keyword(l, &["match "]);
                        body.push(IrNode::Conditional {
                            condition: format!("match {}", cond),
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
                if trimmed.starts_with("if ") {
                    let cond = strip_keyword(trimmed, &["if "]);
                    push_conditional(&mut nodes, &lines, i, cond, line_num);
                } else if trimmed.starts_with("match ") {
                    let cond = strip_keyword(trimmed, &["match "]);
                    nodes.push(IrNode::Conditional {
                        condition: format!("match {}", cond),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn conditionals(nodes: &[IrNode]) -> Vec<&IrNode> {
        nodes.iter().filter(|n| matches!(n, IrNode::Conditional { .. })).collect()
    }

    #[test]
    fn test_if_else_populates_else_branch_not_a_phantom_conditional() {
        let src = "fn f(x: i32) -> i32 {\n    if x > 0 {\n        1\n    } else {\n        2\n    }\n}\n";
        let nodes = RustParser::parse(src);
        let IrNode::Function { body, .. } = &nodes[0] else { panic!("expected function") };
        let conds = conditionals(body);
        assert_eq!(conds.len(), 1, "the '}} else {{' line must not spawn its own Conditional");
        let IrNode::Conditional { else_branch, condition, .. } = conds[0] else { unreachable!() };
        assert!(!else_branch.is_empty(), "if/else must have a populated else_branch");
        assert_eq!(condition, "x > 0", "condition must not retain the 'if ' keyword or trailing brace");
    }

    #[test]
    fn test_guard_clause_with_return_has_populated_then_branch() {
        let src = "fn f(x: i32) -> Result<i32, String> {\n    if x < 0 {\n        return Err(\"negative\".to_string());\n    }\n    Ok(x)\n}\n";
        let nodes = RustParser::parse(src);
        let IrNode::Function { body, .. } = &nodes[0] else { panic!("expected function") };
        let conds = conditionals(body);
        assert_eq!(conds.len(), 1);
        let IrNode::Conditional { then_branch, .. } = conds[0] else { unreachable!() };
        assert!(!then_branch.is_empty(), "guard clause with return must populate then_branch");
    }

    #[test]
    fn test_condition_text_never_doubles_if_keyword() {
        let src = "fn f() {\n    if request.method() != Method::POST {\n        do_thing();\n    }\n}\n";
        let nodes = RustParser::parse(src);
        let IrNode::Function { body, .. } = &nodes[0] else { panic!("expected function") };
        let conds = conditionals(body);
        let IrNode::Conditional { condition, .. } = conds[0] else { unreachable!() };
        assert!(!condition.starts_with("if "), "condition retained the keyword: {condition:?}");
    }

    #[test]
    fn test_match_condition_tagged_for_gaps_analyzer_to_skip() {
        let src = "fn f(x: i32) {\n    match x {\n        0 => {}\n        _ => {}\n    }\n}\n";
        let nodes = RustParser::parse(src);
        let IrNode::Function { body, .. } = &nodes[0] else { panic!("expected function") };
        let conds = conditionals(body);
        assert_eq!(conds.len(), 1);
        let IrNode::Conditional { condition, .. } = conds[0] else { unreachable!() };
        assert!(condition.starts_with("match "), "match conditions must keep a 'match ' prefix so gaps.rs can skip them");
    }

    #[test]
    fn test_else_if_chain_each_link_gets_its_own_conditional_and_prior_gets_else() {
        let src = "fn f(x: i32) -> i32 {\n    if x > 10 {\n        1\n    } else if x > 0 {\n        2\n    } else {\n        3\n    }\n}\n";
        let nodes = RustParser::parse(src);
        let IrNode::Function { body, .. } = &nodes[0] else { panic!("expected function") };
        let conds = conditionals(body);
        assert_eq!(conds.len(), 2, "if + else-if = 2 conditionals, no phantom for the trailing else");
        for c in &conds {
            let IrNode::Conditional { else_branch, .. } = c else { unreachable!() };
            assert!(!else_branch.is_empty(), "every link in the chain must see the else that follows it");
        }
    }
}
