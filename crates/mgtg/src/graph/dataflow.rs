use std::collections::{HashMap, HashSet};

use crate::ir::IrNode;

#[derive(Debug, Clone)]
pub struct DefUse {
    pub defs: HashMap<String, Vec<usize>>,
    pub uses: HashMap<String, Vec<usize>>,
    pub live_at_exit: HashSet<String>,
}

pub fn build_def_use(nodes: &[IrNode]) -> DefUse {
    let mut defs: HashMap<String, Vec<usize>> = HashMap::new();
    let mut uses: HashMap<String, Vec<usize>> = HashMap::new();
    let mut live_at_exit: HashSet<String> = HashSet::new();

    collect_def_uses(nodes, &mut defs, &mut uses, &mut live_at_exit);

    // Variables defined but also used (or returned) are live at exit
    for (var, def_lines) in &defs {
        if uses.contains_key(var) || live_at_exit.contains(var) {
            for _line in def_lines {
                live_at_exit.insert(var.clone());
            }
        }
    }

    DefUse {
        defs,
        uses,
        live_at_exit,
    }
}

fn collect_def_uses(
    nodes: &[IrNode],
    defs: &mut HashMap<String, Vec<usize>>,
    uses: &mut HashMap<String, Vec<usize>>,
    live: &mut HashSet<String>,
) {
    for node in nodes {
        match node {
            IrNode::Assignment { target, source, line } => {
                defs.entry(target.clone())
                    .or_default()
                    .push(*line);
                // source may reference variables
                for token in source.split_whitespace() {
                    let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    if !clean.is_empty() && !is_keyword(clean) && !is_number(clean) {
                        uses.entry(clean.to_string())
                            .or_default()
                            .push(*line);
                    }
                }
            }
            IrNode::Return { value, line } => {
                if let Some(val) = value {
                    for token in val.split_whitespace() {
                        let clean =
                            token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                        if !clean.is_empty() && !is_keyword(clean) && !is_number(clean) {
                            uses.entry(clean.to_string())
                                .or_default()
                                .push(*line);
                            live.insert(clean.to_string());
                        }
                    }
                }
            }
            IrNode::Alloc { target, .. } => {
                defs.entry(target.clone()).or_default().push(node.line());
            }
            IrNode::Call { args, line, .. } => {
                for arg in args {
                    let clean =
                        arg.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    if !clean.is_empty() && !is_keyword(clean) && !is_number(clean) {
                        uses.entry(clean.to_string()).or_default().push(*line);
                    }
                }
            }
            IrNode::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                for token in condition.split_whitespace() {
                    let clean =
                        token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    if !clean.is_empty() && !is_keyword(clean) && !is_number(clean) {
                        uses.entry(clean.to_string()).or_default().push(node.line());
                    }
                }
                collect_def_uses(then_branch, defs, uses, live);
                collect_def_uses(else_branch, defs, uses, live);
            }
            IrNode::Loop { body, .. } => {
                collect_def_uses(body, defs, uses, live);
            }
            IrNode::Function { body, .. } => {
                collect_def_uses(body, defs, uses, live);
            }
            IrNode::Closure { body, captures, line } => {
                for cap in captures {
                    uses.entry(cap.clone()).or_default().push(*line);
                }
                collect_def_uses(body, defs, uses, live);
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
    s.parse::<f64>().is_ok() || s.starts_with('\'') || s.starts_with('"')
}
