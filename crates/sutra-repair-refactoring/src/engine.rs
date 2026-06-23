use sutra_common::engine::AnalysisEngine;
use sutra_common::error::SutraResult;
use sutra_schema::v1::{
    AnalysisResult, AnalyzeRequest, Engine, Finding, Recommendation, Severity,
};

use crate::types::RefactoringConfig;

pub struct RefactoringEngine {
    config: RefactoringConfig,
}

impl RefactoringEngine {
    pub fn new() -> Self {
        Self {
            config: RefactoringConfig::default(),
        }
    }

    pub fn with_config(mut self, config: RefactoringConfig) -> Self {
        self.config = config;
        self
    }

    fn analyze_file(&self, content: &str, file_path: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let _total_lines = lines.len();

        let class_info = self.detect_classes(content);
        let function_info = self.detect_functions(content);
        let max_cyclomatic = function_info
            .iter()
            .map(|f| f.cyclomatic)
            .max()
            .unwrap_or(0);
        let max_nesting = function_info
            .iter()
            .map(|f| f.nesting_depth)
            .max()
            .unwrap_or(0);
        let coupling_pairs = self.detect_coupling(content, &function_info);
        let duplications = self.detect_duplication(content);

        if max_cyclomatic > self.config.cyclomatic_threshold {
            findings.push(
                Finding::new(
                    "REF-EXTRACT-METHOD",
                    Engine::Refactoring,
                    file_path,
                    function_info
                        .iter()
                        .find(|f| f.cyclomatic == max_cyclomatic)
                        .map(|f| f.line)
                        .unwrap_or(1),
                    &format!(
                        "Function cyclomatic complexity {} exceeds threshold {}. Extract method recommended.",
                        max_cyclomatic, self.config.cyclomatic_threshold
                    ),
                    self.severity_for_cyclomatic(max_cyclomatic),
                )
                .with_fix(&format!(
                    "Extract smaller methods: split function with cyclomatic complexity {} into units under {}",
                    max_cyclomatic, self.config.cyclomatic_threshold
                )),
            );
        }

        for cinfo in &class_info {
            if cinfo.loc > self.config.class_loc_threshold {
                findings.push(
                    Finding::new(
                        "REF-EXTRACT-CLASS",
                        Engine::Refactoring,
                        file_path,
                        cinfo.line,
                        &format!(
                            "Class '{}' is {} LOC (threshold: {}). Extract into smaller classes.",
                            cinfo.name, cinfo.loc, self.config.class_loc_threshold
                        ),
                        self.severity_for_loc(cinfo.loc),
                    )
                    .with_fix(&format!(
                        "Extract class '{}' into {} smaller classes (~{} LOC each)",
                        cinfo.name,
                        (cinfo.loc / self.config.class_loc_threshold).max(2),
                        self.config.class_loc_threshold
                    )),
                );
            }
        }

        for pair in &coupling_pairs {
            if pair.strength > self.config.coupling_threshold {
                findings.push(
                    Finding::new(
                        "REF-COUPLING",
                        Engine::Refactoring,
                        file_path,
                        pair.line,
                        &format!(
                            "Tight coupling ({:.2}) between '{}' and '{}'. Separate concerns.",
                            pair.strength, pair.method_a, pair.method_b
                        ),
                        Severity::Warning,
                    )
                    .with_fix(&format!(
                        "Reduce coupling between '{}' and '{}': introduce interface or mediator pattern",
                        pair.method_a, pair.method_b
                    )),
                );
            }
        }

        if duplications > self.config.duplication_threshold {
            findings.push(
                Finding::new(
                    "REF-DUPLICATION",
                    Engine::Refactoring,
                    file_path,
                    1,
                    &format!(
                        "Duplication ratio {:.2} exceeds threshold {:.2}. Extract duplicated code.",
                        duplications, self.config.duplication_threshold
                    ),
                    Severity::Warning,
                )
                .with_fix("Extract duplicated blocks into shared functions"),
            );
        }

        if max_nesting > self.config.nesting_threshold {
            findings.push(
                Finding::new(
                    "REF-NESTING",
                    Engine::Refactoring,
                    file_path,
                    function_info
                        .iter()
                        .find(|f| f.nesting_depth == max_nesting)
                        .map(|f| f.line)
                        .unwrap_or(1),
                    &format!(
                        "Nesting depth {} exceeds threshold {}. Reduce nesting.",
                        max_nesting, self.config.nesting_threshold
                    ),
                    Severity::Warning,
                )
                .with_fix("Reduce nesting via early returns, guard clauses, or extracting helper functions"),
            );
        }

        findings.truncate(self.config.max_refactors_per_file);
        findings
    }

    fn detect_classes(&self, content: &str) -> Vec<ClassInfo> {
        let mut classes = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let is_class = trimmed.starts_with("class ") || trimmed.starts_with("struct ")
                || trimmed.starts_with("pub struct ") || trimmed.starts_with("pub class ")
                || trimmed.starts_with("pub(crate) struct ");
            if !is_class {
                continue;
            }
            let pieces: Vec<&str> = trimmed.split(|c: char| c == '{' || c == '<')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .filter(|s| *s != "pub" && *s != "struct" && *s != "class" && *s != "pub(crate)")
                .collect();
            let name = pieces.first().map(|s| s.to_string()).unwrap_or_else(|| "unknown".into());

            let mut class_loc = 0;
            let mut brace_depth = 0u32;
            let mut in_class = false;
            for (j, l) in lines.iter().enumerate() {
                if j < i { continue; }
                if j == i {
                    for ch in l.chars() {
                        if ch == '{' { in_class = true; brace_depth += 1; }
                    }
                    continue;
                }
                if !in_class { continue; }
                class_loc += 1;
                for ch in l.chars() {
                    match ch {
                        '{' => brace_depth += 1,
                        '}' => brace_depth -= 1,
                        _ => {}
                    }
                }
                if brace_depth == 0 { break; }
            }
            classes.push(ClassInfo {
                name,
                loc: class_loc,
                line: (i + 1) as u32,
            });
        }
        classes
    }

    fn detect_functions(&self, content: &str) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let fn_prefixes = ["fn ", "def ", "function ", "pub fn ", "pub async fn ", "async fn ",
            "pub(crate) fn ", "pub(crate) async fn ", "pub unsafe fn "];

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
                continue;
            }
            let is_function = fn_prefixes.iter().any(|kw| trimmed.starts_with(kw));

            if !is_function {
                continue;
            }

            let before_paren = trimmed.split('(').next().unwrap_or("");
            let name = before_paren
                .split_whitespace()
                .filter(|s| *s != "fn" && *s != "def" && *s != "function"
                    && *s != "pub" && *s != "async" && *s != "unsafe" && *s != "pub(crate)")
                .last()
                .unwrap_or("anonymous")
                .to_string();

            let mut brace_depth: u32 = 0;
            let mut body_lines = 0;
            let mut cyclomatic = 1;
            let mut nesting_depth = 0;
            let mut current_nesting = 0;
            let mut in_function = false;
            let mut pushed = false;

            for (j, l) in lines.iter().enumerate() {
                if j < i {
                    continue;
                }
                if j == i {
                    for ch in l.chars() {
                        if ch == '{' {
                            in_function = true;
                            brace_depth += 1;
                        } else if ch == '}' && in_function {
                            brace_depth = brace_depth.saturating_sub(1);
                        }
                    }
                    if brace_depth == 0 && in_function {
                        in_function = false;  // ponytail: tracking function scope for coupling detection
                        functions.push(FunctionInfo {
                            name: name.clone(),
                            line: (i + 1) as u32,
                            body_lines,
                            cyclomatic,
                            nesting_depth,
                        });
                        pushed = true;
                        break;
                    }
                    continue;
                }
                if !in_function {
                    continue;
                }

                body_lines += 1;

                for ch in l.chars() {
                    match ch {
                        '{' => {
                            brace_depth += 1;
                            current_nesting += 1;
                            nesting_depth = nesting_depth.max(current_nesting);
                        }
                        '}' => {
                            brace_depth -= 1;
                            current_nesting = current_nesting.max(1) - 1;
                        }
                        _ => {}
                    }
                }

                if brace_depth <= 0 {
                    break;
                }

                let ltrimmed = l.trim();
                for kw in &["if ", "else if ", "for ", "while ", "case ", "&&", "||", "catch "] {
                    if ltrimmed.starts_with(kw) {
                        cyclomatic += 1;
                    }
                }
            }

            if !pushed {
                functions.push(FunctionInfo {
                    name,
                    line: (i + 1) as u32,
                    body_lines,
                    cyclomatic,
                    nesting_depth,
                });
            }
        }
        functions
    }

    fn detect_coupling(&self, content: &str, functions: &[FunctionInfo]) -> Vec<CouplingPair> {
        let mut pairs = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
                continue;
            }

            for func in functions {
                if trimmed.contains(&func.name) && !trimmed.starts_with("fn ")
                    && !trimmed.starts_with("def ") && !trimmed.starts_with("function ")
                {
                    let caller = functions.iter().find(|f| {
                        (i + 1) >= f.line as usize && (i + 1) < (f.line + f.body_lines) as usize
                    });

                    if let Some(caller) = caller {
                        if caller.name != func.name {
                            pairs.push(CouplingPair {
                                method_a: caller.name.clone(),
                                method_b: func.name.clone(),
                                strength: 0.75,
                                line: (i + 1) as u32,
                            });
                        }
                    }
                }
            }
        }

        pairs.dedup_by(|a, b| a.method_a == b.method_a && a.method_b == b.method_b);
        pairs
    }

    fn detect_duplication(&self, content: &str) -> f64 {
        let lines: Vec<&str> = content.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if lines.len() < 4 {
            return 0.0;
        }

        let mut duplicate_lines = 0;

        for i in 0..lines.len() {
            let line_len = lines[i].len();
            if line_len < 5 {
                continue;
            }
            for j in (i + 1)..lines.len() {
                if lines[i] == lines[j] {
                    duplicate_lines += 1;
                    break;
                }
            }
        }

        duplicate_lines as f64 / lines.len() as f64
    }

    fn severity_for_cyclomatic(&self, cc: u32) -> Severity {
        if cc > self.config.cyclomatic_threshold * 2 {
            Severity::Error
        } else {
            Severity::Warning
        }
    }

    fn severity_for_loc(&self, loc: u32) -> Severity {
        if loc > self.config.class_loc_threshold * 2 {
            Severity::Error
        } else {
            Severity::Warning
        }
    }

    fn generate_recommendations(&self, findings: &[Finding]) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        let extract_count = findings.iter().filter(|f| f.id.starts_with("REF-EXTRACT")).count();
        let coupling_count = findings.iter().filter(|f| f.id == "REF-COUPLING").count();

        if extract_count > 0 {
            recs.push(Recommendation::new(
                &format!("Found {} extract opportunities. Prioritize refactoring classes with highest LOC first.", extract_count),
                0.8,
            ));
        }
        if coupling_count > 0 {
            recs.push(Recommendation::new(
                &format!(
                    "Found {} tight coupling pairs. Use interface-based design to reduce coupling.",
                    coupling_count
                ),
                0.6,
            ));
        }
        recs
    }
}

impl Default for RefactoringEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisEngine for RefactoringEngine {
    fn name(&self) -> &'static str {
        "refactoring"
    }

    fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
        if !self.config.enabled {
            return Ok(AnalysisResult::new(&request.request_id, &request.commit_hash));
        }

        let start = std::time::Instant::now();
        let mut all_findings = Vec::new();
        const SUPPORTED: [&str; 10] = ["rs", "py", "js", "ts", "java", "kt", "kts", "go", "mjs", "mts"];
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
            0.6
        } else if all_findings.iter().any(|f| f.severity == Severity::Warning) {
            0.3
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

struct ClassInfo {
    name: String,
    loc: u32,
    line: u32,
}

struct FunctionInfo {
    name: String,
    line: u32,
    body_lines: u32,
    cyclomatic: u32,
    nesting_depth: u32,
}

struct CouplingPair {
    method_a: String,
    method_b: String,
    strength: f64,
    line: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_name() {
        let engine = RefactoringEngine::new();
        assert_eq!(engine.name(), "refactoring");
    }

    #[test]
    fn test_detect_functions_rust() {
        let engine = RefactoringEngine::new();
        let content = r#"
fn foo(x: i32) -> i32 {
    let y = x + 1;
    y
}

fn bar() {
    if true {
        for i in 0..10 {
            println!("{}", i);
        }
    }
}
"#;
        let functions = engine.detect_functions(content);
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "foo");
        assert_eq!(functions[1].name, "bar");
        assert!(functions[1].cyclomatic > 1);
    }

    #[test]
    fn test_detect_classes_rust() {
        let engine = RefactoringEngine::new();
        let content = r#"
pub struct Config {
    pub enabled: bool,
}

pub struct Triage {
    pub field1: i32,
    pub field2: String,
}
"#;
        let classes = engine.detect_classes(content);
        assert_eq!(classes.len(), 2);
        assert_eq!(classes[0].name, "Config");
        assert_eq!(classes[1].name, "Triage");
    }

    #[test]
    fn test_detect_functions_python() {
        let engine = RefactoringEngine::new();
        let content = r#"
def foo(x):
    y = x + 1
    return y

def bar():
    if True:
        for i in range(10):
            print(i)
"#;
        let functions = engine.detect_functions(content);
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "foo");
        assert_eq!(functions[1].name, "bar");
    }

    #[test]
    fn test_analyze_file_no_issues() {
        let engine = RefactoringEngine::new();
        let content = r#"
fn simple() -> i32 {
    42
}
"#;
        let findings = engine.analyze_file(content, "test.rs");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_analyze_file_high_cyclomatic() {
        let engine = RefactoringEngine::with_config(RefactoringEngine::new(), RefactoringConfig {
            cyclomatic_threshold: 3,
            ..RefactoringConfig::default()
        });
        let content = r#"
fn complex(x: i32) {
    if x > 0 {
        println!("positive");
    }
    if x > 10 {
        println!("big");
    }
    if x > 100 {
        println!("huge");
    }
    if x < 0 {
        println!("negative");
    }
}
"#;
        let findings = engine.analyze_file(content, "test.rs");
        assert!(findings.iter().any(|f| f.id == "REF-EXTRACT-METHOD"));
    }

    #[test]
    fn test_analyze_file_large_class() {
        let engine = RefactoringEngine::with_config(RefactoringEngine::new(), RefactoringConfig {
            class_loc_threshold: 5,
            ..RefactoringConfig::default()
        });
        let content = "pub struct Large {\n".to_string()
            + &"    field: i32,\n".repeat(10)
            + "}";

        let findings = engine.analyze_file(&content, "test.rs");
        assert!(findings.iter().any(|f| f.id == "REF-EXTRACT-CLASS"));
    }

    #[test]
    fn test_analyze_file_high_nesting() {
        let engine = RefactoringEngine::with_config(RefactoringEngine::new(), RefactoringConfig {
            nesting_threshold: 2,
            ..RefactoringConfig::default()
        });
        let content = r#"
fn deeply_nested() {
    if true {
        for i in 0..10 {
            while false {
                println!("deep");
            }
        }
    }
}
"#;
        let findings = engine.analyze_file(content, "test.rs");
        assert!(findings.iter().any(|f| f.id == "REF-NESTING"));
    }

    #[test]
    fn test_engine_disabled() {
        let engine = RefactoringEngine::with_config(RefactoringEngine::new(), RefactoringConfig {
            enabled: false,
            ..RefactoringConfig::default()
        });
        let req = AnalyzeRequest::new("/nonexistent", "abc");
        let result = engine.analyze(&req).unwrap();
        assert!(result.findings.is_empty());
    }

    #[test]
    fn test_engine_default() {
        let engine = RefactoringEngine::default();
        assert_eq!(engine.name(), "refactoring");
        assert!(engine.config.enabled);
    }

    #[test]
    fn test_detect_duplication() {
        let engine = RefactoringEngine::new();
        let content = "    let x = 42;\n    let y = x + 1;\n    let z = y * 2;\n    let x = 42;\n    let y = x + 1;\n    let z = y * 2;\n";
        let ratio = engine.detect_duplication(content);
        assert!(ratio > 0.0, "expected > 0, got {}", ratio);
    }

    #[test]
    fn test_generate_recommendations() {
        let engine = RefactoringEngine::new();
        let findings = vec![
            Finding::new("REF-EXTRACT-CLASS", Engine::Refactoring, "f.rs", 1, "test", Severity::Warning),
        ];
        let recs = engine.generate_recommendations(&findings);
        assert!(!recs.is_empty());
    }
}
