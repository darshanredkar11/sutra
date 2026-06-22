use std::fs;
use std::path::Path;

use uuid::Uuid;

use crate::analysis::computation::ComputationAnalyzer;
use crate::analysis::complexity::ComplexityAnalyzer;
use crate::analysis::gaps::GapsAnalyzer;
use crate::analysis::memory::MemoryAnalyzer;
use crate::analysis::Analyzer;
use crate::config::{Config, SeverityFilter};
use crate::findings::{compute_health_score, count_severities};
use crate::graph::{cfg::build_cfg, dataflow::build_def_use, refgraph::build_ref_graph, Graphs};
use crate::ir::{AnalysisFile, AnalysisResult, AllocSite, AnalysisSummary, Finding, FuncSignature, IrNode, Metrics};
use crate::output::{format_json, format_pretty, format_quiet};
use crate::parser;

pub struct Scanner {
    config: Config,
}

impl Scanner {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn scan_path(&self, path_str: &str) -> Result<String, String> {
        let result = self.analyze(path_str)?;
        let output = match self.config.output_format {
            crate::config::OutputFormat::Pretty => format_pretty(&result),
            crate::config::OutputFormat::Json => format_json(&result),
            crate::config::OutputFormat::Quiet => format_quiet(&result),
        };
        Ok(output)
    }

    pub fn analyze(&self, path_str: &str) -> Result<AnalysisResult, String> {
        let path = Path::new(path_str);
        if !path.exists() {
            return Err(format!("Path '{}' does not exist", path_str));
        }

        let mut files = Vec::new();

        if path.is_dir() {
            self.scan_dir(path, &mut files)?;
        } else if path.is_file() {
            self.scan_file(path, &mut files)?;
        } else {
            return Err(format!("Path '{}' is not a file or directory", path_str));
        }

        let analysis_id = Uuid::new_v4().to_string();
        let (errors, warnings, info) = count_severities(&files.iter().flat_map(|f| f.findings.clone()).collect::<Vec<_>>());
        let total_files = files.len();
        let total_findings = errors + warnings + info;
        let overall_health = if files.is_empty() {
            1.0
        } else {
            files.iter().map(|f| f.health_score).sum::<f64>() / total_files as f64
        };

        Ok(AnalysisResult {
            version: env!("CARGO_PKG_VERSION").to_string(),
            analysis_id,
            config: self.config.to_map(),
            files,
            summary: AnalysisSummary {
                total_files,
                total_findings,
                errors,
                warnings,
                info,
                overall_health,
            },
        })
    }

    fn scan_dir(&self, dir: &Path, results: &mut Vec<AnalysisFile>) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            if path.is_dir() {
                // Skip hidden directories and node_modules
                if let Some(name) = path.file_name() {
                    let name = name.to_string_lossy();
                    if name.starts_with('.') || name == "node_modules" || name == "__pycache__" {
                        continue;
                    }
                }
                self.scan_dir(&path, results)?;
            } else if path.is_file() {
                self.scan_file(&path, results)?;
            }
        }
        Ok(())
    }

    fn scan_file(&self, path: &Path, results: &mut Vec<AnalysisFile>) -> Result<(), String> {
        let path_str = path.to_string_lossy().to_string();
        let source = fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path_str, e))?;

        let parsed = parser::parse_file(&path_str, &source);
        let (nodes, language) = match parsed {
            Some(p) => p,
            None => return Ok(()), // unsupported language, skip
        };

        // Extract function signatures and allocations
        let mut functions = Vec::new();
        let mut alloc_sites = Vec::new();
        let mut closure_count = 0;
        let mut loop_count = 0;

        extract_metadata(&nodes, &mut functions, &mut alloc_sites, &mut closure_count, &mut loop_count);

        // Build graphs
        let cfg = build_cfg(&nodes);
        let def_use = build_def_use(&nodes);
        let ref_graph = build_ref_graph(&nodes);
        let graphs = Graphs {
            cfg,
            def_use,
            ref_graph,
        };

        // Run analyzers
        let mut findings = Vec::new();
        let mut metrics = Metrics::default();

        if self.config.complexity {
            let analyzer = ComplexityAnalyzer;
            let f = analyzer.analyze(&nodes, &graphs, &path_str);
            metrics.cyclomatic_max = crate::graph::cfg::cyclomatic_complexity_from_ir(&nodes);
            metrics.cognitive_max = crate::graph::cfg::cognitive_complexity(&nodes);
            metrics.nesting_depth_max = crate::graph::cfg::nesting_depth(&nodes);
            findings.extend(f);
        }

        if self.config.gaps {
            let analyzer = GapsAnalyzer;
            let f = analyzer.analyze(&nodes, &graphs, &path_str);
            metrics.missing_branches = f.iter().filter(|ff| ff.subtype == "missing_branch").count();
            metrics.unhandled_null_paths = f.iter().filter(|ff| ff.subtype == "unhandled_null_path").count();
            findings.extend(f);
        }

        if self.config.computation {
            let analyzer = ComputationAnalyzer;
            let f = analyzer.analyze(&nodes, &graphs, &path_str);
            metrics.loop_nest_max = crate::analysis::computation::max_loop_nesting(&nodes);
            metrics.recursion_depth_max = f.iter().filter(|ff| ff.subtype == "recursive_function").count();
            findings.extend(f);
        }

        if self.config.memory {
            let analyzer = MemoryAnalyzer;
            let f = analyzer.analyze(&nodes, &graphs, &path_str);
            metrics.resource_risks = f.iter().filter(|ff| ff.severity == "error").count();
            metrics.closure_captures = f.iter().filter(|ff| ff.subtype == "closure_capture").count();
            findings.extend(f);
        }

        // Filter by severity
        let filtered: Vec<Finding> = findings
            .into_iter()
            .filter(|f| {
                match self.config.min_severity {
                    SeverityFilter::Error => f.severity == "error",
                    SeverityFilter::Warning => f.severity == "error" || f.severity == "warning",
                    SeverityFilter::Info => true,
                }
            })
            .collect();

        let health_score = compute_health_score(&metrics, &filtered);

        results.push(AnalysisFile {
            path: path_str,
            language: language.to_string(),
            functions,
            nodes,
            alloc_sites,
            findings: filtered,
            metrics,
            health_score,
            closure_count,
            loop_count,
            recursion_count: 0,
        });

        Ok(())
    }
}

fn extract_metadata(
    nodes: &[IrNode],
    functions: &mut Vec<FuncSignature>,
    alloc_sites: &mut Vec<AllocSite>,
    closure_count: &mut usize,
    loop_count: &mut usize,
) {
    for node in nodes {
        match node {
            IrNode::Function { name, params, body, .. } => {
                functions.push(FuncSignature {
                    name: name.clone(),
                    params: params.clone(),
                    line: node.line(),
                });
                extract_metadata(body, functions, alloc_sites, closure_count, loop_count);
            }
            IrNode::Closure { body, .. } => {
                *closure_count += 1;
                extract_metadata(body, functions, alloc_sites, closure_count, loop_count);
            }
            IrNode::Alloc {
                target,
                resource,
                line,
            } => {
                alloc_sites.push(AllocSite {
                    target: target.clone(),
                    resource: resource.clone(),
                    line: *line,
                    paired: vec![],
                    escaped: false,
                });
            }
            IrNode::Loop { body, .. } => {
                *loop_count += 1;
                extract_metadata(body, functions, alloc_sites, closure_count, loop_count);
            }
            IrNode::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                extract_metadata(then_branch, functions, alloc_sites, closure_count, loop_count);
                extract_metadata(else_branch, functions, alloc_sites, closure_count, loop_count);
            }
            _ => {}
        }
    }
}
