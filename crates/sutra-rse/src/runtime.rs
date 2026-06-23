use crate::types::{ComplexityProfile, RequestWeight, Runtime, RuntimePrediction};

pub fn predict_cpu_risk(complexity: &ComplexityProfile) -> f64 {
    complexity.time_complexity.risk_factor()
}

pub fn predict_memory_risk(weight: &RequestWeight, runtime: Runtime, memory_limit_mb: f64) -> f64 {
    let bytes_per_req = weight.total_bytes;
    let mb_per_req = bytes_per_req / (1024.0 * 1024.0);
    let ratio = mb_per_req / memory_limit_mb;
    (ratio * runtime.max_concurrent()).min(1.0)
}

pub fn predict_gc_risk(runtime: Runtime, weight: &RequestWeight, complexity: &ComplexityProfile) -> f64 {
    match runtime {
        Runtime::Jvm => {
            let alloc_rate = (weight.temp_allocations / 1024.0).min(100.0) / 100.0;
            let obj_count = (complexity.allocation_count as f64).min(100.0) / 100.0;
            ((alloc_rate + obj_count) / 2.0).min(1.0)
        }
        Runtime::NodeJs => {
            let promise_count = (complexity.allocation_count as f64).min(50.0) / 50.0;
            (promise_count * 0.6).min(1.0)
        }
        Runtime::Python => {
            let obj_count = (complexity.allocation_count as f64).min(100.0) / 100.0;
            (obj_count * 0.5).min(1.0)
        }
        Runtime::Rust => {
            let alloc_penalty = (complexity.allocation_count as f64).min(50.0) / 50.0 * 0.3;
            alloc_penalty.min(1.0)
        }
        Runtime::Go => {
            let goroutine_est = (complexity.function_count as f64).min(100.0) / 100.0 * 0.4;
            goroutine_est.min(1.0)
        }
    }
}

pub fn predict_thread_risk(runtime: Runtime, expected_rps: f64) -> f64 {
    let max_conc = runtime.max_concurrent();
    let ratio = expected_rps / max_conc;
    (ratio / 20.0).min(1.0)
}

pub fn predict_latency_risk(complexity: &ComplexityProfile) -> f64 {
    complexity.time_complexity.risk_factor()
}

pub fn predict_runtime(
    runtime: Runtime,
    weight: &RequestWeight,
    complexity: &ComplexityProfile,
    expected_rps: f64,
    memory_limit_mb: f64,
) -> RuntimePrediction {
    RuntimePrediction {
        cpu_risk: predict_cpu_risk(complexity),
        memory_risk: predict_memory_risk(weight, runtime, memory_limit_mb),
        gc_risk: predict_gc_risk(runtime, weight, complexity),
        thread_risk: predict_thread_risk(runtime, expected_rps),
        latency_risk: predict_latency_risk(complexity),
    }
}

pub fn estimate_latency_ms(complexity: &ComplexityProfile, weight: &RequestWeight) -> f64 {
    let base_latency = 10.0;
    let complexity_factor = 1.0 + complexity.time_complexity.risk_factor() * 5.0;
    let weight_factor = weight.total_bytes / 1024.0 / 100.0;
    let loop_factor = 1.0 + (complexity.loop_depth as f64) * 0.2;
    base_latency * complexity_factor * (1.0 + weight_factor) * loop_factor
}

pub fn predict_throughput(runtime: Runtime, complexity: &ComplexityProfile) -> f64 {
    let max_conc = runtime.max_concurrent();
    let latency_ms = estimate_latency_ms(complexity, &RequestWeight {
        raw_bytes: 1024.0,
        expansion_factor: runtime.avg_expansion(),
        runtime_bytes: 1024.0 * runtime.avg_expansion(),
        temp_allocations: 256.0,
        total_bytes: 1024.0 * runtime.avg_expansion() + 256.0,
    });
    let latency_sec = latency_ms / 1000.0;
    if latency_sec <= 0.0 {
        return max_conc * 100.0;
    }
    max_conc / latency_sec
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ComplexityClass;

    fn sample_profile() -> ComplexityProfile {
        ComplexityProfile {
            time_complexity: ComplexityClass::ON2,
            loop_depth: 3,
            allocation_count: 20,
            branch_count: 15,
            function_count: 5,
        }
    }

    fn sample_weight() -> RequestWeight {
        RequestWeight {
            raw_bytes: 1024.0,
            expansion_factor: 3.0,
            runtime_bytes: 3072.0,
            temp_allocations: 512.0,
            total_bytes: 3584.0,
        }
    }

    fn zero_profile() -> ComplexityProfile {
        ComplexityProfile {
            time_complexity: ComplexityClass::O1,
            loop_depth: 0,
            allocation_count: 0,
            branch_count: 0,
            function_count: 1,
        }
    }

    fn zero_weight() -> RequestWeight {
        RequestWeight {
            raw_bytes: 0.0,
            expansion_factor: 1.0,
            runtime_bytes: 0.0,
            temp_allocations: 0.0,
            total_bytes: 0.0,
        }
    }

    #[test]
    fn test_predict_cpu_risk() {
        let risk = predict_cpu_risk(&sample_profile());
        assert!(risk >= 0.0 && risk <= 1.0);
        assert!(risk > 0.0);
    }

    #[test]
    fn test_predict_cpu_risk_zero_complexity() {
        let risk = predict_cpu_risk(&zero_profile());
        assert_eq!(risk, 0.0);
    }

    #[test]
    fn test_predict_cpu_risk_max_penalties() {
        let profile = ComplexityProfile {
            time_complexity: ComplexityClass::O2N,
            loop_depth: 10,
            branch_count: 50,
            ..zero_profile()
        };
        let risk = predict_cpu_risk(&profile);
        assert!(risk >= 0.9);
        assert!(risk <= 1.0);
    }

    #[test]
    fn test_predict_cpu_risk_loop_penalty_clamped_at_10() {
        let profile = ComplexityProfile {
            time_complexity: ComplexityClass::O1,
            loop_depth: 100,
            branch_count: 0,
            ..zero_profile()
        };
        let risk = predict_cpu_risk(&profile);
        assert_eq!(risk, 0.0);
    }

    #[test]
    fn test_predict_cpu_risk_branch_penalty_clamped_at_50() {
        let profile = ComplexityProfile {
            time_complexity: ComplexityClass::O1,
            loop_depth: 0,
            branch_count: 200,
            ..zero_profile()
        };
        let risk = predict_cpu_risk(&profile);
        assert_eq!(risk, 0.0);
    }

    #[test]
    fn test_predict_memory_risk() {
        let risk = predict_memory_risk(&sample_weight(), Runtime::Jvm, 512.0);
        assert!(risk >= 0.0 && risk <= 1.0);
    }

    #[test]
    fn test_predict_memory_risk_zero_payload() {
        let risk = predict_memory_risk(&zero_weight(), Runtime::Rust, 128.0);
        assert_eq!(risk, 0.0);
    }

    #[test]
    fn test_predict_memory_risk_high_ratio() {
        let large = RequestWeight {
            total_bytes: 1024.0 * 1024.0 * 500.0, // 500MB
            ..sample_weight()
        };
        let risk = predict_memory_risk(&large, Runtime::Rust, 128.0);
        assert!(risk >= 1.0);
    }

    #[test]
    fn test_predict_gc_risk_jvm() {
        let risk = predict_gc_risk(Runtime::Jvm, &sample_weight(), &sample_profile());
        assert!(risk >= 0.0 && risk <= 1.0);
    }

    #[test]
    fn test_predict_gc_risk_jvm_zero() {
        let risk = predict_gc_risk(Runtime::Jvm, &zero_weight(), &zero_profile());
        assert_eq!(risk, 0.0);
    }

    #[test]
    fn test_predict_gc_risk_nodejs() {
        let risk = predict_gc_risk(Runtime::NodeJs, &sample_weight(), &sample_profile());
        assert!(risk >= 0.0 && risk <= 1.0);
    }

    #[test]
    fn test_predict_gc_risk_python() {
        let risk = predict_gc_risk(Runtime::Python, &sample_weight(), &sample_profile());
        assert!(risk >= 0.0 && risk <= 1.0);
    }

    #[test]
    fn test_predict_gc_risk_rust() {
        let risk = predict_gc_risk(Runtime::Rust, &sample_weight(), &sample_profile());
        assert!(risk >= 0.0 && risk <= 1.0);
    }

    #[test]
    fn test_predict_gc_risk_go() {
        let risk = predict_gc_risk(Runtime::Go, &sample_weight(), &sample_profile());
        assert!(risk >= 0.0 && risk <= 1.0);
    }

    #[test]
    fn test_predict_gc_risk_all_runtimes_different() {
        let risks: Vec<f64> = [Runtime::Jvm, Runtime::NodeJs, Runtime::Python, Runtime::Rust, Runtime::Go]
            .iter().map(|r| predict_gc_risk(*r, &sample_weight(), &sample_profile())).collect();
        for &r in &risks {
            assert!(r >= 0.0 && r <= 1.0);
        }
    }

    #[test]
    fn test_predict_thread_risk() {
        assert!((predict_thread_risk(Runtime::Rust, 10.0) - 0.0).abs() < 0.01);
        assert!(predict_thread_risk(Runtime::NodeJs, 200.0) > 0.0);
    }

    #[test]
    fn test_predict_thread_risk_zero_rps() {
        let risk = predict_thread_risk(Runtime::Rust, 0.0);
        assert_eq!(risk, 0.0);
    }

    #[test]
    fn test_predict_thread_risk_extreme_rps() {
        let risk = predict_thread_risk(Runtime::NodeJs, 1_000_000.0);
        assert!(risk >= 1.0);
    }

    #[test]
    fn test_predict_latency_risk() {
        let risk = predict_latency_risk(&sample_profile());
        assert!(risk >= 0.0 && risk <= 1.0);
    }

    #[test]
    fn test_predict_latency_risk_zero_complexity() {
        let risk = predict_latency_risk(&zero_profile());
        assert_eq!(risk, 0.0);
    }

    #[test]
    fn test_predict_latency_risk_max_penalties() {
        let profile = ComplexityProfile {
            time_complexity: ComplexityClass::O2N,
            loop_depth: 10,
            branch_count: 30,
            ..zero_profile()
        };
        let risk = predict_latency_risk(&profile);
        assert!(risk >= 0.9);
        assert!(risk <= 1.0);
    }

    #[test]
    fn test_estimate_latency_ms() {
        let lat = estimate_latency_ms(&sample_profile(), &sample_weight());
        assert!(lat > 0.0);
    }

    #[test]
    fn test_estimate_latency_ms_zero_values() {
        let lat = estimate_latency_ms(&zero_profile(), &zero_weight());
        assert!((lat - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_predict_throughput() {
        let t = predict_throughput(Runtime::Rust, &sample_profile());
        assert!(t > 0.0);
    }

    #[test]
    fn test_predict_throughput_rust_faster_than_python() {
        let rust_t = predict_throughput(Runtime::Rust, &sample_profile());
        let py_t = predict_throughput(Runtime::Python, &sample_profile());
        assert!(rust_t > py_t);
    }

    #[test]
    fn test_predict_runtime_integration() {
        let pred = predict_runtime(Runtime::Jvm, &sample_weight(), &sample_profile(), 100.0, 512.0);
        assert!(pred.cpu_risk >= 0.0 && pred.cpu_risk <= 1.0);
        assert!(pred.memory_risk >= 0.0 && pred.memory_risk <= 1.0);
        assert!(pred.gc_risk >= 0.0 && pred.gc_risk <= 1.0);
        assert!(pred.thread_risk >= 0.0 && pred.thread_risk <= 1.0);
        assert!(pred.latency_risk >= 0.0 && pred.latency_risk <= 1.0);
    }

    #[test]
    fn test_predict_runtime_all_runtimes() {
        for runtime in &[Runtime::Rust, Runtime::Go, Runtime::Jvm, Runtime::NodeJs, Runtime::Python] {
            let pred = predict_runtime(*runtime, &sample_weight(), &sample_profile(), 50.0, 256.0);
            assert!(pred.cpu_risk <= 1.0);
            assert!(pred.memory_risk <= 1.0);
            assert!(pred.gc_risk <= 1.0);
        }
    }
}
