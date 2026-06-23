use serde::{Deserialize, Serialize};

// ── Input types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageData {
    pub line_coverage: f64,
    pub branch_coverage: f64,
    pub function_coverage: f64,
    pub untested_lines: u32,
    pub untested_branches: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalUntestedPath {
    pub function: String,
    pub untested_branches: u32,
    pub branch_description: Vec<String>,
}

// ── Output types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestingGapSpec {
    pub engine: String,
    pub coverage_metrics: CoverageMetrics,
    pub test_gaps: Vec<TestGap>,
    pub summary: TestingGapSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageMetrics {
    pub line_coverage: f64,
    pub branch_coverage: f64,
    pub untested_lines: u32,
    pub coverage_goal: f64,
    pub gap_to_goal: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGap {
    pub id: String,
    pub function: String,
    pub coverage_before_percent: u32,
    pub coverage_after_percent: u32,
    pub coverage_improvement_percent: u32,
    pub untested_branches: Vec<String>,
    pub test_patterns: Vec<TestPattern>,
    pub roi: TestGapRoi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPattern {
    pub pattern_id: String,
    pub pattern: String,
    pub test_cases: Vec<TestCase>,
    pub test_scope: Option<String>,
    pub effort_hours: u32,
    pub test_code_lines: Option<u32>,
    pub test_cases_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub input: String,
    pub expected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGapRoi {
    pub effort_hours: u32,
    pub bug_prevention: String,
    pub roi_value: String,
    pub roi_months: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestingGapSummary {
    pub total_gap: f64,
    pub total_effort_hours: u32,
    pub coverage_target_achievable: bool,
    pub priority: String,
}

// ── Config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TestingGapConfig {
    pub enabled: bool,
    pub coverage_goal: f64,
}

impl Default for TestingGapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            coverage_goal: 0.85,
        }
    }
}
