use crate::types::EvalMetrics;
use crate::model::predict;
use crate::types::{FeatureVector, ModelParams};

struct PredictionPair {
    probability: f64,
    label: bool,
}

/// Compute the confusion matrix and derived metrics.
pub fn evaluate(
    params: &ModelParams,
    features: &[FeatureVector],
    labels: &[bool],
) -> EvalMetrics {
    let n = features.len().min(labels.len());
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut tn = 0usize;
    let mut fn_ = 0usize;

    for i in 0..n {
        let prob = predict(params, &features[i]);
        let predicted = prob >= 0.5;
        let actual = labels[i];

        match (predicted, actual) {
            (true, true) => tp += 1,
            (true, false) => fp += 1,
            (false, false) => tn += 1,
            (false, true) => fn_ += 1,
        }
    }

    EvalMetrics::new(tp, fp, tn, fn_)
}

/// Compute AUC (area under ROC curve) using the trapezoidal rule.
pub fn compute_auc(
    params: &ModelParams,
    features: &[FeatureVector],
    labels: &[bool],
) -> f64 {
    let n = features.len().min(labels.len());
    let mut pairs: Vec<PredictionPair> = (0..n)
        .map(|i| PredictionPair {
            probability: predict(params, &features[i]),
            label: labels[i],
        })
        .collect();

    pairs.sort_by(|a, b| b.probability.partial_cmp(&a.probability).unwrap_or(std::cmp::Ordering::Equal));

    let total_pos = labels.iter().filter(|l| **l).count() as f64;
    let total_neg = labels.len() as f64 - total_pos;

    if total_pos == 0.0 || total_neg == 0.0 {
        return 0.5;
    }

    let mut roc_points: Vec<(f64, f64)> = Vec::new();
    roc_points.push((0.0, 0.0));

    let mut fp_count = 0usize;
    let mut tp_count = 0usize;

    for pair in &pairs {
        if pair.label {
            tp_count += 1;
        } else {
            fp_count += 1;
        }
        let fpr = fp_count as f64 / total_neg;
        let tpr = tp_count as f64 / total_pos;
        roc_points.push((fpr, tpr));
    }

    let mut auc = 0.0;
    for i in 1..roc_points.len() {
        let (x1, y1) = roc_points[i - 1];
        let (x2, y2) = roc_points[i];
        auc += (x2 - x1) * (y1 + y2) / 2.0;
    }

    auc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FeatureVector, LabeledExample, NUM_FEATURES};
    use crate::model::train;

    fn make_fv(data: [f64; NUM_FEATURES]) -> FeatureVector {
        FeatureVector::new(data)
    }

    #[test]
    fn test_evaluate_perfect() {
        let params = ModelParams::zero();
        let features = vec![make_fv([1.0; NUM_FEATURES]), make_fv([0.0; NUM_FEATURES])];
        let labels = vec![true, false];

        // Zero weights always give 0.5, so these will be mixed
        let metrics = evaluate(&params, &features, &labels);
        assert_eq!(metrics.total_samples, 2);
    }

    #[test]
    fn test_evaluate_all_correct() {
        let mut params = ModelParams::zero();
        params.weights[0] = 10.0;
        params.means = [0.0; NUM_FEATURES];
        params.stds = [1.0; NUM_FEATURES];

        let features = vec![
            make_fv([10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),  // positive
            make_fv([-10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]), // negative
        ];
        let labels = vec![true, false];

        let metrics = evaluate(&params, &features, &labels);
        assert_eq!(metrics.true_positives, 1);
        assert_eq!(metrics.true_negatives, 1);
        assert_eq!(metrics.false_positives, 0);
        assert_eq!(metrics.false_negatives, 0);
        assert!((metrics.accuracy - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_evaluate_all_wrong() {
        let mut params = ModelParams::zero();
        params.weights[0] = 10.0; // positive weight → high feature predicts true
        params.means = [0.0; NUM_FEATURES];
        params.stds = [1.0; NUM_FEATURES];

        let features = vec![
            make_fv([10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            make_fv([-10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        ];
        let labels = vec![false, true]; // flipped: first should be false, second true

        let metrics = evaluate(&params, &features, &labels);
        assert_eq!(metrics.true_positives, 0);
        assert_eq!(metrics.true_negatives, 0);
        // Both predictions are wrong but opposite classes
        assert_eq!(metrics.false_positives + metrics.false_negatives, 2);
    }

    #[test]
    fn test_evaluate_from_trained_model() {
        let mut examples = Vec::new();
        for i in 0..10 {
            let rev = if i < 5 { 1.0 } else { 15.0 };
            let bug = if i < 5 { 0.0 } else { 5.0 };
            examples.push(LabeledExample::new(
                make_fv([rev, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, bug, 0.0, 0.0]),
                i >= 5,
            ));
        }

        let params = train(&examples, 0.1, 0.01, 200);
        let features: Vec<FeatureVector> = examples.iter().map(|e| e.features.clone()).collect();
        let labels: Vec<bool> = examples.iter().map(|e| e.label).collect();

        let metrics = evaluate(&params, &features, &labels);
        assert!(metrics.accuracy >= 0.8, "accuracy should be >= 0.8, got {}", metrics.accuracy);
    }

    #[test]
    fn test_auc_near_random() {
        let mut params = ModelParams::zero();
        params.weights[0] = 0.01; // tiny weight — near random, AUC should be ~0.5
        params.means = [0.0; NUM_FEATURES];
        params.stds = [1.0; NUM_FEATURES];

        let features = vec![
            make_fv([1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            make_fv([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        ];
        let labels = vec![true, false];

        let auc = compute_auc(&params, &features, &labels);
        assert!(auc > 0.0, "AUC should be computable, got {}", auc);
    }

    #[test]
    fn test_auc_all_same_label() {
        let params = ModelParams::zero();
        let features = vec![make_fv([1.0; NUM_FEATURES]), make_fv([0.0; NUM_FEATURES])];
        let labels = vec![true, true];

        let auc = compute_auc(&params, &features, &labels);
        assert!((auc - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_auc_perfect() {
        let mut params = ModelParams::zero();
        params.weights[0] = 10.0;
        params.means = [0.0; NUM_FEATURES];
        params.stds = [1.0; NUM_FEATURES];

        let features = vec![
            make_fv([10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            make_fv([-10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        ];
        let labels = vec![true, false];

        let auc = compute_auc(&params, &features, &labels);
        assert!((auc - 1.0).abs() < 1e-10, "perfect AUC should be 1.0, got {}", auc);
    }

    #[test]
    fn test_auc_trained_model() {
        let mut examples = Vec::new();
        for i in 0..8 {
            let rev = if i < 4 { 1.0 } else { 20.0 };
            let bug = if i < 4 { 0.0 } else { 8.0 };
            examples.push(LabeledExample::new(
                make_fv([rev, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, bug, 0.0, 0.0]),
                i >= 4,
            ));
        }

        let params = train(&examples, 0.1, 0.001, 300);
        let features: Vec<FeatureVector> = examples.iter().map(|e| e.features.clone()).collect();
        let labels: Vec<bool> = examples.iter().map(|e| e.label).collect();

        let auc = compute_auc(&params, &features, &labels);
        assert!(auc > 0.5, "trained AUC should be > 0.5, got {}", auc);
    }

    #[test]
    fn test_evaluate_all_perfect() {
        let mut params = ModelParams::zero();
        params.weights[0] = 10.0;
        params.means = [0.0; NUM_FEATURES];
        params.stds = [1.0; NUM_FEATURES];

        let features = vec![
            make_fv([10.0; NUM_FEATURES]),
            make_fv([-10.0; NUM_FEATURES]),
        ];
        let labels = vec![true, false];

        let metrics = evaluate(&params, &features, &labels);
        assert_eq!(metrics.true_positives, 1);
        assert_eq!(metrics.true_negatives, 1);
        assert_eq!(metrics.false_positives, 0);
        assert_eq!(metrics.false_negatives, 0);
        assert_eq!(metrics.total_samples, 2);
        assert!((metrics.accuracy - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_evaluate_all_predictions_wrong() {
        let mut params = ModelParams::zero();
        params.weights[0] = -10.0;
        params.means = [0.0; NUM_FEATURES];
        params.stds = [1.0; NUM_FEATURES];

        let features = vec![
            make_fv([10.0; NUM_FEATURES]),
            make_fv([-10.0; NUM_FEATURES]),
        ];
        let labels = vec![true, false];

        let metrics = evaluate(&params, &features, &labels);
        assert_eq!(metrics.true_positives, 0);
        assert_eq!(metrics.true_negatives, 0);
        assert_eq!(metrics.false_positives, 1);
        assert_eq!(metrics.false_negatives, 1);
        assert!((metrics.accuracy - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_auc_random_classifier() {
        let params = ModelParams::zero();

        let mut features = Vec::new();
        let mut labels = Vec::new();
        for i in 0..200 {
            features.push(make_fv([0.0; NUM_FEATURES]));
            labels.push(i % 2 == 0);
        }
        let auc = compute_auc(&params, &features, &labels);
        assert!((auc - 0.5).abs() < 0.02, "expected ~0.5, got {}", auc);
    }

    #[test]
    fn test_auc_all_incorrect() {
        let mut params = ModelParams::zero();
        params.weights[0] = -100.0;
        params.means = [0.0; NUM_FEATURES];
        params.stds = [1.0; NUM_FEATURES];

        let features = vec![
            make_fv([1.0; NUM_FEATURES]),
            make_fv([0.0; NUM_FEATURES]),
        ];
        let labels = vec![true, false];

        let auc = compute_auc(&params, &features, &labels);
        assert!((auc - 0.0).abs() < 1e-10, "expected 0.0, got {}", auc);
    }
}
