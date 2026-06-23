use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Runtime {
    Jvm,
    NodeJs,
    Python,
    Rust,
    Go,
}

impl Runtime {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "jvm" | "java" | "kotlin" => Some(Runtime::Jvm),
            "node" | "nodejs" | "javascript" | "typescript" | "js" | "ts" => Some(Runtime::NodeJs),
            "python" | "py" => Some(Runtime::Python),
            "rust" | "rs" => Some(Runtime::Rust),
            "go" | "golang" => Some(Runtime::Go),
            _ => None,
        }
    }

    pub fn expansion_factor(&self) -> (f64, f64) {
        match self {
            Runtime::Jvm => (3.0, 10.0),
            Runtime::NodeJs => (2.0, 6.0),
            Runtime::Python => (4.0, 12.0),
            Runtime::Rust => (1.5, 4.0),
            Runtime::Go => (2.0, 5.0),
        }
    }

    pub fn avg_expansion(&self) -> f64 {
        let (lo, hi) = self.expansion_factor();
        (lo + hi) / 2.0
    }

    pub fn max_concurrent(&self) -> f64 {
        match self {
            Runtime::Jvm => 200.0,
            Runtime::NodeJs => 4.0,
            Runtime::Python => 8.0,
            Runtime::Rust => 512.0,
            Runtime::Go => 1000.0,
        }
    }

    pub fn default_memory_mb(&self) -> f64 {
        match self {
            Runtime::Jvm => 512.0,
            Runtime::NodeJs => 256.0,
            Runtime::Python => 128.0,
            Runtime::Rust => 128.0,
            Runtime::Go => 128.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RequestWeight {
    pub raw_bytes: f64,
    pub expansion_factor: f64,
    pub runtime_bytes: f64,
    pub temp_allocations: f64,
    pub total_bytes: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ComplexityProfile {
    pub time_complexity: ComplexityClass,
    pub loop_depth: u32,
    pub allocation_count: u32,
    pub branch_count: u32,
    pub function_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplexityClass {
    O1,
    OLogN,
    ON,
    ONLogN,
    ON2,
    ON3,
    O2N,
}

impl ComplexityClass {
    pub fn risk_factor(&self) -> f64 {
        match self {
            ComplexityClass::O1 => 0.0,
            ComplexityClass::OLogN => 0.1,
            ComplexityClass::ON => 0.3,
            ComplexityClass::ONLogN => 0.4,
            ComplexityClass::ON2 => 0.7,
            ComplexityClass::ON3 => 0.85,
            ComplexityClass::O2N => 1.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ComplexityClass::O1 => "O(1)",
            ComplexityClass::OLogN => "O(log n)",
            ComplexityClass::ON => "O(n)",
            ComplexityClass::ONLogN => "O(n log n)",
            ComplexityClass::ON2 => "O(n²)",
            ComplexityClass::ON3 => "O(n³)",
            ComplexityClass::O2N => "O(2ⁿ)",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QueueingMetrics {
    pub arrival_rate: f64,
    pub service_rate: f64,
    pub utilization: f64,
    pub active_requests: f64,
    pub response_time_ms: f64,
    pub safe_rps: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RuntimePrediction {
    pub cpu_risk: f64,
    pub memory_risk: f64,
    pub gc_risk: f64,
    pub thread_risk: f64,
    pub latency_risk: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivabilityScore {
    pub value: f64,
    pub components: RuntimePrediction,
    pub queueing: QueueingMetrics,
    pub complexity: ComplexityProfile,
    pub weight: RequestWeight,
}

impl SurvivabilityScore {
    pub fn severity(&self) -> sutra_schema::v1::Severity {
        if self.value >= 0.8 {
            sutra_schema::v1::Severity::Info
        } else if self.value >= 0.6 {
            sutra_schema::v1::Severity::Warning
        } else if self.value >= 0.3 {
            sutra_schema::v1::Severity::Error
        } else {
            sutra_schema::v1::Severity::Critical
        }
    }

    pub fn blocked(&self) -> bool {
        self.value < 0.3
    }

    pub fn overall_risk(&self) -> f64 {
        1.0 - self.value
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointAnalysis {
    pub path: String,
    pub method: String,
    pub runtime: Runtime,
    pub survivability: SurvivabilityScore,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RseConfig {
    pub enabled: bool,
    pub runtime: Option<String>,
    pub max_endpoints: usize,
    pub expected_rps: f64,
    pub memory_limit_mb: f64,
}

impl Default for RseConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            runtime: None,
            max_endpoints: 50,
            expected_rps: 30.0,
            memory_limit_mb: 512.0,
        }
    }
}

impl RseConfig {
    pub fn detect_runtime(&self, files: &[String]) -> Runtime {
        if let Some(ref name) = self.runtime {
            return Runtime::from_name(name).unwrap_or(Runtime::Rust);
        }
        for f in files {
            let low = f.to_lowercase();
            if low.ends_with(".java") || low.ends_with(".kt") {
                return Runtime::Jvm;
            }
            if low.ends_with(".js") || low.ends_with(".ts") || low.ends_with(".mjs") {
                return Runtime::NodeJs;
            }
            if low.ends_with(".py") {
                return Runtime::Python;
            }
            if low.ends_with(".rs") {
                return Runtime::Rust;
            }
            if low.ends_with(".go") {
                return Runtime::Go;
            }
        }
        Runtime::Rust
    }
}
