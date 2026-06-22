use std::collections::HashMap;

use crate::ir::{IrNode, ResourceKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceState {
    Allocated,
    Opened,
    Closed,
    Released,
    Disconnected,
    Leaked,
}

#[derive(Debug, Clone)]
pub struct ResourceFsm {
    pub resource: String,
    pub kind: ResourceKind,
    pub alloc_line: usize,
    pub state: ResourceState,
    pub transitions: Vec<(usize, String, ResourceState)>,
}

/// Build a vector of resource FSMs from IR nodes.
/// Tracks state transitions: Open→Close, Connect→Disconnect, Acquire→Release.
pub fn build_resource_fsms(nodes: &[IrNode]) -> Vec<ResourceFsm> {
    let mut allocs: Vec<(String, ResourceKind, usize)> = Vec::new();

    // Collect allocation sites
    collect_allocations(nodes, &mut allocs);

    let mut fsms: Vec<ResourceFsm> = allocs
        .into_iter()
        .map(|(name, kind, line)| ResourceFsm {
            resource: name,
            kind,
            alloc_line: line,
            state: ResourceState::Allocated,
            transitions: vec![],
        })
        .collect();

    // Build transition map: variable_name -> state index
    let mut var_map: HashMap<String, usize> = HashMap::new();
    for (i, fsm) in fsms.iter().enumerate() {
        var_map.insert(fsm.resource.clone(), i);
    }

    // Walk nodes to find pairing operations
    find_pairings(nodes, &mut fsms, &var_map);

    // Mark unpaired resources as leaked
    for fsm in &mut fsms {
        if fsm.state == ResourceState::Allocated || fsm.state == ResourceState::Opened {
            fsm.state = ResourceState::Leaked;
        }
    }

    fsms
}

fn collect_allocations(nodes: &[IrNode], allocs: &mut Vec<(String, ResourceKind, usize)>) {
    for node in nodes {
        match node {
            IrNode::Alloc {
                target,
                resource,
                line,
            } => {
                allocs.push((target.clone(), resource.clone(), *line));
            }
            IrNode::Call { name, args, line } => {
                let lower = name.to_lowercase();
                if lower.contains("open") || lower.contains("connect") || lower.contains("fopen")
                {
                    if let Some(first) = args.first() {
                        allocs.push((first.clone(), ResourceKind::File, *line));
                    }
                }
            }
            IrNode::Function { body, .. } | IrNode::Closure { body, .. } => {
                collect_allocations(body, allocs);
            }
            IrNode::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                collect_allocations(then_branch, allocs);
                collect_allocations(else_branch, allocs);
            }
            IrNode::Loop { body, .. } => {
                collect_allocations(body, allocs);
            }
            _ => {}
        }
    }
}

fn find_pairings(
    nodes: &[IrNode],
    fsms: &mut [ResourceFsm],
    var_map: &HashMap<String, usize>,
) {
    for node in nodes {
        match node {
            IrNode::Call { name, line, .. } => {
                let lower = name.to_lowercase();
                // Check method calls like file.close(), conn.disconnect()
                if let Some(dot) = name.find('.') {
                    let var = name[..dot].to_string();
                    let method = name[dot + 1..].to_string();
                    if let Some(&idx) = var_map.get(&var) {
                        let method_lower = method.to_lowercase();
                        if method_lower == "close" || method_lower == "release" {
                            fsms[idx].state = ResourceState::Closed;
                            fsms[idx].transitions.push((*line, method_lower, ResourceState::Closed));
                        } else if method_lower == "disconnect" {
                            fsms[idx].state = ResourceState::Disconnected;
                            fsms[idx].transitions.push((*line, method_lower, ResourceState::Disconnected));
                        }
                    }
                }
                // Free-standing calls like close(file), disconnect(socket)
                if lower == "close" || lower == "release" || lower == "disconnect" {
                    for arg in &node.args() {
                        if let Some(&idx) = var_map.get(arg) {
                            fsms[idx].state = ResourceState::Closed;
                            fsms[idx].transitions.push((*line, lower.clone(), ResourceState::Closed));
                        }
                    }
                }
            }
            IrNode::Function { body, .. } | IrNode::Closure { body, .. } => {
                find_pairings(body, fsms, var_map);
            }
            IrNode::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                find_pairings(then_branch, fsms, var_map);
                find_pairings(else_branch, fsms, var_map);
            }
            IrNode::Loop { body, .. } => {
                find_pairings(body, fsms, var_map);
            }
            _ => {}
        }
    }
}

// Helper to get args from Call
impl IrNode {
    pub fn args(&self) -> Vec<String> {
        if let IrNode::Call { args, .. } = self {
            args.clone()
        } else {
            vec![]
        }
    }
}
