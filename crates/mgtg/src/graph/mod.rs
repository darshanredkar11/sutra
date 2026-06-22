pub mod cfg;
pub mod dataflow;
pub mod refgraph;
pub mod fsm;

use crate::ir::IrNode;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BbId(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub enum EdgeKind {
    True,
    False,
    Unconditional,
    BackEdge,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub id: BbId,
    pub nodes: Vec<IrNode>,
    pub edges: Vec<(BbId, EdgeKind)>,
}

#[derive(Debug, Clone)]
pub struct Cfg {
    pub blocks: Vec<Block>,
    pub entry: BbId,
}

pub struct Graphs {
    pub cfg: Cfg,
    pub def_use: dataflow::DefUse,
    pub ref_graph: refgraph::RefGraph,
}
