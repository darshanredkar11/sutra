use sutra_common::engine::AnalysisEngine;
use sutra_repair_refactoring::engine::RefactoringEngine;
use sutra_schema::v1::{AnalyzeRequest, Engine};

#[test]
fn test_refactoring_findings_include_spec_data() {
    let engine = RefactoringEngine::new();

    let request = AnalyzeRequest {
        request_id: "test-spec-001".to_string(),
        repo_path: "/Users/darshanredkar/darshan/observalog".to_string(),
        engines: vec![Engine::Refactoring],
        commit_hash: "HEAD".to_string(),
        config: Default::default(),
    };

    let result = engine.analyze(&request);
    assert!(result.is_ok(), "Engine should not error on valid repo");

    let analysis = result.unwrap();
    assert!(!analysis.findings.is_empty(), "Should find refactoring opportunities");

    // Validate that findings have spec_data populated
    let mut findings_with_spec = 0;
    let mut findings_with_confidence = 0;
    let mut findings_with_edge_cases = 0;

    for finding in &analysis.findings {
        println!("Finding: {} - {}", finding.id, finding.message);

        if finding.spec_data.is_some() {
            findings_with_spec += 1;
            let spec = finding.spec_data.as_ref().unwrap();
            println!("  ✓ spec_data: {:?}", spec);

            // Validate spec structure
            if let Some(obj) = spec.as_object() {
                assert!(obj.contains_key("type"), "Spec should have 'type' field");
                assert!(obj.contains_key("impact"), "Spec should have 'impact' field");
                assert!(obj.contains_key("effort"), "Spec should have 'effort' field");
                assert!(obj.contains_key("roi"), "Spec should have 'roi' field");
            }
        }

        if finding.confidence.is_some() {
            findings_with_confidence += 1;
            let conf = finding.confidence.unwrap();
            println!("  ✓ confidence: {:.2}", conf);
            assert!(conf > 0.0 && conf <= 1.0, "Confidence should be in range [0.0, 1.0]");
        }

        if finding.edge_cases.is_some() {
            findings_with_edge_cases += 1;
            let cases = finding.edge_cases.as_ref().unwrap();
            println!("  ✓ edge_cases: {} items", cases.len());
            assert!(!cases.is_empty(), "Edge cases should not be empty");
        }
    }

    println!("\n✓ Findings with spec_data: {}/{}", findings_with_spec, analysis.findings.len());
    println!("✓ Findings with confidence: {}/{}", findings_with_confidence, analysis.findings.len());
    println!("✓ Findings with edge_cases: {}/{}", findings_with_edge_cases, analysis.findings.len());

    // At least 50% of findings should have spec_data
    assert!(findings_with_spec as f64 >= (analysis.findings.len() as f64 * 0.5),
        "At least 50% of findings should have spec_data populated");

    // At least 50% of findings should have confidence
    assert!(findings_with_confidence as f64 >= (analysis.findings.len() as f64 * 0.5),
        "At least 50% of findings should have confidence populated");
}
