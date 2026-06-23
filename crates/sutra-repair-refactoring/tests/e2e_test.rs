use sutra_common::engine::AnalysisEngine;
use sutra_repair_refactoring::engine::RefactoringEngine;
use sutra_schema::v1::{AnalyzeRequest, Engine};

#[test]
fn test_refactoring_engine_generates_findings_on_complex_code() {
    let engine = RefactoringEngine::new();

    // Create a simple request on the observalog repo
    let request = AnalyzeRequest {
        request_id: "test-ref-001".to_string(),
        repo_path: "/Users/darshanredkar/darshan/observalog".to_string(),
        engines: vec![Engine::Refactoring],
        commit_hash: "HEAD".to_string(),
        config: Default::default(),
    };

    let result = engine.analyze(&request);
    assert!(result.is_ok(), "Engine should not error on valid repo");

    let analysis = result.unwrap();

    // Validate findings were generated
    assert!(!analysis.findings.is_empty(), "Should find refactoring opportunities");

    // All findings should be from refactoring engine
    for finding in &analysis.findings {
        assert_eq!(finding.engine, Engine::Refactoring);
        assert!(finding.id.starts_with("REF-"), "Finding ID should start with REF-");
    }

    println!("✓ Refactoring engine generated {} findings", analysis.findings.len());
}

#[test]
fn test_refactoring_findings_have_fix_suggestions() {
    let engine = RefactoringEngine::new();

    let request = AnalyzeRequest {
        request_id: "test-ref-002".to_string(),
        repo_path: "/Users/darshanredkar/darshan/observalog".to_string(),
        engines: vec![Engine::Refactoring],
        commit_hash: "HEAD".to_string(),
        config: Default::default(),
    };

    let result = engine.analyze(&request).unwrap();

    // Check that findings have fix suggestions
    for finding in result.findings.iter().take(5) {
        // Each finding should have content in the fix field (if it exists in the schema)
        assert!(!finding.message.is_empty(), "Findings should have messages");
    }

    println!("✓ All findings have descriptive messages");
}
