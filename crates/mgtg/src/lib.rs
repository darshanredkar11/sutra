pub mod ir;
pub mod config;
pub mod findings;
pub mod output;
pub mod scanner;
pub mod parser;
pub mod graph;
pub mod analysis;

#[cfg(feature = "wasm")]
pub mod wasm;

use config::Config;
use scanner::Scanner;
use ir::AnalysisResult;

/// Analyze a file or directory and return structured results.
/// This is the main library entry point for agent/LLM integration.
pub fn analyze(path: &str, config: Option<Config>) -> Result<AnalysisResult, String> {
    let cfg = config.unwrap_or_default();
    let scanner = Scanner::new(cfg);
    scanner.analyze(path)
}

/// Quick health score for a file or directory. Returns 0.0–1.0.
pub fn health_score(path: &str) -> Result<f64, String> {
    let mut config = Config::default();
    config.output_format = crate::config::OutputFormat::Quiet;
    let result = analyze(path, Some(config))?;
    Ok(result.summary.overall_health)
}
