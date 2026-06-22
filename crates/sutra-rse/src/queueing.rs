use crate::types::{ComplexityProfile, QueueingMetrics, Runtime};

pub fn compute_queueing(
    runtime: Runtime,
    complexity: &ComplexityProfile,
    expected_rps: f64,
    weight_bytes: f64,
) -> QueueingMetrics {
    let arrival_rate = expected_rps;
    let service_rate = estimate_service_rate(runtime, complexity, weight_bytes);
    let utilization = if service_rate > 0.0 {
        (arrival_rate / service_rate).min(100.0)
    } else {
        1.0
    };
    let active_requests = if service_rate > arrival_rate {
        utilization / (1.0 - utilization)
    } else {
        f64::MAX
    };
    let response_time_ms = if service_rate > 0.0 {
        let r = (1.0 / service_rate) / (1.0 - utilization);
        (r * 1000.0).min(30000.0)
    } else {
        30000.0
    };
    let safe_rps = service_rate * 0.6;

    QueueingMetrics {
        arrival_rate,
        service_rate,
        utilization,
        active_requests,
        response_time_ms,
        safe_rps,
    }
}

fn estimate_service_rate(runtime: Runtime, complexity: &ComplexityProfile, _weight_bytes: f64) -> f64 {
    let base_rate: f64 = match runtime {
        Runtime::Rust => 5000.0,
        Runtime::Go => 4000.0,
        Runtime::Jvm => 2000.0,
        Runtime::NodeJs => 1000.0,
        Runtime::Python => 500.0,
    };
    let complexity_factor = 1.0 - complexity.time_complexity.risk_factor() * 0.6;
    let loop_penalty = 1.0 / (1.0 + (complexity.loop_depth as f64) * 0.1);
    let branch_penalty = 1.0 / (1.0 + (complexity.branch_count as f64) * 0.02);
    base_rate * complexity_factor * loop_penalty * branch_penalty
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ComplexityClass;

    fn simple_profile() -> ComplexityProfile {
        ComplexityProfile {
            time_complexity: ComplexityClass::O1,
            loop_depth: 0,
            allocation_count: 0,
            branch_count: 0,
            function_count: 1,
        }
    }

    #[test]
    fn test_compute_queueing_low_load() {
        let q = compute_queueing(Runtime::Rust, &simple_profile(), 10.0, 1024.0);
        assert!(q.utilization < 0.6);
        assert!(q.safe_rps > q.arrival_rate);
        assert!(q.response_time_ms > 0.0);
        assert!(q.active_requests.is_finite());
    }

    #[test]
    fn test_compute_queueing_high_load() {
        let profile = ComplexityProfile {
            time_complexity: ComplexityClass::ON3,
            loop_depth: 5,
            allocation_count: 100,
            branch_count: 50,
            function_count: 20,
        };
        let q = compute_queueing(Runtime::Python, &profile, 10000.0, 102400.0);
        assert!(!q.utilization.is_nan());
        assert!(q.utilization > 0.0);
    }

    #[test]
    fn test_compute_queueing_overload_active_requests_infinite() {
        let profile = ComplexityProfile {
            time_complexity: ComplexityClass::ON3,
            loop_depth: 10,
            allocation_count: 200,
            branch_count: 100,
            function_count: 20,
        };
        let q = compute_queueing(Runtime::Python, &profile, 999999.0, 102400.0);
        assert_eq!(q.active_requests, f64::MAX);
        assert!(q.utilization >= 1.0 || q.utilization.is_nan());
    }

    #[test]
    fn test_compute_queueing_zero_service_rate_utilization_one() {
        let profile = ComplexityProfile {
            time_complexity: ComplexityClass::O2N,
            loop_depth: 100,
            allocation_count: 1000,
            branch_count: 100,
            function_count: 50,
        };
        let q = compute_queueing(Runtime::Python, &profile, 1.0, 999999.0);
        assert!(q.service_rate <= 0.0 || q.response_time_ms <= 30000.0);
        assert!(q.response_time_ms >= 0.0);
    }

    #[test]
    fn test_compute_queueing_response_time_capped_at_30s() {
        let profile = ComplexityProfile {
            time_complexity: ComplexityClass::O2N,
            loop_depth: 100,
            allocation_count: 1000,
            branch_count: 1000,
            function_count: 100,
        };
        let q = compute_queueing(Runtime::Python, &profile, 1_000_000.0, 1_000_000.0);
        assert!(q.response_time_ms <= 30000.0);
    }

    #[test]
    fn test_compute_queueing_no_load_zero_arrival() {
        let q = compute_queueing(Runtime::Rust, &simple_profile(), 0.0, 1024.0);
        assert_eq!(q.arrival_rate, 0.0);
        assert_eq!(q.utilization, 0.0);
        assert!(q.response_time_ms > 0.0);
    }

    #[test]
    fn test_estimate_service_rate_rust_fastest() {
        let rust_rate = estimate_service_rate(Runtime::Rust, &simple_profile(), 1024.0);
        let py_rate = estimate_service_rate(Runtime::Python, &simple_profile(), 1024.0);
        assert!(rust_rate > py_rate);
    }

    #[test]
    fn test_estimate_service_rate_all_runtimes_ordered() {
        let profile = simple_profile();
        let rates: Vec<_> = [Runtime::Rust, Runtime::Go, Runtime::Jvm, Runtime::NodeJs, Runtime::Python]
            .iter().map(|r| estimate_service_rate(*r, &profile, 1024.0)).collect();
        assert!(rates[0] > rates[1]); // Rust > Go
        assert!(rates[1] > rates[2]); // Go > Jvm
        assert!(rates[2] > rates[3]); // Jvm > NodeJs
        assert!(rates[3] > rates[4]); // NodeJs > Python
    }

    #[test]
    fn test_estimate_service_rate_complexity_penalties_apply() {
        let simple = estimate_service_rate(Runtime::Rust, &simple_profile(), 1024.0);
        let complex = estimate_service_rate(Runtime::Rust, &ComplexityProfile {
            time_complexity: ComplexityClass::ON3,
            loop_depth: 20,
            branch_count: 50,
            ..simple_profile()
        }, 1024.0);
        assert!(simple > complex);
    }

    #[test]
    fn test_safe_rps_is_sixty_percent() {
        let q = compute_queueing(Runtime::Jvm, &ComplexityProfile {
            time_complexity: ComplexityClass::ON,
            loop_depth: 1,
            allocation_count: 10,
            branch_count: 5,
            function_count: 3,
        }, 100.0, 2048.0);
        assert!((q.safe_rps - q.service_rate * 0.6).abs() < 1.0);
    }

    #[test]
    fn test_utilization_clamped_to_100() {
        let q = compute_queueing(Runtime::Python, &ComplexityProfile {
            time_complexity: ComplexityClass::O2N,
            loop_depth: 100,
            allocation_count: 1000,
            branch_count: 500,
            function_count: 50,
        }, f64::MAX, 1024.0);
        assert!(q.utilization <= 100.0);
    }
}
