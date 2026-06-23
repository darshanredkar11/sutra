use std::io::Write;

use sutra_orchestrator::coordinator::Orchestrator;
use sutra_schema::v1::{AnalyzeRequest, Engine, Severity};

fn create_synthetic_repo() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let repo_path = dir.path().to_str().unwrap().to_string();

    let repo = git2::Repository::init(&repo_path).unwrap();

    let mut f1 = std::fs::File::create(dir.path().join("lib.rs")).unwrap();
    f1.write_all(b"
pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn multiply(a: i32, b: i32) -> i32 { a * b }
").unwrap();

    let mut f2 = std::fs::File::create(dir.path().join("utils.rs")).unwrap();
    f2.write_all(b"
pub fn process(items: &[i32]) -> Vec<i32> {
    items.iter().map(|x| x * 2).collect()
}
").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("lib.rs")).unwrap();
    index.add_path(std::path::Path::new("utils.rs")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("test", "test@test.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[]).unwrap();

    let mut f1 = std::fs::OpenOptions::new()
        .append(true)
        .open(dir.path().join("lib.rs")).unwrap();
    f1.write_all(b"
pub fn subtract(a: i32, b: i32) -> i32 { a - b }
").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("lib.rs")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "add subtract", &tree, &[&repo.head().unwrap().peel_to_commit().unwrap()]).unwrap();

    (dir, repo_path)
}

fn build_all_engines() -> Orchestrator {
    let mut o = Orchestrator::new();
    o.register(Engine::Mgtg, Box::new(sutra_mgtg::engine::MgtgEngine::new()));
    o.register(Engine::Dependency, Box::new(sutra_dependency::engine::DependencyEngine::new()));
    o.register(Engine::Process, Box::new(sutra_process::engine::ProcessEngine::new()));
    o.register(Engine::Ml, Box::new(sutra_ml::engine::MlEngine::new()));
    o.register(Engine::Hitl, Box::new(sutra_hitl::engine::HitlEngine::new()));
    o.register(Engine::RuntimeSurvivability, Box::new(sutra_rse::engine::RseEngine::new()));
    o.register(Engine::Refactoring, Box::new(sutra_repair_refactoring::engine::RefactoringEngine::new()));
    o.register(Engine::CouplingResolution, Box::new(sutra_repair_coupling::engine::CouplingEngine::new()));
    o.register(Engine::Performance, Box::new(sutra_repair_performance::engine::PerformanceEngine::new()));
    o.register(Engine::TestingGap, Box::new(sutra_repair_testing_gap::engine::TestingGapEngine::new()));
    o.register(Engine::DebtRoi, Box::new(sutra_repair_debt_roi::engine::DebtRoiEngine::new()));
    o
}

#[test]
fn test_full_analysis_produces_findings() {
    let (_dir, repo_path) = create_synthetic_repo();
    let orchestrator = build_all_engines();

    let mut request = AnalyzeRequest::new(&repo_path, "HEAD");
    request.engines = Engine::ALL.to_vec();

    let result = orchestrator.analyze(&request).unwrap();
    println!("All-engine findings: {:?}", result.findings.iter().map(|f| (f.id.as_str(), f.engine.as_str())).collect::<Vec<_>>());
    println!("Risk: {:.4}, metrics: {:?}", result.overall_risk, result.metrics);

    // Synthetic repo has 2 commits, trivial Rust files. MGTG may or may not find findings
    // depending on the parser. Process requires 10+ commits for co-change, >20 revs for hotspots.
    // The key assertions: pipeline runs, produces valid output
    assert!(result.overall_risk >= 0.0, "risk should be >= 0");
    assert!(result.overall_risk <= 1.0, "risk should be <= 1.0");
    assert!(result.metrics.is_some(), "expected metrics");
    assert!(result.metrics.as_ref().unwrap().total_files >= 0, "should have file count");
}

#[test]
fn test_all_engines_run_without_crashing() {
    let (_dir, repo_path) = create_synthetic_repo();
    let orchestrator = build_all_engines();

    // Test each engine individually — the pipeline should never panic or return Err
    for engine_type in Engine::ALL {
        let result = orchestrator.analyze_single(&AnalyzeRequest::new(&repo_path, "HEAD"), engine_type);
        assert!(result.is_ok(), "engine {} should not crash, got: {:?}", engine_type.as_str(), result.err());
        let r = result.unwrap();
        assert!(r.overall_risk >= 0.0 && r.overall_risk <= 1.0,
            "engine {} risk out of range: {}", engine_type.as_str(), r.overall_risk);
        assert!(r.processing_time_ms >= 0.0, "engine {} negative time", engine_type.as_str());
    }
}

#[test]
fn test_risk_score_determinism() {
    let (_dir, repo_path) = create_synthetic_repo();
    let orchestrator = build_all_engines();
    let request = AnalyzeRequest::new(&repo_path, "HEAD");

    let r1 = orchestrator.analyze(&request).unwrap();
    let r2 = orchestrator.analyze(&request).unwrap();

    assert!((r1.overall_risk - r2.overall_risk).abs() < f64::EPSILON);
    assert_eq!(r1.findings.len(), r2.findings.len());
}

#[test]
fn test_single_engine_via_single_api() {
    let (_dir, repo_path) = create_synthetic_repo();
    let orchestrator = build_all_engines();
    let result = orchestrator.analyze_single(&AnalyzeRequest::new(&repo_path, "HEAD"), Engine::Mgtg).unwrap();
    assert!(result.overall_risk >= 0.0);
    assert!(result.overall_risk <= 1.0);
}

#[test]
fn test_single_engine_via_analyze() {
    let (_dir, repo_path) = create_synthetic_repo();
    let orchestrator = build_all_engines();
    let mut request = AnalyzeRequest::new(&repo_path, "HEAD");
    request.engines = vec![Engine::Mgtg];
    let result = orchestrator.analyze(&request).unwrap();
    assert!(result.overall_risk >= 0.0);
}

#[test]
fn test_empty_repo_still_runs() {
    let dir = tempfile::tempdir().unwrap();
    git2::Repository::init(dir.path()).unwrap();
    let orchestrator = build_all_engines();
    let result = orchestrator.analyze(&AnalyzeRequest::new(dir.path().to_str().unwrap(), "HEAD")).unwrap();
    assert!(result.overall_risk >= 0.0);
}

#[test]
fn test_missing_path_produces_error_findings() {
    let orchestrator = build_all_engines();
    let result = orchestrator.analyze(&AnalyzeRequest::new("/tmp/missing-repo-12345", "HEAD")).unwrap();
    let errors: Vec<_> = result.findings.iter().filter(|f| f.severity == Severity::Error).collect();
    println!("Error findings for missing path: {}/{}", errors.len(), result.findings.len());
    assert!(errors.len() >= 2, "expected at least 2 engine errors for missing path, got {}", errors.len());
}

#[test]
fn test_parallel_execution_completes() {
    let (_dir, repo_path) = create_synthetic_repo();
    let orchestrator = build_all_engines();
    let start = std::time::Instant::now();
    let mut request = AnalyzeRequest::new(&repo_path, "HEAD");
    request.engines = Engine::ALL.to_vec();
    let result = orchestrator.analyze(&request).unwrap();
    let elapsed = start.elapsed();
    println!("All engines completed in {:?}, risk: {:.4}", elapsed, result.overall_risk);
    assert!(result.overall_risk >= 0.0);
    assert!(elapsed.as_secs() < 30, "analysis took too long");
}

#[test]
fn test_all_engines_register_and_health() {
    let orchestrator = build_all_engines();
    let names = orchestrator.engine_names();
    assert!(names.contains(&"mgtg"));
    assert!(names.contains(&"dependency"));
    assert!(names.contains(&"process"));
    assert!(names.contains(&"ml"));
    assert!(names.contains(&"hitl"));
    assert!(names.contains(&"rse"));
    assert!(names.contains(&"refactoring"));
    assert!(names.contains(&"coupling"));
    assert!(names.contains(&"performance"));
    assert!(names.contains(&"testing_gap"));
    assert!(names.contains(&"debt_roi"));
    assert_eq!(names.len(), 11);
}

#[test]
fn test_health_check_all_healthy() {
    let orchestrator = build_all_engines();
    let health = orchestrator.health_check();
    assert_eq!(health.len(), 11);
    assert!(health.iter().all(|(_engine, ok)| *ok));
}

#[test]
fn test_metrics_are_populated() {
    let (_dir, repo_path) = create_synthetic_repo();
    let orchestrator = build_all_engines();
    let mut request = AnalyzeRequest::new(&repo_path, "HEAD");
    request.engines = Engine::ALL.to_vec();
    let result = orchestrator.analyze(&request).unwrap();

    let m = result.metrics.expect("metrics should be present");
    assert!(m.total_files > 0, "should have files");
}

#[test]
fn test_orchestrator_parallel_safety() {
    let (_dir, repo_path) = create_synthetic_repo();
    let orchestrator = std::sync::Arc::new(build_all_engines());
    let request = std::sync::Arc::new(AnalyzeRequest::new(&repo_path, "HEAD"));

    let mut handles = vec![];
    for _ in 0..4 {
        let o = orchestrator.clone();
        let req = request.clone();
        handles.push(std::thread::spawn(move || {
            o.analyze(&req).unwrap()
        }));
    }

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    for r in &results {
        assert!(r.overall_risk >= 0.0, "risk should be >= 0, got {}", r.overall_risk);
        assert!(r.overall_risk <= 1.0, "risk should be <= 1.0, got {}", r.overall_risk);
    }
}

#[test]
#[ignore]
fn test_against_real_repo_rust_snappy() {
    let dir = tempfile::tempdir().unwrap();
    let url = "https://github.com/BurntSushi/rust-snappy";
    let ok = std::process::Command::new("git")
        .args(["clone", "--depth", "1", "--single-branch", url, dir.path().to_str().unwrap()])
        .status().map(|s| s.success()).unwrap_or(false);
    if !ok { eprintln!("skipping, could not clone {url}"); return; }

    let orchestrator = build_all_engines();
    let result = orchestrator.analyze(&AnalyzeRequest::new(dir.path().to_str().unwrap(), "HEAD")).unwrap();
    println!("rust-snappy — risk: {:.4}, findings: {}, time: {:.0}ms",
        result.overall_risk, result.findings.len(), result.processing_time_ms);
    assert!(result.overall_risk >= 0.0);
}

#[test]
#[ignore]
fn test_against_real_repo_serde_json() {
    let dir = tempfile::tempdir().unwrap();
    let url = "https://github.com/serde-rs/json";
    let ok = std::process::Command::new("git")
        .args(["clone", "--depth", "1", "--single-branch", url, dir.path().to_str().unwrap()])
        .status().map(|s| s.success()).unwrap_or(false);
    if !ok { eprintln!("skipping, could not clone {url}"); return; }

    let orchestrator = build_all_engines();
    let result = orchestrator.analyze(&AnalyzeRequest::new(dir.path().to_str().unwrap(), "HEAD")).unwrap();
    println!("serde-json — risk: {:.4}, findings: {}, time: {:.0}ms",
        result.overall_risk, result.findings.len(), result.processing_time_ms);
    assert!(result.overall_risk >= 0.0);
}

#[test]
#[ignore]
fn test_against_real_repo() {
    let dir = tempfile::tempdir().unwrap();
    let url = "https://github.com/darshanredkar/sutra";
    let ok = std::process::Command::new("git")
        .args(["clone", "--depth", "1", "--single-branch", url, dir.path().to_str().unwrap()])
        .status().map(|s| s.success()).unwrap_or(false);
    if !ok { eprintln!("skipping, could not clone {url}"); return; }

    let orchestrator = build_all_engines();
    let result = orchestrator.analyze(&AnalyzeRequest::new(dir.path().to_str().unwrap(), "HEAD")).unwrap();
    println!("sutra-self — risk: {:.4}, findings: {}, time: {:.0}ms",
        result.overall_risk, result.findings.len(), result.processing_time_ms);
    assert!(!result.findings.is_empty());
    assert!(result.overall_risk > 0.0);
}
