pub mod complexity;
pub mod gaps;
pub mod computation;
pub mod memory;

use crate::ir::Finding;
use crate::graph::Graphs;
use crate::ir::IrNode;

pub trait Analyzer {
    fn analyze(&self, nodes: &[IrNode], graphs: &Graphs, file: &str) -> Vec<Finding>;
    fn name(&self) -> &'static str;
}
