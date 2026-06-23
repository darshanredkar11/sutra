use serde::{Deserialize, Serialize};

// ── Input types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtInput {
    pub refactoring_spec: Option<serde_json::Value>,
    pub coupling_spec: Option<serde_json::Value>,
    pub performance_spec: Option<serde_json::Value>,
    pub testing_gap_spec: Option<serde_json::Value>,
}

// ── Output types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtRoiSpec {
    pub engine: String,
    pub debt_items: Vec<DebtItem>,
    pub ranked_by_roi: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtItem {
    pub id: String,
    pub category: String,
    pub issue: String,
    pub source_engine: String,
    pub current_cost: CurrentCost,
    pub payoff_cost: PayoffCost,
    pub roi: DebtRoi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentCost {
    pub incident_rate_per_year: Option<u32>,
    pub incident_cost_each: Option<u32>,
    pub incident_cost_annual: Option<u32>,
    pub maintenance_cost_hours_annual: Option<u32>,
    pub maintenance_cost_annual: Option<u32>,
    pub timeout_failures_per_year: Option<u32>,
    pub timeout_cost_each: Option<u32>,
    pub timeout_cost_annual: Option<u32>,
    pub scalability_limit: Option<String>,
    pub growth_opportunity_cost: Option<u32>,
    pub total_annual_cost: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoffCost {
    pub effort_hours: u32,
    pub effort_cost: u32,
    pub infrastructure_cost_monthly: Option<u32>,
    pub infrastructure_cost_annual: Option<u32>,
    pub total_payoff_cost: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtRoi {
    pub annual_savings: Option<u32>,
    pub net_value_first_year: Option<u32>,
    pub payoff_months: f64,
    pub roi_months: f64,
    pub priority: u32,
}

// ── Config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DebtRoiConfig {
    pub enabled: bool,
}

impl Default for DebtRoiConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}
