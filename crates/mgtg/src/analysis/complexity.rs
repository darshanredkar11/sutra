use crate::analysis::Analyzer;
use crate::findings::finding_id;
use crate::graph::cfg::{cognitive_complexity, cyclomatic_complexity_from_ir, nesting_depth};
use crate::graph::Graphs;
use crate::ir::{Finding, IrNode};

pub struct ComplexityAnalyzer;

impl Analyzer for ComplexityAnalyzer {
    fn analyze(&self, nodes: &[IrNode], _graphs: &Graphs, file: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        let cc = cyclomatic_complexity_from_ir(nodes);
        let nc = nesting_depth(nodes);
        let cog = cognitive_complexity(nodes);

        if cc > 10 {
            findings.push(Finding {
                id: finding_id("C", 0),
                category: "complexity".to_string(),
                subtype: "high_cyclomatic".to_string(),
                severity: "warning".to_string(),
                file: file.to_string(),
                line: 1,
                column: 0,
                message: format!("Cyclomatic complexity is {} (threshold: 10)", cc),
                graph_ref: None,
            });
        }

        if nc > 4 {
            findings.push(Finding {
                id: finding_id("C", 1),
                category: "complexity".to_string(),
                subtype: "deep_nesting".to_string(),
                severity: "info".to_string(),
                file: file.to_string(),
                line: 1,
                column: 0,
                message: format!("Maximum nesting depth is {} (threshold: 4)", nc),
                graph_ref: None,
            });
        }

        if cog > 20 {
            findings.push(Finding {
                id: finding_id("C", 2),
                category: "complexity".to_string(),
                subtype: "high_cognitive".to_string(),
                severity: "info".to_string(),
                file: file.to_string(),
                line: 1,
                column: 0,
                message: format!("Cognitive complexity is {} (threshold: 20)", cog),
                graph_ref: None,
            });
        }

        findings
    }

    fn name(&self) -> &'static str {
        "complexity"
    }
}
