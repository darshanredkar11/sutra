use std::collections::HashMap;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;

use crate::types::{DepEdge, DepNode, DependencyGraph};

pub struct PetGraphWrapper {
    pub graph: DiGraph<DepNode, DepEdge>,
    pub node_map: HashMap<String, NodeIndex>,
}

impl PetGraphWrapper {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: DepNode) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(&node.id) {
            return idx;
        }
        let idx = self.graph.add_node(node.clone());
        self.node_map.insert(node.id, idx);
        idx
    }

    pub fn add_edge(
        &mut self,
        source_id: &str,
        target_id: &str,
        edge: DepEdge,
    ) -> Option<()> {
        let source = *self.node_map.get(source_id)?;
        let target = *self.node_map.get(target_id)?;
        self.graph.add_edge(source, target, edge);
        Some(())
    }

    pub fn node_index(&self, id: &str) -> Option<NodeIndex> {
        self.node_map.get(id).copied()
    }

    pub fn find_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut index = 0usize;
        let mut indices: HashMap<NodeIndex, usize> = HashMap::new();
        let mut lowlink: HashMap<NodeIndex, usize> = HashMap::new();
        let mut on_stack: HashMap<NodeIndex, bool> = HashMap::new();
        let mut stack: Vec<NodeIndex> = Vec::new();

        for node in self.graph.node_indices() {
            indices.insert(node, usize::MAX);
            lowlink.insert(node, usize::MAX);
            on_stack.insert(node, false);
        }

        fn strongconnect(
            v: NodeIndex,
            graph: &DiGraph<DepNode, DepEdge>,
            index: &mut usize,
            indices: &mut HashMap<NodeIndex, usize>,
            lowlink: &mut HashMap<NodeIndex, usize>,
            stack: &mut Vec<NodeIndex>,
            on_stack: &mut HashMap<NodeIndex, bool>,
            cycles: &mut Vec<Vec<String>>,
        ) {
            indices.insert(v, *index);
            lowlink.insert(v, *index);
            *index += 1;
            stack.push(v);
            on_stack.insert(v, true);

            for edge in graph.edges_directed(v, petgraph::Direction::Outgoing) {
                let w = edge.target();
                if indices[&w] == usize::MAX {
                    strongconnect(w, graph, index, indices, lowlink, stack, on_stack, cycles);
                    let v_lowlink = lowlink[&v].min(lowlink[&w]);
                    lowlink.insert(v, v_lowlink);
                } else if on_stack[&w] {
                    let v_lowlink = lowlink[&v].min(indices[&w]);
                    lowlink.insert(v, v_lowlink);
                }
            }

            if lowlink[&v] == indices[&v] {
                let mut component = Vec::new();
                loop {
                    let w = stack.pop().unwrap();
                    on_stack.insert(w, false);
                    component.push(graph[w].id.clone());
                    if w == v {
                        break;
                    }
                }
                if component.len() > 1 {
                    cycles.push(component);
                }
            }
        }

        for v in self.graph.node_indices() {
            if indices[&v] == usize::MAX {
                strongconnect(
                    v, &self.graph, &mut index, &mut indices, &mut lowlink,
                    &mut stack, &mut on_stack, &mut cycles,
                );
            }
        }

        cycles
    }

    pub fn compute_fan_in_out(&self) -> (HashMap<String, usize>, HashMap<String, usize>) {
        let mut fan_in: HashMap<String, usize> = HashMap::new();
        let mut fan_out: HashMap<String, usize> = HashMap::new();

        for node in self.graph.node_indices() {
            let id = self.graph[node].id.clone();
            let in_count = self
                .graph
                .edges_directed(node, petgraph::Direction::Incoming)
                .count();
            let out_count = self
                .graph
                .edges_directed(node, petgraph::Direction::Outgoing)
                .count();
            if in_count > 0 {
                fan_in.insert(id.clone(), in_count);
            }
            if out_count > 0 {
                fan_out.insert(id, out_count);
            }
        }

        (fan_in, fan_out)
    }

    pub fn to_dependency_graph(&self) -> DependencyGraph {
        let nodes: Vec<DepNode> = self.graph.node_weights().cloned().collect();
        let edges: Vec<DepEdge> = self.graph.edge_weights().cloned().collect();
        let cycles = self.find_cycles();
        let (fan_in, fan_out) = self.compute_fan_in_out();

        DependencyGraph {
            nodes,
            edges,
            cycles,
            fan_in,
            fan_out,
        }
    }
}

impl Default for PetGraphWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ImportKind;

    fn make_node(id: &str) -> DepNode {
        DepNode {
            id: id.to_string(),
            file_path: format!("{}.py", id),
            module_name: id.to_string(),
            language: "python".into(),
        }
    }

    fn make_edge(source: &str, target: &str) -> DepEdge {
        DepEdge {
            source_id: source.to_string(),
            target_id: target.to_string(),
            line: 1,
            kind: ImportKind::Static,
        }
    }

    #[test]
    fn test_empty_graph() {
        let pg = PetGraphWrapper::new();
        assert_eq!(pg.graph.node_count(), 0);
        assert_eq!(pg.graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut pg = PetGraphWrapper::new();
        let idx = pg.add_node(make_node("a"));
        assert_eq!(pg.graph.node_count(), 1);
        assert_eq!(pg.graph[idx].id, "a");
    }

    #[test]
    fn test_add_node_dedup() {
        let mut pg = PetGraphWrapper::new();
        let idx1 = pg.add_node(make_node("a"));
        let idx2 = pg.add_node(make_node("a"));
        assert_eq!(idx1, idx2);
        assert_eq!(pg.graph.node_count(), 1);
    }

    #[test]
    fn test_add_edge() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("a"));
        pg.add_node(make_node("b"));
        pg.add_edge("a", "b", make_edge("a", "b"));
        assert_eq!(pg.graph.edge_count(), 1);
    }

    #[test]
    fn test_add_edge_missing_source() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("b"));
        let result = pg.add_edge("a", "b", make_edge("a", "b"));
        assert!(result.is_none());
    }

    #[test]
    fn test_no_cycles() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("a"));
        pg.add_node(make_node("b"));
        pg.add_node(make_node("c"));
        pg.add_edge("a", "b", make_edge("a", "b"));
        pg.add_edge("b", "c", make_edge("b", "c"));
        let cycles = pg.find_cycles();
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_simple_cycle() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("a"));
        pg.add_node(make_node("b"));
        pg.add_edge("a", "b", make_edge("a", "b"));
        pg.add_edge("b", "a", make_edge("b", "a"));
        let cycles = pg.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);
    }

    #[test]
    fn test_triangular_cycle() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("a"));
        pg.add_node(make_node("b"));
        pg.add_node(make_node("c"));
        pg.add_edge("a", "b", make_edge("a", "b"));
        pg.add_edge("b", "c", make_edge("b", "c"));
        pg.add_edge("c", "a", make_edge("c", "a"));
        let cycles = pg.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn test_self_loop_ignored() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("a"));
        pg.add_edge("a", "a", make_edge("a", "a"));
        let cycles = pg.find_cycles();
        assert!(cycles.is_empty(), "self-loops should not count as cycles");
    }

    #[test]
    fn test_fan_in_out() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("a"));
        pg.add_node(make_node("b"));
        pg.add_node(make_node("c"));
        pg.add_edge("a", "b", make_edge("a", "b"));
        pg.add_edge("c", "b", make_edge("c", "b"));
        let (fi, fo) = pg.compute_fan_in_out();
        assert_eq!(fi.get("b"), Some(&2));
        assert_eq!(fo.get("a"), Some(&1));
        assert_eq!(fo.get("c"), Some(&1));
        assert!(fo.get("b").is_none() || fo.get("b") == Some(&0));
    }

    #[test]
    fn test_to_dependency_graph() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("a"));
        pg.add_node(make_node("b"));
        pg.add_edge("a", "b", make_edge("a", "b"));
        pg.add_edge("b", "a", make_edge("b", "a"));
        let dg = pg.to_dependency_graph();
        assert_eq!(dg.node_count(), 2);
        assert_eq!(dg.edge_count(), 2);
        assert_eq!(dg.cycle_count(), 1);
    }

    #[test]
    fn test_empty_graph_returns_no_cycles() {
        let pg = PetGraphWrapper::new();
        let cycles = pg.find_cycles();
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_single_node_no_cycle() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("a"));
        let cycles = pg.find_cycles();
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_two_nodes_no_edge_no_cycle() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("a"));
        pg.add_node(make_node("b"));
        let cycles = pg.find_cycles();
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_large_cycle_100_nodes() {
        let mut pg = PetGraphWrapper::new();
        for i in 0..100 {
            pg.add_node(make_node(&format!("n{}", i)));
        }
        for i in 0..100 {
            let next = (i + 1) % 100;
            pg.add_edge(
                &format!("n{}", i),
                &format!("n{}", next),
                make_edge(&format!("n{}", i), &format!("n{}", next)),
            );
        }
        let cycles = pg.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 100);
    }

    #[test]
    fn test_multiple_independent_cycles() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("a"));
        pg.add_node(make_node("b"));
        pg.add_edge("a", "b", make_edge("a", "b"));
        pg.add_edge("b", "a", make_edge("b", "a"));
        pg.add_node(make_node("c"));
        pg.add_node(make_node("d"));
        pg.add_node(make_node("e"));
        pg.add_edge("c", "d", make_edge("c", "d"));
        pg.add_edge("d", "e", make_edge("d", "e"));
        pg.add_edge("e", "c", make_edge("e", "c"));
        let cycles = pg.find_cycles();
        assert_eq!(cycles.len(), 2);
    }

    #[test]
    fn test_diamond_shape_no_cycle() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("a"));
        pg.add_node(make_node("b"));
        pg.add_node(make_node("c"));
        pg.add_node(make_node("d"));
        pg.add_edge("a", "b", make_edge("a", "b"));
        pg.add_edge("a", "c", make_edge("a", "c"));
        pg.add_edge("b", "d", make_edge("b", "d"));
        pg.add_edge("c", "d", make_edge("c", "d"));
        let cycles = pg.find_cycles();
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_complex_graph_with_cyclic_and_acyclic_parts() {
        let mut pg = PetGraphWrapper::new();
        pg.add_node(make_node("a"));
        pg.add_node(make_node("b"));
        pg.add_node(make_node("c"));
        pg.add_edge("a", "b", make_edge("a", "b"));
        pg.add_edge("b", "c", make_edge("b", "c"));
        pg.add_edge("c", "a", make_edge("c", "a"));
        pg.add_node(make_node("d"));
        pg.add_node(make_node("e"));
        pg.add_edge("c", "d", make_edge("c", "d"));
        pg.add_edge("d", "e", make_edge("d", "e"));
        pg.add_node(make_node("f"));
        pg.add_node(make_node("g"));
        pg.add_edge("f", "g", make_edge("f", "g"));
        let cycles = pg.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }
}
