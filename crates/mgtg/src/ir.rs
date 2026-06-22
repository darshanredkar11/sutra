use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LoopKind {
    For,
    While,
    DoWhile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResourceKind {
    File,
    Socket,
    Lock,
    DbConnection,
    HttpClient,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IrNode {
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<IrNode>,
        line: usize,
    },
    Conditional {
        condition: String,
        then_branch: Vec<IrNode>,
        else_branch: Vec<IrNode>,
        line: usize,
    },
    Loop {
        kind: LoopKind,
        condition: String,
        body: Vec<IrNode>,
        line: usize,
    },
    Assignment {
        target: String,
        source: String,
        line: usize,
    },
    Call {
        name: String,
        args: Vec<String>,
        line: usize,
    },
    Alloc {
        target: String,
        resource: ResourceKind,
        line: usize,
    },
    Closure {
        captures: Vec<String>,
        body: Vec<IrNode>,
        line: usize,
    },
    Return {
        value: Option<String>,
        line: usize,
    },
    Variable {
        name: String,
        line: usize,
    },
    NullCheck {
        target: String,
        line: usize,
    },
    Unresolved {
        text: String,
        line: usize,
    },
}

impl IrNode {
    pub fn line(&self) -> usize {
        match self {
            IrNode::Function { line: l, .. }
            | IrNode::Conditional { line: l, .. }
            | IrNode::Loop { line: l, .. }
            | IrNode::Assignment { line: l, .. }
            | IrNode::Call { line: l, .. }
            | IrNode::Alloc { line: l, .. }
            | IrNode::Closure { line: l, .. }
            | IrNode::Return { line: l, .. }
            | IrNode::Variable { line: l, .. }
            | IrNode::NullCheck { line: l, .. }
            | IrNode::Unresolved { line: l, .. } => *l,
        }
    }

    pub fn all_lines(&self) -> Vec<usize> {
        let mut lines = vec![self.line()];
        match self {
            IrNode::Function { body, .. }
            | IrNode::Closure { body, .. } => {
                for child in body {
                    lines.extend(child.all_lines());
                }
            }
            IrNode::Conditional { then_branch, else_branch, .. } => {
                for child in then_branch {
                    lines.extend(child.all_lines());
                }
                for child in else_branch {
                    lines.extend(child.all_lines());
                }
            }
            IrNode::Loop { body, .. } => {
                for child in body {
                    lines.extend(child.all_lines());
                }
            }
            _ => {}
        }
        lines
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllocSite {
    pub target: String,
    pub resource: ResourceKind,
    pub line: usize,
    pub paired: Vec<usize>,
    pub escaped: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuncSignature {
    pub name: String,
    pub params: Vec<String>,
    pub line: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AnalysisFile {
    pub path: String,
    pub language: String,
    pub functions: Vec<FuncSignature>,
    pub nodes: Vec<IrNode>,
    pub alloc_sites: Vec<AllocSite>,
    pub findings: Vec<Finding>,
    pub metrics: Metrics,
    pub health_score: f64,
    pub closure_count: usize,
    pub loop_count: usize,
    pub recursion_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metrics {
    pub cyclomatic_max: usize,
    pub cyclomatic_total: usize,
    pub cognitive_max: usize,
    pub nesting_depth_max: usize,
    pub missing_branches: usize,
    pub unhandled_null_paths: usize,
    pub resource_risks: usize,
    pub loop_nest_max: usize,
    pub recursion_depth_max: usize,
    pub closure_captures: usize,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            cyclomatic_max: 0,
            cyclomatic_total: 0,
            cognitive_max: 0,
            nesting_depth_max: 0,
            missing_branches: 0,
            unhandled_null_paths: 0,
            resource_risks: 0,
            loop_nest_max: 0,
            recursion_depth_max: 0,
            closure_captures: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    pub id: String,
    pub category: String,
    pub subtype: String,
    pub severity: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub graph_ref: Option<GraphRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphRef {
    pub node: String,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisResult {
    pub version: String,
    pub analysis_id: String,
    pub config: HashMap<String, bool>,
    pub files: Vec<AnalysisFile>,
    pub summary: AnalysisSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisSummary {
    pub total_files: usize,
    pub total_findings: usize,
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    pub overall_health: f64,
}

use serde::{Deserialize, Serialize};
