use sutra_common::engine::AnalysisEngine;
use sutra_common::error::SutraResult;
use sutra_schema::v1::{
    AnalysisResult, AnalyzeRequest, Engine, Finding, Recommendation, Severity,
};

use crate::types::PerformanceConfig;

pub struct PerformanceEngine {
    config: PerformanceConfig,
}

impl PerformanceEngine {
    pub fn new() -> Self {
        Self {
            config: PerformanceConfig::default(),
        }
    }

    pub fn with_config(mut self, config: PerformanceConfig) -> Self {
        self.config = config;
        self
    }

    fn analyze_file(&self, content: &str, file_path: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        let functions = self.detect_functions(content);
        let io_patterns = self.detect_io_patterns(content);
        let allocation_patterns = self.detect_allocation_patterns(content);
        let nested_loops = self.detect_nested_loops(content);
        let sync_calls = self.detect_sync_blocking_calls(content);

        for func in &functions {
            if func.body_lines > 50 {
                let effort_hours = ((func.body_lines / 50) as f64 * 6.0).ceil() as u32;
                let optimization_potential = ((func.body_lines as f64 / 200.0) * 100.0).min(40.0) as u32;

                let spec = serde_json::json!({
                    "type": "split_large_function",
                    "current_state": {
                        "function_size_lines": func.body_lines,
                        "estimated_cyclomatic_complexity": "High (needs profiling)",
                        "cache_efficiency": "Low"
                    },
                    "proposed_state": {
                        "target_lines_per_function": 30,
                        "num_functions": (func.body_lines / 30).max(2),
                        "cache_efficiency": "Better"
                    },
                    "impact": {
                        "latency_reduction_percent": optimization_potential,
                        "cache_hit_rate_improvement": "20-30%",
                        "testability_improvement": "High",
                        "readability_improvement": "Very High"
                    },
                    "optimization_opportunities": [
                        "Reduce CPU cache misses by splitting into smaller functions",
                        "Enable better branch prediction",
                        "Improve SIMD vectorization opportunities",
                        "Enable better inlining decisions by compiler"
                    ],
                    "effort": {
                        "estimated_hours": effort_hours,
                        "complexity_of_refactor": "medium",
                        "risk_of_bugs": 0.10,
                        "profiling_required": true
                    },
                    "roi": {
                        "latency_reduction_percent": optimization_potential,
                        "roi_months": format!("{:.2}", (effort_hours as f64 * 100.0) / (optimization_potential.max(1) as f64 * 10.0)),
                        "priority": "medium"
                    }
                });

                findings.push(
                    Finding::new(
                        "PERF-LARGE-FUNC",
                        Engine::Performance,
                        file_path,
                        func.line,
                        &format!(
                            "Large function '{}' ({} lines). High maintenance cost and potential performance issues.",
                            func.name, func.body_lines
                        ),
                        Severity::Warning,
                    )
                    .with_fix("Split into smaller functions. Profile to identify hot spots.")
                    .with_spec_data(spec)
                    .with_confidence(0.82)
                    .with_edge_cases(vec![
                        "Profile before and after to confirm latency improvements".into(),
                        "Ensure inlining decisions don't negate benefits".into(),
                        "Watch for register pressure from additional function calls".into(),
                    ])
                );
            }
        }

        for func in &functions {
            let io_count = io_patterns.iter().filter(|io| {
                io.line >= func.line && io.line < func.line + func.body_lines
            }).count();
            if io_count > 2 {
                let effort_hours = ((io_count as f64 / 3.0) * 8.0).ceil() as u32;
                let latency_reduction = (io_count as f64 * 50.0) as u32; // ~50ms per batched I/O

                let spec = serde_json::json!({
                    "type": "batch_and_cache_io",
                    "current_state": {
                        "io_operations": io_count,
                        "roundtrips": io_count,
                        "latency_contribution": "High (likely 60-80% of function time)"
                    },
                    "proposed_state": {
                        "io_batches": (io_count / 3).max(1),
                        "cache_layers": 2,
                        "roundtrips": (io_count / 3).max(1)
                    },
                    "impact": {
                        "latency_reduction_ms": latency_reduction,
                        "throughput_improvement": "Very High",
                        "resource_utilization": "Better connection pooling"
                    },
                    "optimization_strategies": [
                        "Batch multiple I/O calls into single operation",
                        "Add local cache (L1) for repeated queries",
                        "Implement distributed cache (Redis) for cross-service cache",
                        "Use connection pooling to reduce overhead"
                    ],
                    "effort": {
                        "estimated_hours": effort_hours,
                        "complexity_of_refactor": "medium",
                        "infrastructure": "May need cache system",
                        "risk_of_bugs": 0.12
                    },
                    "roi": {
                        "latency_reduction_ms": latency_reduction,
                        "roi_months": format!("{:.2}", (effort_hours as f64 * 100.0) / (latency_reduction.max(1) as f64)),
                        "priority": "critical"
                    }
                });

                findings.push(
                    Finding::new(
                        "PERF-IO-HEAVY",
                        Engine::Performance,
                        file_path,
                        func.line,
                        &format!(
                            "I/O heavy function '{}' ({} I/O operations). Consider batching or caching.",
                            func.name, io_count
                        ),
                        Severity::Warning,
                    )
                    .with_fix("Batch I/O operations, add caching layer, or use connection pooling.")
                    .with_spec_data(spec)
                    .with_confidence(0.89)
                    .with_edge_cases(vec![
                        "Monitor cache hit rates to validate strategy effectiveness".into(),
                        "Implement cache invalidation strategy for stale data risks".into(),
                        "Consider eventual consistency implications for distributed cache".into(),
                    ])
                );
            }
        }

        for pair in &nested_loops {
            let depth = pair.depth;
            if depth >= 2 {
                let severity = if depth >= 3 { Severity::Error } else { Severity::Warning };
                let time_complexity_exponent = depth;
                let optimization_factor = (2_u64.pow(time_complexity_exponent as u32) as f64).log2() as u32;

                let spec = serde_json::json!({
                    "type": "reduce_nesting_depth",
                    "current_state": {
                        "nesting_depth": depth,
                        "time_complexity": format!("O(n^{})", depth),
                        "estimated_iterations": format!("10^{} with n=10", depth)
                    },
                    "proposed_state": {
                        "nesting_depth": 1,
                        "time_complexity": "O(n log n) or O(n)",
                        "algorithm": "Hash-based lookup or divide-and-conquer"
                    },
                    "impact": {
                        "speedup_factor": optimization_factor,
                        "latency_reduction_percent": 85,
                        "scalability": "Linear vs polynomial"
                    },
                    "optimization_approaches": [
                        "Use hash map for inner loop lookup (O(n) vs O(n²))",
                        "Extract inner loop into separate function",
                        "Restructure algorithm (sort + single pass vs nested iteration)",
                        "Use Set data structure for membership testing"
                    ],
                    "effort": {
                        "estimated_hours": if depth >= 3 { 16u32 } else { 8u32 },
                        "complexity_of_refactor": if depth >= 3 { "high" } else { "medium" },
                        "algorithmic_complexity": "May require algorithm redesign",
                        "risk_of_bugs": if depth >= 3 { 0.20 } else { 0.12 }
                    },
                    "roi": {
                        "speedup_factor": optimization_factor,
                        "roi_months": format!("{:.2}", 1.0),
                        "priority": if depth >= 3 { "critical" } else { "high" }
                    }
                });

                findings.push(
                    Finding::new(
                        "PERF-NESTED-LOOP",
                        Engine::Performance,
                        file_path,
                        pair.line,
                        &format!(
                            "Nested loop (depth {}) at line {}. Potential O(n^{}) bottleneck.",
                            depth, pair.line, depth
                        ),
                        severity,
                    )
                    .with_fix(&format!(
                        "Reduce nesting: extract inner loop, use hash maps, or restructure algorithm."
                    ))
                    .with_spec_data(spec)
                    .with_confidence(0.95)
                    .with_edge_cases(vec![
                        "Verify correctness after algorithm restructuring".into(),
                        "Profile new algorithm on realistic dataset sizes".into(),
                        "Test edge cases (empty collections, single element, etc.)".into(),
                    ])
                );
            }
        }

        if !io_patterns.is_empty() && !sync_calls.is_empty() {
            let sync_io_count = sync_calls.len();
            let latency_impact = (sync_io_count as f64 * 100.0) as u32; // ~100ms per blocking call
            let effort_hours = ((sync_io_count as f64 / 3.0) * 20.0).ceil() as u32;

            let spec = serde_json::json!({
                "type": "async_io_migration",
                "current_state": {
                    "blocking_io_calls": sync_io_count,
                    "threading_model": "Sync with thread pool (expensive)",
                    "latency_impact_ms": latency_impact
                },
                "proposed_state": {
                    "blocking_io_calls": 0,
                    "threading_model": "Async/await with event loop",
                    "connection_pooling": true
                },
                "impact": {
                    "latency_reduction_ms": latency_impact,
                    "throughput_improvement": "5-10x (eliminates thread blocking)",
                    "resource_utilization": "99% reduction in thread count",
                    "scalability": "Horizontal scaling enabled"
                },
                "migration_strategy": {
                    "phase_1": "Mark blocking functions with #[tokio::main] or similar",
                    "phase_2": "Convert I/O operations to async equivalents",
                    "phase_3": "Implement connection pooling (tokio-postgres, sqlx, etc.)",
                    "phase_4": "Add structured concurrency with select!, join!, etc.",
                    "phase_5": "Profile and optimize hot paths"
                },
                "effort": {
                    "estimated_hours": effort_hours,
                    "complexity_of_refactor": "high",
                    "infrastructure": "Async runtime (Tokio, async-std, etc.)",
                    "risk_of_bugs": 0.18
                },
                "roi": {
                    "latency_reduction_ms": latency_impact,
                    "throughput_improvement": "5-10x",
                    "roi_months": format!("{:.2}", (effort_hours as f64 * 100.0) / (latency_impact.max(1) as f64)),
                    "priority": "critical"
                }
            });

            findings.push(
                Finding::new(
                    "PERF-SYNC-IO",
                    Engine::Performance,
                    file_path,
                    sync_calls[0].line,
                    "Synchronous I/O calls detected. Consider async/await for non-blocking I/O.",
                    Severity::Warning,
                )
                .with_fix("Replace synchronous I/O with async equivalents. Use connection pooling.")
                .with_spec_data(spec)
                .with_confidence(0.94)
                .with_edge_cases(vec![
                    "Ensure all I/O operations are converted to async versions".into(),
                    "Watch for blocking operations hiding in async context (careful with spawn_blocking)".into(),
                    "Implement proper backpressure and flow control in async pipelines".into(),
                    "Test cancellation behavior and ensure cleanup on task cancellation".into(),
                ])
            );
        }

        for alloc in &allocation_patterns {
            if alloc.count > 10 {
                let gc_pressure = (alloc.count as f64 * 2.5) as u32; // ms of GC pause time
                let effort_hours = ((alloc.count as f64 / 15.0) * 10.0).ceil() as u32;

                let spec = serde_json::json!({
                    "type": "reduce_allocation_pressure",
                    "current_state": {
                        "allocations_per_call": alloc.count,
                        "gc_pause_time_ms": gc_pressure,
                        "heap_churn": "Very High"
                    },
                    "proposed_state": {
                        "allocations_per_call": (alloc.count / 3).max(1),
                        "gc_pause_time_ms": gc_pressure / 3,
                        "strategy": "Object pooling or stack allocation"
                    },
                    "impact": {
                        "gc_pause_reduction_ms": gc_pressure - (gc_pressure / 3),
                        "throughput_improvement": "20-40%",
                        "latency_variance_reduction": "Significant (fewer GC stalls)"
                    },
                    "optimization_strategies": [
                        "Implement object pooling for frequently allocated objects",
                        "Use stack allocation for small objects (Stack frame optimization)",
                        "Reuse buffers across function calls (buffer ring, thread-local storage)",
                        "Consider arena allocators for batch operations",
                        "Profile with heap analyzer to identify allocation patterns"
                    ],
                    "effort": {
                        "estimated_hours": effort_hours,
                        "complexity_of_refactor": "medium",
                        "profiling_required": true,
                        "risk_of_bugs": 0.14
                    },
                    "roi": {
                        "gc_pause_reduction_ms": gc_pressure - (gc_pressure / 3),
                        "roi_months": format!("{:.2}", (effort_hours as f64 * 100.0) / (gc_pressure as f64 + 1.0)),
                        "priority": "high"
                    }
                });

                findings.push(
                    Finding::new(
                        "PERF-ALLOC",
                        Engine::Performance,
                        file_path,
                        alloc.line,
                        &format!(
                            "Hot allocation site: {} allocations at line {}. GC pressure risk.",
                            alloc.count, alloc.line
                        ),
                        Severity::Warning,
                    )
                    .with_fix("Implement object pooling, use stack allocation, or reuse buffers to reduce heap churn.")
                    .with_spec_data(spec)
                    .with_confidence(0.87)
                    .with_edge_cases(vec![
                        "Profile memory usage before and after optimization".into(),
                        "Monitor GC pause time impact under production load".into(),
                        "Ensure thread safety if using thread-local pools".into(),
                    ])
                );
            }
        }

        if findings.len() > 5 {
            findings.truncate(5);
        }

        findings
    }

    fn detect_functions(&self, content: &str) -> Vec<FunctionProfile> {
        let mut functions = Vec::new();
        let fn_prefixes = ["fn ", "def ", "function ", "pub fn ", "pub async fn ", "async fn "];
        let mut in_fn = false;
        let mut fn_name = String::new();
        let mut fn_line = 0u32;
        let mut fn_body = 0u32;
        let mut brace_depth = 0;

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
                continue;
            }

            if !in_fn {
                let is_start = fn_prefixes.iter().any(|kw| trimmed.starts_with(kw));
                if !is_start { continue; }
                let before_paren = trimmed.split('(').next().unwrap_or("");
                fn_name = before_paren
                    .split_whitespace()
                    .filter(|s| *s != "fn" && *s != "def" && *s != "function"
                        && *s != "pub" && *s != "async" && *s != "unsafe" && *s != "pub(crate)")
                    .last()
                    .unwrap_or("anonymous")
                    .to_string();
                fn_line = (i + 1) as u32;

                if trimmed.contains('{') {
                    in_fn = true;
                    brace_depth = 1;
                    for ch in trimmed.chars().skip_while(|c| *c != '{').skip(1) {
                        if ch == '}' { brace_depth -= 1; }
                    }
                    if brace_depth == 0 {
                        in_fn = false;
                        functions.push(FunctionProfile {
                            name: fn_name.clone(),
                            line: fn_line,
                            body_lines: 0,
                        });
                    }
                }
                continue;
            }

            fn_body += 1;
            for ch in line.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            functions.push(FunctionProfile {
                                name: fn_name.clone(),
                                line: fn_line,
                                body_lines: fn_body,
                            });
                            in_fn = false;
                            fn_body = 0;
                            fn_name.clear();
                        }
                    }
                    _ => {}
                }
            }
        }
        functions
    }

    fn detect_io_patterns(&self, content: &str) -> Vec<IoCall> {
        let mut calls = Vec::new();
        let io_keywords = [
            ".read(", ".write(", ".query(", ".fetch(", ".execute(",
            "fs::read", "fs::write", "File::open", "fopen", "fread", "fwrite",
            "http::", "reqwest::", "curl", "axios.", "fetch(", "XMLHttpRequest",
            "database", "mongodb.", "postgres", "sqlite", "redis",
            "stdin", "stdout", "net::", "TcpStream", "UdpSocket",
        ];

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }
            if io_keywords.iter().any(|kw| trimmed.contains(kw)) {
                calls.push(IoCall { line: (i + 1) as u32 });
            }
        }
        calls
    }

    fn detect_allocation_patterns(&self, content: &str) -> Vec<AllocationSite> {
        let mut sites = Vec::new();
        let alloc_keywords = [
            "new ", "Box::new", "Rc::new", "Arc::new", "Vec::new",
            "String::new", "HashMap::new", "vec![", "format!(",
            "malloc", "calloc", "alloc",
            "new Array", "new Map", "new Object", "new Set",
            "new ", "clone()", "to_owned()", "to_string()",
        ];

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }
            let count = alloc_keywords.iter().filter(|kw| trimmed.contains(*kw)).count();
            if count > 0 {
                sites.push(AllocationSite {
                    line: (i + 1) as u32,
                    count: count as u32,
                });
            }
        }
        sites
    }

    fn detect_nested_loops(&self, content: &str) -> Vec<NestedLoop> {
        let mut loops = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut nesting = 0u32;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }

            let is_loop = trimmed.starts_with("for ")
                || trimmed.starts_with("while ")
                || trimmed.starts_with("loop ")
                || trimmed.starts_with("for(")
                || trimmed.starts_with("while(");

            if is_loop {
                nesting += 1;
                if nesting >= 2 {
                    loops.push(NestedLoop {
                        line: (i + 1) as u32,
                        depth: nesting,
                    });
                }
            }

            if trimmed.contains('}') {
                let close_count = trimmed.chars().filter(|c| *c == '}').count() as u32;
                nesting = nesting.saturating_sub(close_count);
            }
        }
        loops
    }

    fn detect_sync_blocking_calls(&self, content: &str) -> Vec<SyncCall> {
        let mut calls = Vec::new();
        let sync_patterns = [
            ".unwrap()", ".expect(", "std::sync::", "Mutex::lock",
            "thread::sleep", "std::thread::sleep", "sleep(",
            ".recv()", ".blocking_", "block_on",
            "wait()", "join()", "result()",
        ];

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if sync_patterns.iter().any(|p| trimmed.contains(p)) {
                calls.push(SyncCall { line: (i + 1) as u32 });
            }
        }
        calls
    }

    fn generate_recommendations(&self, findings: &[Finding]) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        let io_count = findings.iter().filter(|f| f.id == "PERF-IO-HEAVY").count();
        let loop_count = findings.iter().filter(|f| f.id == "PERF-NESTED-LOOP").count();
        let large_count = findings.iter().filter(|f| f.id == "PERF-LARGE-FUNC").count();

        if io_count > 0 {
            recs.push(Recommendation::new(
                &format!("Found {} I/O-heavy functions. Add caching and batch operations.", io_count),
                0.8,
            ));
        }
        if loop_count > 0 {
            recs.push(Recommendation::new(
                &format!("Found {} nested loops. Restructure to reduce algorithmic complexity.", loop_count),
                0.9,
            ));
        }
        if large_count > 0 {
            recs.push(Recommendation::new(
                &format!("Found {} large functions. Split into smaller units.", large_count),
                0.6,
            ));
        }
        recs
    }
}

impl Default for PerformanceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisEngine for PerformanceEngine {
    fn name(&self) -> &'static str {
        "performance"
    }

    fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
        if !self.config.enabled {
            return Ok(AnalysisResult::new(&request.request_id, &request.commit_hash));
        }

        let start = std::time::Instant::now();
        let mut all_findings = Vec::new();

        const SUPPORTED: [&str; 10] = ["rs", "py", "js", "ts", "java", "kt", "go", "mjs", "mts", "c"];
        let files: Vec<_> = walkdir::WalkDir::new(&request.repo_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|ext| SUPPORTED.contains(&ext))
                    .unwrap_or(false)
            })
            .filter(|e| !e.path().to_string_lossy().contains("target/")
                && !e.path().to_string_lossy().contains("node_modules/")
                && !e.path().to_string_lossy().contains(".git/"))
            .map(|e| e.path().to_string_lossy().into_owned())
            .collect();

        for file_path in &files {
            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let findings = self.analyze_file(&content, file_path);
            all_findings.extend(findings);
        }

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        let recommendations = self.generate_recommendations(&all_findings);

        let overall_risk = if all_findings.iter().any(|f| f.severity == Severity::Error) {
            0.7
        } else if all_findings.iter().any(|f| f.severity == Severity::Warning) {
            0.4
        } else {
            0.0
        };

        Ok(AnalysisResult {
            request_id: request.request_id.clone(),
            commit_hash: request.commit_hash.clone(),
            overall_risk,
            findings: all_findings,
            recommendations,
            metrics: None,
            processing_time_ms: elapsed,
            blocked_merge: false,
            jit_features: None,
        })
    }
}

struct FunctionProfile {
    name: String,
    line: u32,
    body_lines: u32,
}

struct IoCall {
    line: u32,
}

struct AllocationSite {
    line: u32,
    count: u32,
}

struct NestedLoop {
    line: u32,
    depth: u32,
}

struct SyncCall {
    line: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_name() {
        let engine = PerformanceEngine::new();
        assert_eq!(engine.name(), "performance");
    }

    #[test]
    fn test_detect_functions() {
        let engine = PerformanceEngine::new();
        let content = "fn a() {}\nfn b() {}\nfn c() {}\n";
        let functions = engine.detect_functions(content);
        assert_eq!(functions.len(), 3);
    }

    #[test]
    fn test_detect_nested_loops() {
        let engine = PerformanceEngine::new();
        let content = "fn test() {\n    for i in 0..10 {\n        for j in 0..10 {\n            println!(\"{}\", i);\n        }\n    }\n}\n";
        let loops = engine.detect_nested_loops(content);
        assert!(!loops.is_empty());
        assert!(loops.iter().any(|l| l.depth >= 2));
    }

    #[test]
    fn test_detect_io_patterns() {
        let engine = PerformanceEngine::new();
        let content = "fn read() -> String {\n    fs::read_to_string(\"file.txt\")\n}\n";
        let io = engine.detect_io_patterns(content);
        assert!(!io.is_empty());
    }

    #[test]
    fn test_engine_disabled() {
        let engine = PerformanceEngine::with_config(PerformanceEngine::new(), PerformanceConfig {
            enabled: false,
            ..PerformanceConfig::default()
        });
        let req = AnalyzeRequest::new("/nonexistent", "abc");
        let result = engine.analyze(&req).unwrap();
        assert!(result.findings.is_empty());
    }

    #[test]
    fn test_engine_default() {
        let engine = PerformanceEngine::default();
        assert_eq!(engine.name(), "performance");
        assert!(engine.config.enabled);
    }
}
