use std::path::PathBuf;

use clap::{Parser, Subcommand};
use sutra_ci::sarif;
use sutra_common::error::SutraResult;
use sutra_llm::types::LLMConfig;
use sutra_orchestrator::coordinator::Orchestrator;
use sutra_schema::v1::{AnalysisResult, AnalyzeRequest, Engine};

#[derive(Parser)]
#[command(name = "sutra", version, about = "Predict production software failures")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run analysis on a repository
    Analyze {
        /// Path to the repository
        path: PathBuf,
        /// Engine(s) to run (mgtg, dependency, process, ml, all)
        #[arg(short, long, default_value = "all")]
        engine: String,
        /// Output format (pretty, json, sarif)
        #[arg(short, long, default_value = "pretty")]
        format: String,
        /// Commit hash to analyze
        #[arg(short, long, default_value = "HEAD")]
        commit: String,
        /// Path to architecture TOML (dependency engine)
        #[arg(long)]
        arch: Option<PathBuf>,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Enable LLM validation (uses Ollama at localhost:11434)
        #[arg(long)]
        llm: bool,
        /// LLM model name (default: llama3.2)
        #[arg(long, default_value = "llama3.2")]
        llm_model: String,
        /// Ollama server URL
        #[arg(long, default_value = "http://localhost:11434")]
        ollama_url: String,
    },
    /// Quick health score for a repository
    Health {
        /// Path to the repository
        path: PathBuf,
    },
    /// Start the HTTP API server
    Server {
        /// Port to listen on (default: $PORT env var or 8080)
        #[arg(short, long)]
        port: Option<u16>,
    },
}

fn build_orchestrator(arch_path: Option<PathBuf>) -> Orchestrator {
    let mut o = Orchestrator::new();
    o.register(Engine::Mgtg, Box::new(sutra_mgtg::engine::MgtgEngine::new()));

    let dep_engine = if let Some(ref arch_path) = arch_path {
        match std::fs::read_to_string(arch_path) {
            Ok(toml) => sutra_dependency::engine::DependencyEngine::new().with_architecture(&toml),
            Err(e) => {
                eprintln!("warning: cannot read arch config '{}': {}", arch_path.display(), e);
                sutra_dependency::engine::DependencyEngine::new()
            }
        }
    } else {
        sutra_dependency::engine::DependencyEngine::new()
    };
    o.register(Engine::Dependency, Box::new(dep_engine));

    o.register(
        Engine::Process,
        Box::new(sutra_process::engine::ProcessEngine::new()),
    );

    o.register(
        Engine::Ml,
        Box::new(sutra_ml::engine::MlEngine::new()),
    );

    o.register(
        Engine::Hitl,
        Box::new(sutra_hitl::engine::HitlEngine::new()),
    );

    o
}

fn parse_engines(s: &str) -> Vec<Engine> {
    match s.to_lowercase().as_str() {
        "all" => vec![Engine::Mgtg, Engine::Dependency, Engine::Process, Engine::Ml, Engine::Hitl],
        "mgtg" => vec![Engine::Mgtg],
        "dependency" | "dep" => vec![Engine::Dependency],
        "process" | "proc" => vec![Engine::Process],
        "ml" => vec![Engine::Ml],
        "hitl" => vec![Engine::Hitl],
        _ => {
            eprintln!("unknown engine '{}', running all", s);
            vec![Engine::Mgtg, Engine::Dependency, Engine::Process, Engine::Ml, Engine::Hitl]
        }
    }
}

fn format_output(result: &AnalysisResult, format: &str) -> String {
    match format {
        "json" => serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".into()),
        "sarif" => sarif::to_sarif_json(result),
        _ => format_pretty(result),
    }
}

fn format_pretty(result: &AnalysisResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("Sutra Analysis Report\n"));
    out.push_str(&format!("  Request: {}\n", result.request_id));
    out.push_str(&format!("  Commit:  {}\n", result.commit_hash));
    out.push_str(&format!("  Risk:    {:.2}\n", result.overall_risk));
    out.push_str(&format!("  Time:    {:.0}ms\n", result.processing_time_ms));
    out.push_str(&format!(
        "  Findings: {} ({} errors, {} warnings)\n",
        result.findings.len(),
        result.findings.iter().filter(|f| f.severity == sutra_schema::v1::Severity::Error).count(),
        result.findings.iter().filter(|f| f.severity == sutra_schema::v1::Severity::Warning).count(),
    ));

    if result.blocked_merge {
        out.push_str("  Merge:   BLOCKED\n");
    }

    if !result.findings.is_empty() {
        out.push_str("\nFindings:\n");
        for f in &result.findings {
            out.push_str(&format!(
                "  [{}] {}:{} {} — {}\n",
                f.id, f.file_path, f.line, f.message, format_severity(f.severity)
            ));
        }
    }

    if !result.recommendations.is_empty() {
        out.push_str("\nRecommendations:\n");
        for rec in &result.recommendations {
            out.push_str(&format!("  - {} ({:.0}%)\n", rec.text, rec.priority * 100.0));
        }
    }

    out
}

fn format_severity(s: sutra_schema::v1::Severity) -> &'static str {
    match s {
        sutra_schema::v1::Severity::Critical => "CRITICAL",
        sutra_schema::v1::Severity::Error => "ERROR",
        sutra_schema::v1::Severity::Warning => "WARNING",
        sutra_schema::v1::Severity::Info => "INFO",
    }
}

#[tokio::main]
async fn main() -> SutraResult<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze {
            path,
            engine,
            format,
            commit,
            arch,
            output,
            llm,
            llm_model,
            ollama_url,
        } => {
            let orchestrator = build_orchestrator(arch);
            let engines = parse_engines(&engine);

            let mut request = AnalyzeRequest::new(
                path.to_str().unwrap_or("."),
                &commit,
            );
            request.engines = engines;

            let result = if llm {
                let config = LLMConfig {
                    model: llm_model,
                    ollama_url,
                    ..LLMConfig::default()
                };
                sutra_llm::pipeline::analyze_with_llm(&orchestrator, &request, Some(config), 0.8)?
            } else {
                orchestrator.analyze(&request)?
            };

            let output_text = format_output(&result, &format);
            match output {
                Some(p) => {
                    std::fs::write(&p, &output_text)
                        .map_err(|e| sutra_common::error::SutraError::Io(e))?;
                    eprintln!("wrote output to {}", p.display());
                }
                None => {
                    println!("{}", output_text);
                }
            }
        }
        Commands::Health { path } => {
            let orchestrator = build_orchestrator(None);
            let engines = orchestrator.engine_names();
            println!("Sutra Health Check");
            println!("  Path: {}", path.display());
            println!("  Engines: {}", engines.join(", "));
            println!("  Status: OK");
        }
        Commands::Server { port } => {
            let port = port
                .or_else(|| std::env::var("PORT").ok().and_then(|p| p.parse().ok()))
                .unwrap_or(8080);
            let orchestrator = build_orchestrator(None);
            sutra_orchestrator::server::start_server(orchestrator, port).await?;
        }
    }

    Ok(())
}
