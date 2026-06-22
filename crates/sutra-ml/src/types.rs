/// 14-dimensional feature vector matching JIT defect prediction features.
pub const NUM_FEATURES: usize = 14;

pub const FEATURE_NAMES: [&str; NUM_FEATURES] = [
    "revisions",
    "distinct_committers",
    "lines_added",
    "lines_deleted",
    "total_lines_changed",
    "entropy",
    "num_directories",
    "avg_files_per_commit",
    "age_days",
    "weighted_age_days",
    "recent_commits",
    "bug_fix_commits",
    "owner_contribution",
    "minor_contributors",
];

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FeatureVector {
    pub features: [f64; NUM_FEATURES],
}

impl FeatureVector {
    pub fn new(features: [f64; NUM_FEATURES]) -> Self {
        Self { features }
    }

    pub fn from_slice(slice: &[f64]) -> Option<Self> {
        if slice.len() != NUM_FEATURES {
            return None;
        }
        let mut features = [0.0; NUM_FEATURES];
        features.copy_from_slice(slice);
        Some(Self { features })
    }
}

/// A labeled training example.
#[derive(Debug, Clone)]
pub struct LabeledExample {
    pub features: FeatureVector,
    pub label: bool,
}

impl LabeledExample {
    pub fn new(features: FeatureVector, label: bool) -> Self {
        Self { features, label }
    }
}

/// Logistic regression model parameters.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ModelParams {
    pub weights: [f64; NUM_FEATURES],
    pub bias: f64,
    pub means: [f64; NUM_FEATURES],
    pub stds: [f64; NUM_FEATURES],
    pub feature_names: Vec<String>,
}

impl ModelParams {
    pub fn zero() -> Self {
        Self {
            weights: [0.0; NUM_FEATURES],
            bias: 0.0,
            means: [0.0; NUM_FEATURES],
            stds: [1.0; NUM_FEATURES],
            feature_names: FEATURE_NAMES.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Prediction output for a single file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Prediction {
    pub file_path: String,
    pub probability: f64,
    pub predicted_class: bool,
}

impl Prediction {
    pub fn new(file_path: &str, probability: f64) -> Self {
        Self {
            file_path: file_path.to_owned(),
            probability,
            predicted_class: probability >= 0.5,
        }
    }
}

/// Evaluation metrics for a binary classifier.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EvalMetrics {
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub true_positives: usize,
    pub false_positives: usize,
    pub true_negatives: usize,
    pub false_negatives: usize,
    pub total_samples: usize,
}

impl EvalMetrics {
    pub fn new(
        tp: usize,
        fp: usize,
        tn: usize,
        fn_: usize,
    ) -> Self {
        let total = tp + fp + tn + fn_;
        let accuracy = if total > 0 { (tp + tn) as f64 / total as f64 } else { 0.0 };
        let precision = if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 };
        let recall = if tp + fn_ > 0 { tp as f64 / (tp + fn_) as f64 } else { 0.0 };
        let f1_score = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };

        Self {
            accuracy,
            precision,
            recall,
            f1_score,
            true_positives: tp,
            false_positives: fp,
            true_negatives: tn,
            false_negatives: fn_,
            total_samples: total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_vector_new() {
        let fv = FeatureVector::new([1.0; NUM_FEATURES]);
        assert_eq!(fv.features.len(), NUM_FEATURES);
    }

    #[test]
    fn test_feature_vector_from_slice() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let fv = FeatureVector::from_slice(&data).unwrap();
        assert!((fv.features[0] - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_feature_vector_wrong_length() {
        assert!(FeatureVector::from_slice(&[1.0, 2.0]).is_none());
    }

    #[test]
    fn test_labeled_example() {
        let fv = FeatureVector::new([0.0; NUM_FEATURES]);
        let ex = LabeledExample::new(fv, true);
        assert!(ex.label);
    }

    #[test]
    fn test_model_params_zero() {
        let params = ModelParams::zero();
        for w in &params.weights {
            assert!((*w - 0.0).abs() < f64::EPSILON);
        }
        assert!((params.bias - 0.0).abs() < f64::EPSILON);
        for s in &params.stds {
            assert!((*s - 1.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn test_model_params_serde_roundtrip() {
        let params = ModelParams::zero();
        let json = serde_json::to_string(&params).unwrap();
        let back: ModelParams = serde_json::from_str(&json).unwrap();
        assert_eq!(params, back);
    }

    #[test]
    fn test_prediction_new() {
        let p = Prediction::new("src/main.rs", 0.75);
        assert_eq!(p.file_path, "src/main.rs");
        assert!((p.probability - 0.75).abs() < f64::EPSILON);
        assert!(p.predicted_class);
    }

    #[test]
    fn test_prediction_below_threshold() {
        let p = Prediction::new("f.rs", 0.3);
        assert!(!p.predicted_class);
    }

    #[test]
    fn test_prediction_at_threshold() {
        let p = Prediction::new("f.rs", 0.5);
        assert!(p.predicted_class);
    }

    #[test]
    fn test_eval_metrics_perfect() {
        let m = EvalMetrics::new(10, 0, 10, 0);
        assert!((m.accuracy - 1.0).abs() < f64::EPSILON);
        assert!((m.precision - 1.0).abs() < f64::EPSILON);
        assert!((m.recall - 1.0).abs() < f64::EPSILON);
        assert!((m.f1_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_eval_metrics_worst() {
        let m = EvalMetrics::new(0, 10, 0, 10);
        assert!((m.accuracy - 0.0).abs() < f64::EPSILON);
        assert!((m.precision - 0.0).abs() < f64::EPSILON);
        assert!((m.recall - 0.0).abs() < f64::EPSILON);
        assert!((m.f1_score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_eval_metrics_partial() {
        let m = EvalMetrics::new(8, 2, 7, 3);
        assert_eq!(m.total_samples, 20);
        assert!((m.accuracy - 0.75).abs() < 1e-10);
        assert!((m.precision - 0.8).abs() < 1e-10);
        assert!((m.recall - 0.727_272_727_272_727_3).abs() < 1e-10);
        assert!((m.f1_score - 0.761_904_761_904_762).abs() < 1e-10);
    }

    #[test]
    fn test_eval_metrics_zero_denominator() {
        let m = EvalMetrics::new(0, 0, 0, 0);
        assert!((m.accuracy - 0.0).abs() < f64::EPSILON);
        assert!((m.precision - 0.0).abs() < f64::EPSILON);
        assert!((m.recall - 0.0).abs() < f64::EPSILON);
        assert!((m.f1_score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_eval_metrics_serde_roundtrip() {
        let m = EvalMetrics::new(10, 3, 8, 2);
        let json = serde_json::to_string(&m).unwrap();
        let back: EvalMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn test_prediction_serde() {
        let p = Prediction::new("f.rs", 0.95);
        let json = serde_json::to_string(&p).unwrap();
        let back: Prediction = serde_json::from_str(&json).unwrap();
        assert_eq!(p.file_path, back.file_path);
    }
}
