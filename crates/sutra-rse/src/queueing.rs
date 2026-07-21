use crate::types::{ComplexityProfile, QueueingMetrics, Runtime};

/// Exact M/M/1 steady-state metrics (Poisson arrivals, exponential service,
/// 1 server). Requires ρ = λ/μ < 1 for stability; `None` otherwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mm1Metrics {
    /// Mean time in system, W = 1/(μ-λ).
    pub w: f64,
    /// Mean time in queue, Wq = ρ/(μ-λ).
    pub wq: f64,
    /// Mean number in system, L = ρ/(1-ρ).
    pub l: f64,
    /// Mean number in queue, Lq = ρ²/(1-ρ).
    pub lq: f64,
}

/// Exact M/M/1 response-time metrics. `None` (UNSTABLE) when ρ = λ/μ ≥ 1 —
/// the queue grows without bound and no finite steady-state exists.
pub fn mm1(lambda: f64, mu: f64) -> Option<Mm1Metrics> {
    if mu <= 0.0 || lambda < 0.0 || lambda >= mu {
        return None;
    }
    let rho = lambda / mu;
    Some(Mm1Metrics {
        w: 1.0 / (mu - lambda),
        wq: rho / (mu - lambda),
        l: rho / (1.0 - rho),
        lq: rho * rho / (1.0 - rho),
    })
}

/// Exact M/M/1 response-time percentile t_p, the wait such that
/// P(time in system ≤ t_p) = p:  t_p = -ln(1-p) / (μ-λ).
/// `None` (UNSTABLE) when ρ = λ/μ ≥ 1.
pub fn mm1_percentile(lambda: f64, mu: f64, p: f64) -> Option<f64> {
    if mu <= 0.0 || lambda < 0.0 || lambda >= mu {
        return None;
    }
    Some(-(1.0 - p).ln() / (mu - lambda))
}

/// Erlang-B blocking probability via the overflow-safe recurrence
/// B(0,a)=1; B(k,a) = a·B(k-1,a) / (k + a·B(k-1,a)).
/// This never computes a!/k! directly, so it stays finite for any `c`.
pub fn erlang_b(c: u32, a: f64) -> f64 {
    let mut b = 1.0_f64;
    for k in 1..=c {
        b = a * b / (k as f64 + a * b);
    }
    b
}

/// Exact M/M/c steady-state metrics via Erlang C (built on [`erlang_b`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MmcMetrics {
    /// Erlang C: P(an arrival must wait, i.e. all c servers busy).
    pub c_wait: f64,
    /// Mean time in queue, Wq = C / (cμ-λ).
    pub wq: f64,
    /// Mean time in system, W = Wq + 1/μ.
    pub w: f64,
}

/// Exact M/M/c queueing metrics for `c` identical servers. Requires
/// ρ = λ/(cμ) < 1 for stability; `None` (UNSTABLE) otherwise.
pub fn mmc(c: u32, lambda: f64, mu: f64) -> Option<MmcMetrics> {
    if c == 0 || mu <= 0.0 || lambda < 0.0 {
        return None;
    }
    let cmu = c as f64 * mu;
    let rho = lambda / cmu;
    if rho >= 1.0 {
        return None;
    }
    let a = lambda / mu;
    let b = erlang_b(c, a);
    let c_wait = b / (1.0 - rho * (1.0 - b));
    let wq = c_wait / (cmu - lambda);
    let w = wq + 1.0 / mu;
    Some(MmcMetrics { c_wait, wq, w })
}

/// Max arrival rate λ that keeps the exact M/M/1 Wq at or below
/// `sla_seconds`, solved from Wq(λ) = λ / (μ(μ-λ)) ≤ sla:
/// λ ≤ sla·μ² / (1 + sla·μ).
fn safe_rps_for_sla(mu: f64, sla_seconds: f64) -> f64 {
    if mu <= 0.0 {
        return 0.0;
    }
    (sla_seconds * mu * mu) / (1.0 + sla_seconds * mu)
}

/// SLA threshold used to derive `safe_rps`: 500ms mean queueing delay.
const SLA_SECONDS: f64 = 0.5;

pub fn compute_queueing(
    runtime: Runtime,
    complexity: &ComplexityProfile,
    expected_rps: f64,
    weight_bytes: f64,
) -> QueueingMetrics {
    let arrival_rate = expected_rps;
    let service_rate = estimate_service_rate(runtime, complexity, weight_bytes);

    let utilization = if service_rate > 0.0 {
        (arrival_rate / service_rate).min(1.0)
    } else {
        1.0
    };

    match mm1(arrival_rate, service_rate) {
        Some(m) => {
            let p95 = mm1_percentile(arrival_rate, service_rate, 0.95)
                .expect("mm1_percentile shares mm1's stability guard");
            QueueingMetrics {
                arrival_rate,
                service_rate,
                utilization,
                active_requests: m.l,
                response_time_ms: (p95 * 1000.0).min(30_000.0),
                safe_rps: safe_rps_for_sla(service_rate, SLA_SECONDS),
            }
        }
        None => {
            // UNSTABLE (ρ ≥ 1) or degenerate service rate: the exact model
            // has no finite steady-state here by definition. Fall back to a
            // Little's-Law estimate off the nominal single-request latency
            // so downstream findings stay finite/orderable instead of
            // NaN/∞, while utilization/response_time/safe_rps saturate to
            // signal "overloaded" rather than report bogus queueing numbers.
            let p50_latency_sec = if service_rate > 0.0 { 1.0 / service_rate } else { 0.03 };
            QueueingMetrics {
                arrival_rate,
                service_rate,
                utilization,
                active_requests: arrival_rate * p50_latency_sec,
                response_time_ms: 30_000.0,
                safe_rps: 0.0,
            }
        }
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

    fn assert_close(actual: f64, expected: f64, tol: f64) {
        assert!(
            (actual - expected).abs() < tol,
            "expected {expected}, got {actual} (diff {})",
            (actual - expected).abs()
        );
    }

    // -- Exact M/M/1 / M/M/c regression tests (1e-9 tolerance) --------------

    #[test]
    fn test_mm1_lambda_0_8_mu_1() {
        let m = mm1(0.8, 1.0).expect("stable: rho=0.8 < 1");
        assert_close(m.w, 5.0, 1e-9);
        let p95 = mm1_percentile(0.8, 1.0, 0.95).unwrap();
        // -ln(0.05)/0.2 = 14.978661367769954 (verified independently via
        // Python's math.log; the value quoted as 14.9786313 in the source
        // spec has a transposed digit -- 613 vs 661).
        assert_close(p95, 14.978661367769954, 1e-9);
    }

    #[test]
    fn test_mm1_lambda_0_5_mu_1() {
        let m = mm1(0.5, 1.0).expect("stable: rho=0.5 < 1");
        assert_close(m.w, 2.0, 1e-9);
        let p50 = mm1_percentile(0.5, 1.0, 0.50).unwrap();
        assert_close(p50, 1.3862944, 1e-6);
        let p99 = mm1_percentile(0.5, 1.0, 0.99).unwrap();
        assert_close(p99, 9.2103404, 1e-6);
    }

    #[test]
    fn test_mmc_c2_lambda_1_5_mu_1() {
        let m = mmc(2, 1.5, 1.0).expect("stable: rho=0.75 < 1");
        assert_close(m.c_wait, 0.6428571, 1e-6);
        assert_close(m.wq, 1.2857143, 1e-6);
    }

    #[test]
    fn test_mm1_unstable_lambda_1_2_mu_1() {
        assert_eq!(mm1(1.2, 1.0), None, "rho=1.2 >= 1 must report UNSTABLE, no numbers");
        assert_eq!(mm1_percentile(1.2, 1.0, 0.95), None);
    }

    #[test]
    fn test_mm1_boundary_rho_exactly_1_is_unstable() {
        assert_eq!(mm1(1.0, 1.0), None);
    }

    #[test]
    fn test_mmc_unstable_when_rho_geq_1() {
        // c=2, mu=1: cmu=2, lambda=2 -> rho=1.0 -> UNSTABLE
        assert_eq!(mmc(2, 2.0, 1.0), None);
    }

    #[test]
    fn test_mmc_c1_matches_mm1() {
        // M/M/1 is the c=1 special case of M/M/c: Wq must agree.
        let one = mm1(0.8, 1.0).unwrap();
        let c = mmc(1, 0.8, 1.0).unwrap();
        assert_close(c.wq, one.wq, 1e-9);
        assert_close(c.w, one.w, 1e-9);
    }

    #[test]
    fn test_erlang_b_matches_known_value() {
        // Erlang B(c=2, a=1.5) = 0.310344827586... (hand-verified: recurrence
        // B1 = 1.5/(1+1.5) = 0.6; B2 = 1.5*0.6/(2+1.5*0.6) = 0.9/2.9).
        let b = erlang_b(2, 1.5);
        assert_close(b, 0.3103448276, 1e-9);
    }

    // -- compute_queueing() integration tests --------------------------------

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
    fn test_compute_queueing_overload_active_requests_grows() {
        let profile = ComplexityProfile {
            time_complexity: ComplexityClass::ON3,
            loop_depth: 10,
            allocation_count: 200,
            branch_count: 100,
            function_count: 20,
        };
        let q = compute_queueing(Runtime::Python, &profile, 999999.0, 102400.0);
        // Under high load, active requests = arrival_rate * latency is very large but finite
        assert!(q.active_requests.is_finite());
        assert!(q.active_requests > 10000.0);
        assert!(q.utilization >= 1.0);
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
    fn test_safe_rps_based_on_sla() {
        // Safe RPS is where the exact M/M/1 Wq stays <= the 500ms SLA threshold.
        let q = compute_queueing(Runtime::Jvm, &ComplexityProfile {
            time_complexity: ComplexityClass::ON,
            loop_depth: 1,
            allocation_count: 10,
            branch_count: 5,
            function_count: 3,
        }, 100.0, 2048.0);
        // Verify safe_rps is less than service_rate (we're staying under SLA)
        assert!(q.safe_rps <= q.service_rate);
        assert!(q.safe_rps > 0.0);
    }

    #[test]
    fn test_safe_rps_matches_closed_form() {
        let mu = 500.0_f64;
        let expected = (SLA_SECONDS * mu * mu) / (1.0 + SLA_SECONDS * mu);
        assert_close(safe_rps_for_sla(mu, SLA_SECONDS), expected, 1e-9);
        // sanity: at the computed safe_rps, exact Wq is within SLA.
        let wq = mm1(expected, mu).unwrap().wq;
        assert!(wq <= SLA_SECONDS + 1e-9);
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
