use serde::{Deserialize, Serialize};

// ── Input types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityProfile {
    pub time_complexity: String,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub nesting_depth: u32,
    pub function_count: u32,
    pub lines_of_code: u32,
    pub class_count: u32,
    pub method_coupling: Vec<MethodCoupling>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodCoupling {
    pub method_a: String,
    pub method_b: String,
    pub shared_state: Vec<String>,
    pub coupling_strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetrics {
    pub duplication_lines: u32,
    pub duplication_ratio: f64,
    pub avg_function_length: u32,
    pub max_function_length: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestInfo {
    pub test_coverage: f64,
    pub untested_paths: u32,
}

// ── Output types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringSpec {
    pub engine: String,
    pub source_file: String,
    pub refactors: Vec<Refactoring>,
    pub summary: RefactoringSummary,
    pub processing: ProcessingInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refactoring {
    pub id: String,
    pub r#type: String,
    pub severity: String,
    pub description: String,
    pub current_state: CurrentState,
    pub proposed_state: ProposedState,
    pub impact: Impact,
    pub effort: Effort,
    pub dependencies: Dependencies,
    pub validation: Validation,
    pub roi: Roi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentState {
    pub structure: String,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub lines_of_code: u32,
    pub methods: u32,
    pub shared_fields: u32,
    pub cohesion_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedState {
    pub structure: String,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub lines_of_code_per_class: Vec<u32>,
    pub methods_per_class: Vec<u32>,
    pub shared_fields_per_class: Vec<u32>,
    pub cohesion_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impact {
    pub complexity_reduction: ComplexityReduction,
    pub maintainability: Maintainability,
    pub testability: Testability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityReduction {
    pub metric: String,
    pub before: u32,
    pub after: u32,
    pub reduction_percent: f64,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Maintainability {
    pub before_score: u32,
    pub after_score: u32,
    pub improvement_percent: f64,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Testability {
    pub before_untested_paths: u32,
    pub after_untested_paths: u32,
    pub path_reduction_percent: f64,
    pub test_complexity_before: String,
    pub test_complexity_after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effort {
    pub estimated_hours: u32,
    pub effort_breakdown: EffortBreakdown,
    pub complexity_of_refactor: String,
    pub risk_of_bugs: f64,
    pub reversibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffortBreakdown {
    pub design_time: u32,
    pub extraction_time: u32,
    pub testing_time: u32,
    pub buffer: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependencies {
    pub files_affected: Vec<String>,
    pub api_changes: Vec<ApiChange>,
    pub dependency_additions: Vec<String>,
    pub dependency_removals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiChange {
    pub old_signature: String,
    pub new_signature: String,
    pub breaking: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validation {
    pub confidence: f64,
    pub confidence_reasoning: String,
    pub edge_cases: Vec<String>,
    pub validation_strategy: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Roi {
    pub incident_prevention: IncidentPrevention,
    pub total_value_per_year: String,
    pub effort_cost: String,
    pub roi_months: f64,
    pub priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentPrevention {
    pub current_maintenance_bugs_per_year: u32,
    pub predicted_bugs_after_refactor: u32,
    pub bug_prevention_value: String,
    pub onboarding_time_reduction_hours: u32,
    pub onboarding_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringSummary {
    pub total_refactors_found: u32,
    pub total_complexity_reduction: String,
    pub total_effort_hours: u32,
    pub combined_roi_months: f64,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingInfo {
    pub algorithm_version: String,
    pub execution_time_ms: u64,
    pub files_analyzed: u32,
    pub confidence_level: String,
}

// ── Refactoring type enum ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefactoringType {
    ExtractClass,
    ExtractMethod,
    MergeClasses,
    ReduceNesting,
}

impl RefactoringType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RefactoringType::ExtractClass => "extract_class",
            RefactoringType::ExtractMethod => "extract_method",
            RefactoringType::MergeClasses => "merge_classes",
            RefactoringType::ReduceNesting => "reduce_nesting",
        }
    }
}

// ── Config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RefactoringConfig {
    pub enabled: bool,
    pub max_refactors_per_file: usize,
    pub cyclomatic_threshold: u32,
    pub class_loc_threshold: u32,
    pub coupling_threshold: f64,
    pub duplication_threshold: f64,
    pub nesting_threshold: u32,
}

impl Default for RefactoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_refactors_per_file: 3,
            cyclomatic_threshold: 15,
            class_loc_threshold: 300,
            coupling_threshold: 0.7,
            duplication_threshold: 0.2,
            nesting_threshold: 5,
        }
    }
}
