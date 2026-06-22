use crate::graph::{BbId, Block, Cfg};
use crate::ir::IrNode;

/// Build a simple CFG — currently a pass-through that collects all nodes.
/// The real complexity analysis is computed directly from the IR tree.
pub fn build_cfg(nodes: &[IrNode]) -> Cfg {
    let mut all = Vec::new();
    flatten_for_cfg(nodes, &mut all);

    Cfg {
        blocks: vec![Block {
            id: BbId(0),
            nodes: all,
            edges: vec![],
        }],
        entry: BbId(0),
    }
}

fn flatten_for_cfg(nodes: &[IrNode], out: &mut Vec<IrNode>) {
    for node in nodes {
        match node {
            IrNode::Function { body, .. } | IrNode::Closure { body, .. } => {
                flatten_for_cfg(body, out);
            }
            other => {
                out.push(other.clone());
            }
        }
    }
}

/// Compute cyclomatic complexity directly from the IR tree.
/// M = 1 + number of branch points (conditionals + loops + functions)
pub fn cyclomatic_complexity_from_ir(nodes: &[IrNode]) -> usize {
    fn walk(nodes: &[IrNode], count: &mut usize) {
        for node in nodes {
            match node {
                IrNode::Conditional {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    *count += 1;
                    walk(then_branch, count);
                    walk(else_branch, count);
                }
                IrNode::Loop { body, .. } => {
                    *count += 1;
                    walk(body, count);
                }
                IrNode::Function { body, .. } | IrNode::Closure { body, .. } => {
                    walk(body, count);
                }
                _ => {}
            }
        }
    }
    let mut count = 1;
    walk(nodes, &mut count);
    count
}

/// Legacy: compute from CFG graph
pub fn cyclomatic_complexity(cfg: &Cfg) -> usize {
    let e = cfg.blocks.iter().map(|b| b.edges.len()).sum::<usize>();
    let n = cfg.blocks.len();
    if e >= n {
        e - n + 2
    } else {
        1
    }
}

/// Compute nesting depth from a tree of IR nodes
pub fn nesting_depth(nodes: &[IrNode]) -> usize {
    fn max_depth(nodes: &[IrNode], depth: usize) -> usize {
        let mut max = depth;
        for node in nodes {
            match node {
                IrNode::Conditional {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    let d = (depth + 1)
                        .max(max_depth(then_branch, depth + 1))
                        .max(max_depth(else_branch, depth + 1));
                    max = max.max(d);
                }
                IrNode::Loop { body, .. } => {
                    let d = (depth + 1).max(max_depth(body, depth + 1));
                    max = max.max(d);
                }
                IrNode::Function { body, .. } | IrNode::Closure { body, .. } => {
                    let d = max_depth(body, depth);
                    max = max.max(d);
                }
                _ => {}
            }
        }
        max
    }
    max_depth(nodes, 0)
}

/// Cognitive complexity: each nesting level adds weight.
pub fn cognitive_complexity(nodes: &[IrNode]) -> usize {
    fn walk(nodes: &[IrNode], nesting: usize) -> usize {
        let mut total = 0;
        for node in nodes {
            match node {
                IrNode::Conditional {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    total += 1 + nesting;
                    total += walk(then_branch, nesting + 1);
                    total += walk(else_branch, nesting + 1);
                }
                IrNode::Loop { body, .. } => {
                    total += 2 + nesting;
                    total += walk(body, nesting + 1);
                }
                IrNode::Function { body, .. } | IrNode::Closure { body, .. } => {
                    total += walk(body, nesting);
                }
                _ => {}
            }
        }
        total
    }
    walk(nodes, 0)
}
