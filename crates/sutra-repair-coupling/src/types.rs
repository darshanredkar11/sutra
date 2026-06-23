use serde::{Deserialize, Serialize};

// ── Input types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoChangePair {
    pub file_a: String,
    pub file_b: String,
    pub co_change_count: u32,
    pub co_change_ratio: f64,
    pub shared_functions: Vec<String>,
    pub call_graph_distance: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub nodes: Vec<DepNode>,
    pub edges: Vec<DepEdge>,
    pub cycles: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepNode {
    pub id: String,
    pub module: String,
    pub lines: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepEdge {
    pub source: String,
    pub target: String,
    pub call_count: u32,
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeMetrics {
    pub latency_p50_ms: f64,
    pub latency_p95_ms: f64,
    pub latency_p99_ms: f64,
    pub throughput_rps: f64,
    pub bottleneck: String,
}

// ── Output types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingSpec {
    pub engine: String,
    pub analysis_scope: String,
    pub couplings_found: Vec<CouplingFinding>,
    pub summary: CouplingSummary,
    pub processing: ProcessingInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingFinding {
    pub id: String,
    pub r#type: String,
    pub severity: String,
    pub description: String,
    pub current_coupling: CurrentCoupling,
    pub proposed_architecture: ProposedArchitecture,
    pub impact: CouplingImpact,
    pub effort: CouplingEffort,
    pub dependencies: CouplingDeps,
    pub validation: CouplingValidation,
    pub roi: CouplingRoi,
    pub migration_plan: MigrationPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentCoupling {
    pub modules_involved: Vec<String>,
    pub coupling_metric: f64,
    pub coupling_type: String,
    pub call_sequence: Vec<String>,
    pub bottleneck: Option<BottleneckInfo>,
    pub co_change_evidence: Vec<CoChangeEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleneckInfo {
    pub stage: String,
    pub latency_ms: u32,
    pub reason: String,
    pub dependency_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoChangeEvidence {
    pub files: Vec<String>,
    pub cochanges: u32,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedArchitecture {
    pub strategy: String,
    pub description: String,
    pub before_architecture: ArchSnapshot,
    pub after_architecture: ArchSnapshot,
    pub new_components: Vec<NewComponent>,
    pub dependency_changes: DepChanges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchSnapshot {
    pub pattern: String,
    pub diagram: String,
    pub workers: u32,
    pub queues: u32,
    pub critical_path_ms: u32,
    pub parallelization: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewComponent {
    pub name: String,
    pub r#type: String,
    pub purpose: String,
    pub config: serde_json::Value,
    pub replicas: Option<u32>,
    pub responsibilities: Option<Vec<String>>,
    pub resource_profile: Option<String>,
    pub batching: Option<BatchingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchingConfig {
    pub window_ms: u32,
    pub max_batch_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepChanges {
    pub removed_direct_calls: Vec<String>,
    pub added_indirect_calls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingImpact {
    pub throughput: ThroughputImpact,
    pub latency: LatencyImpact,
    pub resource_cost: ResourceCost,
    pub coupling: CouplingMetric,
    pub maintainability: Maintainability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputImpact {
    pub before_rps: f64,
    pub after_rps: f64,
    pub improvement_percent: f64,
    pub reasoning: String,
    pub bottleneck_before: String,
    pub bottleneck_after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyImpact {
    pub p50_before_ms: f64,
    pub p50_after_ms: f64,
    pub p95_before_ms: f64,
    pub p95_after_ms: f64,
    pub p99_before_ms: f64,
    pub p99_after_ms: f64,
    pub reasoning: String,
    pub caveat: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCost {
    pub cpu_before: String,
    pub cpu_after: String,
    pub memory_before: String,
    pub memory_after: String,
    pub cost_delta_per_month: String,
    pub cost_savings_from_fewer_incidents: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingMetric {
    pub before_score: f64,
    pub after_score: f64,
    pub improvement_percent: f64,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Maintainability {
    pub before: String,
    pub after: String,
    pub deployment_change: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingEffort {
    pub estimated_hours: u32,
    pub effort_breakdown: EffortBreakdown,
    pub complexity_of_refactor: String,
    pub risk_of_bugs: f64,
    pub reversibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffortBreakdown {
    pub design_time: u32,
    pub kafka_setup: u32,
    pub worker_implementation: u32,
    pub testing_integration: u32,
    pub deployment_safety: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingDeps {
    pub new_external_dependencies: Vec<String>,
    pub infrastructure_changes: Vec<String>,
    pub api_breaking_changes: Vec<ApiBreakingChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiBreakingChange {
    pub endpoint: String,
    pub change: String,
    pub impact: String,
    pub migration_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingValidation {
    pub confidence: f64,
    pub confidence_reasoning: String,
    pub edge_cases: Vec<String>,
    pub validation_strategy: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingRoi {
    pub incidents_prevented: IncidentsPrevented,
    pub capacity_gain: CapacityGain,
    pub total_value_per_year: String,
    pub effort_cost: String,
    pub infrastructure_cost_per_month: String,
    pub roi_months: f64,
    pub priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentsPrevented {
    pub current_rate_per_year: u32,
    pub issue: String,
    pub predicted_rate_after: f64,
    pub incident_cost_per_year: String,
    pub incident_cost_prevented: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityGain {
    pub current_capacity_rps: f64,
    pub new_capacity_rps: f64,
    pub growth_runway_months: u32,
    pub cost_of_not_refactoring: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub phase_1: MigrationPhase,
    pub phase_2: MigrationPhase,
    pub phase_3: MigrationPhase,
    pub phase_4: MigrationPhase,
    pub rollback: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPhase {
    pub duration_weeks: u32,
    pub work: String,
    pub risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingSummary {
    pub total_couplings_found: u32,
    pub critical_couplings: u32,
    pub total_throughput_gain: String,
    pub total_effort_hours: u32,
    pub total_roi_months: f64,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingInfo {
    pub algorithm_version: String,
    pub execution_time_ms: u64,
    pub modules_analyzed: u32,
    pub co_change_pairs_analyzed: u32,
    pub confidence_level: String,
}

// ── Coupling type enum ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CouplingType {
    CircularDependency,
    CoChangeCluster,
    TightBinding,
    SequentialBottleneck,
}

impl CouplingType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CouplingType::CircularDependency => "circular_dependency",
            CouplingType::CoChangeCluster => "co_change_cluster",
            CouplingType::TightBinding => "tight_binding",
            CouplingType::SequentialBottleneck => "sequential_bottleneck",
        }
    }
}

// ── Config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CouplingConfig {
    pub enabled: bool,
    pub co_change_ratio_threshold: f64,
    pub chain_depth_threshold: u32,
}

impl Default for CouplingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            co_change_ratio_threshold: 0.6,
            chain_depth_threshold: 3,
        }
    }
}
