use crate::types::{ComplexityProfile, QueueingMetrics, RequestWeight, RuntimePrediction, SurvivabilityScore};

pub fn compute_survivability(
    components: RuntimePrediction,
    queueing: QueueingMetrics,
    complexity: ComplexityProfile,
    weight: RequestWeight,
) -> SurvivabilityScore {
    let cpu_risk = components.cpu_risk;
    let memory_risk = components.memory_risk;
    let gc_risk = components.gc_risk;
    let thread_risk = components.thread_risk;
    let latency_risk = components.latency_risk;
    let queueing_risk = compute_queueing_risk(&queueing);

    let max_risk = cpu_risk
        .max(memory_risk)
        .max(gc_risk)
        .max(thread_risk)
        .max(latency_risk)
        .max(queueing_risk);

    let value = (1.0 - max_risk).max(0.0).min(1.0);

    SurvivabilityScore {
        value,
        components,
        queueing,
        complexity,
        weight,
    }
}

fn compute_queueing_risk(q: &QueueingMetrics) -> f64 {
    if q.utilization.is_nan() || q.utilization.is_infinite() {
        return 1.0;
    }
    match q.utilization {
        u if u < 0.6 => u / 0.6 * 0.3,
        u if u < 0.8 => 0.3 + (u - 0.6) / 0.2 * 0.3,
        u if u < 1.0 => 0.6 + (u - 0.8) / 0.2 * 0.3,
        _ => 1.0,
    }
}

pub fn healthy_threshold(score: &SurvivabilityScore) -> bool {
    score.value >= 0.6
}

pub fn max_safe_rps(score: &SurvivabilityScore) -> f64 {
    let utilization_headroom = if score.queueing.utilization > 0.0 {
        (0.6 / score.queueing.utilization).min(10.0)
    } else {
        10.0
    };
    score.queueing.safe_rps * utilization_headroom
}

pub fn format_survivability_summary(score: &SurvivabilityScore) -> Vec<(String, String)> {
    vec![
        ("Survivability".into(), format!("{:.2}", score.value)),
        ("CPU Risk".into(), format!("{:.2}", score.components.cpu_risk)),
        ("Memory Risk".into(), format!("{:.2}", score.components.memory_risk)),
        ("GC Risk".into(), format!("{:.2}", score.components.gc_risk)),
        ("Thread Risk".into(), format!("{:.2}", score.components.thread_risk)),
        ("Latency Risk".into(), format!("{:.2}", score.components.latency_risk)),
        ("Complexity".into(), score.complexity.time_complexity.label().into()),
        ("Utilization".into(), format!("{:.1}%", score.queueing.utilization * 100.0)),
        ("Safe RPS".into(), format!("{:.0}", score.queueing.safe_rps)),
        ("Response Time".into(), format!("{:.1}ms", score.queueing.response_time_ms)),
        ("Payload Expansion".into(), format!("{:.1}x", score.weight.expansion_factor)),
        ("Predicted Memory".into(), format!("{:.1}KB/req", score.weight.total_bytes / 1024.0)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ComplexityClass, RuntimePrediction};

    fn sample_prediction() -> RuntimePrediction {
        RuntimePrediction {
            cpu_risk: 0.3,
            memory_risk: 0.2,
            gc_risk: 0.1,
            thread_risk: 0.05,
            latency_risk: 0.25,
        }
    }

    fn sample_queueing() -> QueueingMetrics {
        QueueingMetrics {
            arrival_rate: 100.0,
            service_rate: 500.0,
            utilization: 0.2,
            active_requests: 0.25,
            response_time_ms: 2.5,
            safe_rps: 300.0,
        }
    }

    fn sample_complexity() -> ComplexityProfile {
        ComplexityProfile {
            time_complexity: ComplexityClass::ON,
            loop_depth: 1,
            allocation_count: 10,
            branch_count: 5,
            function_count: 3,
        }
    }

    fn sample_weight() -> RequestWeight {
        RequestWeight {
            raw_bytes: 1024.0,
            expansion_factor: 3.0,
            runtime_bytes: 3072.0,
            temp_allocations: 256.0,
            total_bytes: 3328.0,
        }
    }

    #[test]
    fn test_compute_survivability_healthy() {
        let score = compute_survivability(
            sample_prediction(),
            sample_queueing(),
            sample_complexity(),
            sample_weight(),
        );
        assert!(score.value > 0.5);
        assert!(healthy_threshold(&score));
    }

    #[test]
    fn test_compute_survivability_failing() {
        let bad_pred = RuntimePrediction {
            cpu_risk: 0.9,
            memory_risk: 0.8,
            gc_risk: 0.7,
            thread_risk: 0.6,
            latency_risk: 0.85,
        };
        let bad_queueing = QueueingMetrics {
            arrival_rate: 10000.0,
            service_rate: 100.0,
            utilization: 100.0,
            active_requests: f64::MAX,
            response_time_ms: 30000.0,
            safe_rps: 60.0,
        };
        let score = compute_survivability(
            bad_pred,
            bad_queueing,
            sample_complexity(),
            sample_weight(),
        );
        assert!(score.value < 0.3);
        assert!(!healthy_threshold(&score));
        assert!(score.blocked());
    }

    #[test]
    fn test_survivability_score_severity_thresholds() {
        let base = SurvivabilityScore {
            value: 0.0,
            components: sample_prediction(),
            queueing: sample_queueing(),
            complexity: sample_complexity(),
            weight: sample_weight(),
        };

        let critical = SurvivabilityScore { value: 0.29, ..clone_score(&base) };
        assert_eq!(critical.severity(), sutra_schema::v1::Severity::Critical);
        assert!(critical.blocked());

        let error = SurvivabilityScore { value: 0.3, ..clone_score(&base) };
        assert_eq!(error.severity(), sutra_schema::v1::Severity::Error);
        assert!(!error.blocked());

        let warning = SurvivabilityScore { value: 0.6, ..clone_score(&base) };
        assert_eq!(warning.severity(), sutra_schema::v1::Severity::Warning);
        assert!(!warning.blocked());

        let info = SurvivabilityScore { value: 0.8, ..clone_score(&base) };
        assert_eq!(info.severity(), sutra_schema::v1::Severity::Info);
        assert!(!info.blocked());
    }

    fn clone_score(s: &SurvivabilityScore) -> SurvivabilityScore {
        SurvivabilityScore {
            value: s.value,
            components: s.components,
            queueing: s.queueing,
            complexity: s.complexity.clone(),
            weight: s.weight.clone(),
        }
    }

    #[test]
    fn test_survivability_score_overall_risk() {
        let score = SurvivabilityScore {
            value: 0.75,
            components: sample_prediction(),
            queueing: sample_queueing(),
            complexity: sample_complexity(),
            weight: sample_weight(),
        };
        assert!((score.overall_risk() - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_queueing_risk_low() {
        let q = QueueingMetrics {
            utilization: 0.2,
            ..sample_queueing()
        };
        let risk = compute_queueing_risk(&q);
        assert!(risk < 0.3);
        assert!(risk >= 0.0);
    }

    #[test]
    fn test_compute_queueing_risk_at_boundaries() {
        let q = sample_queueing();

        let at_06 = QueueingMetrics { utilization: 0.6, ..q };
        let risk_06 = compute_queueing_risk(&at_06);
        assert!((risk_06 - 0.3).abs() < 0.01);

        let at_08 = QueueingMetrics { utilization: 0.8, ..q };
        let risk_08 = compute_queueing_risk(&at_08);
        assert!((risk_08 - 0.6).abs() < 0.01);

        let at_10 = QueueingMetrics { utilization: 1.0, ..q };
        let risk_10 = compute_queueing_risk(&at_10);
        assert!((risk_10 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_queueing_risk_nan_utilization() {
        let q = QueueingMetrics {
            utilization: f64::NAN,
            ..sample_queueing()
        };
        let risk = compute_queueing_risk(&q);
        assert!((risk - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_queueing_risk_infinite_utilization() {
        let q = QueueingMetrics {
            utilization: f64::INFINITY,
            ..sample_queueing()
        };
        let risk = compute_queueing_risk(&q);
        assert!((risk - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_queueing_risk_high_but_under_one() {
        let q = QueueingMetrics {
            utilization: 0.95,
            ..sample_queueing()
        };
        let risk = compute_queueing_risk(&q);
        assert!(risk > 0.6 && risk < 1.0);
    }

    #[test]
    fn test_max_safe_rps() {
        let score = compute_survivability(
            sample_prediction(),
            sample_queueing(),
            sample_complexity(),
            sample_weight(),
        );
        let safe = max_safe_rps(&score);
        assert!(safe > 0.0);
    }

    #[test]
    fn test_max_safe_rps_zero_utilization() {
        let score = SurvivabilityScore {
            value: 0.95,
            components: sample_prediction(),
            queueing: QueueingMetrics { utilization: 0.0, ..sample_queueing() },
            complexity: sample_complexity(),
            weight: sample_weight(),
        };
        assert!((max_safe_rps(&score) - 10.0 * score.queueing.safe_rps).abs() < f64::EPSILON);
    }

    #[test]
    fn test_format_summary() {
        let score = compute_survivability(
            sample_prediction(),
            sample_queueing(),
            sample_complexity(),
            sample_weight(),
        );
        let summary = format_survivability_summary(&score);
        assert_eq!(summary.len(), 12);
    }
}
