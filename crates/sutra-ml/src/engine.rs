use std::time::Instant;

use sutra_common::engine::AnalysisEngine;
use sutra_common::error::{SutraError, SutraResult};
use sutra_schema::v1::{
    AnalysisResult, AnalyzeRequest, Recommendation,
};

use crate::model::{predict, train};
use crate::persist::{load_model, save_model};
use crate::types::{FeatureVector, LabeledExample, ModelParams, Prediction};

pub struct MlEngine {
    model_path: Option<String>,
    model: Option<ModelParams>,
    learning_rate: f64,
    l2_lambda: f64,
    epochs: usize,
}

impl MlEngine {
    pub fn new() -> Self {
        Self {
            model_path: None,
            model: None,
            learning_rate: 0.1,
            l2_lambda: 0.001,
            epochs: 500,
        }
    }

    pub fn with_model_path(mut self, path: &str) -> Self {
        self.model_path = Some(path.to_string());
        if let Ok(model) = load_model(path) {
            self.model = Some(model);
        }
        self
    }

    pub fn with_hyperparams(mut self, lr: f64, l2: f64, epochs: usize) -> Self {
        self.learning_rate = lr;
        self.l2_lambda = l2;
        self.epochs = epochs;
        self
    }

    pub fn train_on_labeled(
        &mut self,
        examples: &[LabeledExample],
    ) -> SutraResult<ModelParams> {
        let params = train(examples, self.learning_rate, self.l2_lambda, self.epochs);

        if let Some(path) = &self.model_path {
            save_model(&params, path)
                .map_err(|e| SutraError::engine("ml", format!("save model: {}", e)))?;
        }

        self.model = Some(params.clone());
        Ok(params)
    }

    pub fn predict_features(&self, features: &[FeatureVector]) -> SutraResult<Vec<Prediction>> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| SutraError::engine("ml", "model not trained or loaded"))?;

        Ok(features
            .iter()
            .enumerate()
            .map(|(i, fv)| {
                let prob = predict(model, fv);
                Prediction::new(&format!("file_{}", i), prob)
            })
            .collect())
    }
}

impl Default for MlEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisEngine for MlEngine {
    fn name(&self) -> &'static str {
        "ml"
    }

    fn analyze(&self, _request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
        let start = Instant::now();

        let _model = self
            .model
            .as_ref()
            .ok_or_else(|| SutraError::engine("ml", "no trained model available. train with `train_on_labeled` first or load from path"))?;

        // Without access to process analysis features in the AnalyzeRequest,
        // we return a basic result indicating no ML findings.
        // In practice, the orchestrator would pass JIT features through.
        Ok(AnalysisResult {
            request_id: _request.request_id.clone(),
            commit_hash: _request.commit_hash.clone(),
            overall_risk: 0.0,
            findings: vec![],
            recommendations: vec![Recommendation::new(
                "ML engine available but requires feature input from ProcessEngine. Train with labeled data first.",
                0.5,
            )],
            metrics: None,
            processing_time_ms: start.elapsed().as_secs_f64() * 1000.0,
            blocked_merge: false,
        })
    }
}

/// Helper: convert JIT features from sutra-process to ML feature vectors.
pub fn jit_features_to_fvs(
    jit_features: &[sutra_schema::v1::FeatureMap],
) -> Vec<FeatureVector> {
    jit_features
        .iter()
        .map(|fm| {
            let mut features = [0.0; 14];
            let names = [
                "revisions", "distinct_committers", "lines_added", "lines_deleted",
                "total_lines_changed", "entropy", "num_directories", "avg_files_per_commit",
                "age_days", "weighted_age_days", "recent_commits", "bug_fix_commits",
                "owner_contribution", "minor_contributors",
            ];

            for (i, name) in names.iter().enumerate() {
                features[i] = fm.get(*name).copied().unwrap_or(0.0);
            }

            FeatureVector::new(features)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval;
    use crate::types::{FeatureVector, LabeledExample};

    fn make_example(data: [f64; 14], label: bool) -> LabeledExample {
        LabeledExample::new(FeatureVector::new(data), label)
    }

    #[test]
    fn test_engine_name() {
        let engine = MlEngine::new();
        assert_eq!(engine.name(), "ml");
    }

    #[test]
    fn test_engine_analyze_without_model() {
        let engine = MlEngine::new();
        let req = AnalyzeRequest::new("/repo", "abc");
        let result = engine.analyze(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no trained model"));
    }

    #[test]
    fn test_engine_default() {
        let engine = MlEngine::default();
        assert_eq!(engine.name(), "ml");
    }

    #[test]
    fn test_train_and_predict() {
        let mut engine = MlEngine::new()
            .with_hyperparams(0.1, 0.001, 300);

        let examples = vec![
            make_example([10.0, 3.0, 500.0, 200.0, 700.0, 2.5, 4.0, 3.0, 100.0, 30.0, 8.0, 5.0, 0.6, 2.0], true),
            make_example([1.0, 1.0, 10.0, 5.0, 15.0, 0.5, 1.0, 1.0, 10.0, 5.0, 0.0, 0.0, 0.9, 0.0], false),
            make_example([8.0, 2.0, 300.0, 100.0, 400.0, 2.0, 3.0, 2.5, 80.0, 25.0, 5.0, 3.0, 0.7, 1.0], true),
            make_example([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], false),
        ];

        engine.train_on_labeled(&examples).unwrap();

        let probs = engine.predict_features(&examples.iter().map(|e| e.features.clone()).collect::<Vec<_>>()).unwrap();
        assert_eq!(probs.len(), 4);
    }

    #[test]
    fn test_predict_without_model() {
        let engine = MlEngine::new();
        let features = vec![FeatureVector::new([0.0; 14])];
        let result = engine.predict_features(&features);
        assert!(result.is_err());
    }

    #[test]
    fn test_jit_features_to_fvs() {
        use std::collections::HashMap;
        let mut fm = HashMap::new();
        fm.insert("revisions".into(), 10.0);
        fm.insert("entropy".into(), 2.5);
        fm.insert("unknown".into(), 99.0);

        let fvs = jit_features_to_fvs(&[fm]);
        assert_eq!(fvs.len(), 1);
        assert!((fvs[0].features[0] - 10.0).abs() < f64::EPSILON);
        assert!((fvs[0].features[5] - 2.5).abs() < f64::EPSILON);
        assert!((fvs[0].features[1] - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_jit_features_empty() {
        assert!(jit_features_to_fvs(&[]).is_empty());
    }

    #[test]
    fn test_with_hyperparams() {
        let engine = MlEngine::new()
            .with_hyperparams(0.01, 0.1, 100);
        assert!((engine.learning_rate - 0.01).abs() < f64::EPSILON);
        assert!((engine.l2_lambda - 0.1).abs() < f64::EPSILON);
        assert_eq!(engine.epochs, 100);
    }

    #[test]
    fn test_train_evaluate_metrics() {
        let mut engine = MlEngine::new().with_hyperparams(0.1, 0.001, 500);

        let mut examples = Vec::new();
        for i in 0..8 {
            let rev = if i < 4 { 1.0 } else { 20.0 };
            let bug = if i < 4 { 0.0 } else { 10.0 };
            examples.push(make_example(
                [rev, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, bug, 0.0, 0.0],
                i >= 4,
            ));
        }

        let params = engine.train_on_labeled(&examples).unwrap();

        let features: Vec<FeatureVector> = examples.iter().map(|e| e.features.clone()).collect();
        let labels: Vec<bool> = examples.iter().map(|e| e.label).collect();

        let metrics = eval::evaluate(&params, &features, &labels);
        assert!(metrics.accuracy >= 0.75, "accuracy should be >= 0.75, got {}", metrics.accuracy);
        assert!(metrics.f1_score > 0.0, "F1 should be > 0.0, got {}", metrics.f1_score);
    }
}
