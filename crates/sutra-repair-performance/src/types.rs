use serde::{Deserialize, Serialize};

// ── Input types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProfile {
    pub function: String,
    pub call_frequency: CallFrequency,
    pub latency: LatencyStats,
    pub breakdown: Vec<StageBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallFrequency {
    pub calls_per_second: f64,
    pub peak_rps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageBreakdown {
    pub stage: String,
    pub latency_ms: u32,
    pub variance_ms: u32,
    pub io_ops: u32,
    pub io_type: Option<String>,
    pub rows_returned: Option<u32>,
    pub expensive_operations: Option<Vec<String>>,
    pub allocations: Option<u32>,
    pub allocation_size_bytes: Option<u64>,
    pub cache_hit_rate: Option<f64>,
    pub cache_miss_cost_ms: Option<u32>,
    pub data_size_kb: Option<u32>,
    pub network_latency_ms: Option<u32>,
    pub timeout_rate: Option<f64>,
    pub batch_potential: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub memory: MemoryMetrics,
    pub cpu: CpuMetrics,
    pub network: NetworkMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub baseline_mb: u32,
    pub per_request_mb: u32,
    pub peak_memory_mb: u32,
    pub gc_pause_ms: u32,
    pub gc_frequency_per_second: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub cores_available: u32,
    pub cpu_percent_at_8rps: u32,
    pub cpu_percent_at_peak: u32,
    pub hotspots: Vec<CpuHotspot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuHotspot {
    pub function: String,
    pub cpu_percent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub bandwidth_kb_sec: u32,
    pub bandwidth_available_kb_sec: u32,
    pub timeout_rate: f64,
}

// ── Output types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSpec {
    pub engine: String,
    pub analysis_scope: String,
    pub bottlenecks_found: Vec<Bottleneck>,
    pub combined_optimization_plan: CombinedPlan,
    pub processing: ProcessingInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    pub id: String,
    pub r#type: String,
    pub severity: String,
    pub description: String,
    pub current_bottleneck: CurrentBottleneck,
    pub optimization_strategies: Vec<OptimizationStrategy>,
    pub recommended_optimizations: Vec<RecommendedOpt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentBottleneck {
    pub function: String,
    pub stage_latency_ms: u32,
    pub stage_variance_ms: u32,
    pub total_function_latency_ms: u32,
    pub bottleneck_contribution_percent: u32,
    pub root_cause: String,
    pub measurement_confidence: f64,
    pub measurement_basis: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationStrategy {
    pub id: String,
    pub strategy: String,
    pub description: String,
    pub mechanism: serde_json::Value,
    pub impact: OptImpact,
    pub implementation: OptImplementation,
    pub validation: OptValidation,
    pub constraints: OptConstraints,
    pub roi: OptRoi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptImpact {
    pub latency_before_ms: Option<u32>,
    pub latency_after_ms: Option<u32>,
    pub latency_reduction_percent: Option<u32>,
    pub throughput_before_rps: Option<f64>,
    pub throughput_after_rps: Option<f64>,
    pub throughput_gain_percent: Option<u32>,
    pub caveat: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptImplementation {
    pub approach: String,
    pub code_changes: Vec<String>,
    pub effort_hours: u32,
    pub complexity: String,
    pub risk_of_bugs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptValidation {
    pub confidence: f64,
    pub test_strategy: Vec<String>,
    pub caveat: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptConstraints {
    pub tradeoff: String,
    pub edge_case: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptRoi {
    pub effort_hours: Option<u32>,
    pub effort_cost: String,
    pub infrastructure_savings: Option<String>,
    pub infrastructure_cost: Option<String>,
    pub incident_prevention: Option<String>,
    pub throughput_gain_value: Option<String>,
    pub total_value: String,
    pub roi_months: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedOpt {
    pub id: String,
    pub rationale: String,
    pub priority: u32,
    pub implement_now: bool,
    pub trigger: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedPlan {
    pub optimizations: Vec<PlanItem>,
    pub total_effort_hours: u32,
    pub total_latency_reduction_ms: u32,
    pub latency_before_p95_ms: u32,
    pub latency_after_p95_ms: u32,
    pub latency_reduction_percent: u32,
    pub throughput_before_rps: f64,
    pub throughput_after_rps: f64,
    pub roi_months: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanItem {
    pub id: String,
    pub effort_hours: u32,
    pub latency_reduction_ms: u32,
    pub priority: u32,
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingInfo {
    pub algorithm_version: String,
    pub execution_time_ms: u64,
    pub functions_analyzed: u32,
    pub bottlenecks_detected: u32,
    pub optimizations_proposed: u32,
    pub confidence_level: String,
}

// ── Config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    pub enabled: bool,
    pub latency_threshold_ms: u32,
    pub sla_p95_ms: f64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            latency_threshold_ms: 100,
            sla_p95_ms: 500.0,
        }
    }
}
