use crate::types::{FeatureVector, ModelParams, NUM_FEATURES};

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

fn dot_product(weights: &[f64; NUM_FEATURES], features: &[f64; NUM_FEATURES]) -> f64 {
    weights.iter().zip(features.iter()).map(|(w, x)| w * x).sum()
}

fn compute_means(examples: &[[f64; NUM_FEATURES]]) -> [f64; NUM_FEATURES] {
    let n = examples.len() as f64;
    let mut means = [0.0; NUM_FEATURES];
    for ex in examples {
        for (i, val) in ex.iter().enumerate() {
            means[i] += val;
        }
    }
    for m in &mut means {
        *m /= n;
    }
    means
}

fn compute_stds(examples: &[[f64; NUM_FEATURES]], means: &[f64; NUM_FEATURES]) -> [f64; NUM_FEATURES] {
    let n = examples.len() as f64;
    let mut variances = [0.0; NUM_FEATURES];
    for ex in examples {
        for (i, val) in ex.iter().enumerate() {
            let diff = val - means[i];
            variances[i] += diff * diff;
        }
    }
    let mut stds = [0.0; NUM_FEATURES];
    for (i, v) in variances.iter().enumerate() {
        let var = v / n;
        stds[i] = if var > 1e-12 { var.sqrt() } else { 1.0 };
    }
    stds
}

pub fn standard_scale(features: &[f64; NUM_FEATURES], means: &[f64; NUM_FEATURES], stds: &[f64; NUM_FEATURES]) -> [f64; NUM_FEATURES] {
    let mut scaled = [0.0; NUM_FEATURES];
    for i in 0..NUM_FEATURES {
        scaled[i] = (features[i] - means[i]) / stds[i];
    }
    scaled
}

fn predict_probability_scaled(params: &ModelParams, scaled: &[f64; NUM_FEATURES]) -> f64 {
    let z = dot_product(&params.weights, scaled) + params.bias;
    sigmoid(z)
}

pub fn predict(params: &ModelParams, features: &FeatureVector) -> f64 {
    let scaled = standard_scale(&features.features, &params.means, &params.stds);
    predict_probability_scaled(params, &scaled)
}

pub fn predict_batch(params: &ModelParams, batch: &[FeatureVector]) -> Vec<f64> {
    batch.iter().map(|fv| predict(params, fv)).collect()
}

/// Train a logistic regression model using SGD with L2 regularization.
/// Returns the trained ModelParams with scaling statistics.
pub fn train(
    examples: &[crate::types::LabeledExample],
    learning_rate: f64,
    l2_lambda: f64,
    epochs: usize,
) -> ModelParams {
    let n = examples.len();
    if n == 0 {
        return ModelParams::zero();
    }

    let raw_features: Vec<[f64; NUM_FEATURES]> = examples
        .iter()
        .map(|ex| ex.features.features)
        .collect();

    let means = compute_means(&raw_features);
    let stds = compute_stds(&raw_features, &means);

    let mut params = ModelParams {
        weights: [0.0; NUM_FEATURES],
        bias: 0.0,
        means,
        stds,
        feature_names: crate::types::FEATURE_NAMES.iter().map(|s| s.to_string()).collect(),
    };

    for epoch in 0..epochs {
        let mut total_loss = 0.0;

        for example in examples.iter() {
            let scaled = standard_scale(&example.features.features, &params.means, &params.stds);
            let prob = predict_probability_scaled(&params, &scaled);
            let label = if example.label { 1.0 } else { 0.0 };
            let error = prob - label;

            total_loss += if example.label {
                -prob.ln().max(-1e10)
            } else {
                -(1.0 - prob).ln().max(-1e10)
            };

            let lr = learning_rate / (1.0 + 0.001 * epoch as f64);

            for (i, s) in scaled.iter().enumerate() {
                let gradient = error * s + l2_lambda * params.weights[i];
                params.weights[i] -= lr * gradient;
            }
            params.bias -= lr * error;
        }

        let _ = total_loss;
    }

    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FeatureVector, LabeledExample, NUM_FEATURES};

    proptest::proptest! {
        #[test]
        fn test_sigmoid_symmetry(z in -100.0f64..100.0) {
            let left = sigmoid(-z);
            let right = 1.0 - sigmoid(z);
            proptest::prop_assert!((left - right).abs() < 1e-10, "σ(-{}) = {}, 1-σ({}) = {}", z, left, z, right);
        }

        #[test]
        fn test_sigmoid_positive_bound(z in 40.0f64..100.0) {
            proptest::prop_assert!((sigmoid(z) - 1.0).abs() < 1e-10, "σ({}) = {}", z, sigmoid(z));
        }

        #[test]
        fn test_sigmoid_negative_bound(z in -100.0f64..-40.0) {
            proptest::prop_assert!(sigmoid(z).abs() < 1e-10, "σ({}) = {}", z, sigmoid(z));
        }
    }

    #[test]
    fn test_sigmoid_zero_exact() {
        assert_eq!(sigmoid(0.0), 0.5);
    }

    #[test]
    fn test_sigmoid_709() {
        assert!((sigmoid(709.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_sigmoid_neg_709() {
        assert!(sigmoid(-709.0).abs() < 1e-10);
    }

    fn make_example(features: [f64; NUM_FEATURES], label: bool) -> LabeledExample {
        LabeledExample::new(FeatureVector::new(features), label)
    }

    #[test]
    fn test_sigmoid_bounds() {
        assert!((sigmoid(100.0) - 1.0).abs() < 1e-10);
        assert!((sigmoid(-100.0) - 0.0).abs() < 1e-10);
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_dot_product() {
        let w = [1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let x = [3.0, 2.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert!((dot_product(&w, &x) - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_standard_scale_identity() {
        let features = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let means = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let stds = [1.0; NUM_FEATURES];
        let scaled = standard_scale(&features, &means, &stds);
        for s in &scaled {
            assert!((*s - 0.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn test_standard_scale_unit() {
        let features = [3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0];
        let means = [1.0; NUM_FEATURES];
        let stds = [2.0; NUM_FEATURES];
        let scaled = standard_scale(&features, &means, &stds);
        for s in &scaled {
            assert!((*s - 1.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn test_predict_zero_weights() {
        let params = ModelParams::zero();
        let fv = FeatureVector::new([1.0; NUM_FEATURES]);
        let prob = predict(&params, &fv);
        assert!((prob - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_predict_positive_weights() {
        let mut params = ModelParams::zero();
        params.weights[0] = 1.0;
        params.bias = 0.0;
        let fv = FeatureVector::new([2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let prob = predict(&params, &fv);
        assert!(prob > 0.5);
    }

    #[test]
    fn test_predict_negative_weights() {
        let mut params = ModelParams::zero();
        params.weights[0] = -1.0;
        let fv = FeatureVector::new([2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let prob = predict(&params, &fv);
        assert!(prob < 0.5);
    }

    #[test]
    fn test_predict_batch() {
        let params = ModelParams::zero();
        let batch = vec![
            FeatureVector::new([0.0; NUM_FEATURES]),
            FeatureVector::new([1.0; NUM_FEATURES]),
        ];
        let probs = predict_batch(&params, &batch);
        assert_eq!(probs.len(), 2);
        for p in &probs {
            assert!((*p - 0.5).abs() < 1e-10);
        }
    }

    #[test]
    fn test_train_converges_simple() {
        // Positive class: high revisions, high bug_fix_commits
        // Negative class: low revisions, no bug fixes
        let pos = make_example(
            [10.0, 3.0, 500.0, 200.0, 700.0, 2.5, 4.0, 3.0, 100.0, 30.0, 8.0, 5.0, 0.6, 2.0],
            true,
        );
        let neg = make_example(
            [1.0, 1.0, 10.0, 5.0, 15.0, 0.5, 1.0, 1.0, 10.0, 5.0, 0.0, 0.0, 0.9, 0.0],
            false,
        );

        let examples = vec![pos, neg];
        let params = train(&examples, 0.1, 0.01, 100);

        let prob_pos = predict(&params, &examples[0].features);
        let prob_neg = predict(&params, &examples[1].features);

        assert!(prob_pos > 0.5, "positive should be >0.5, got {}", prob_pos);
        assert!(prob_neg < 0.5, "negative should be <0.5, got {}", prob_neg);
    }

    #[test]
    fn test_train_empty() {
        let params = train(&[], 0.1, 0.01, 10);
        for w in &params.weights {
            assert!((*w - 0.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn test_train_more_examples() {
        let mut examples = Vec::new();
        for i in 0..5 {
            let base = (i + 1) as f64 * 2.0;
            examples.push(make_example(
                [base, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, base, 0.0, 0.0],
                i >= 3,
            ));
        }

        let params = train(&examples, 0.1, 0.001, 200);

        for (i, ex) in examples.iter().enumerate() {
            let prob = predict(&params, &ex.features);
            if i >= 3 {
                assert!(prob > 0.5, "ex[{}] should be positive, prob={}", i, prob);
            }
        }
    }

    #[test]
    fn test_compute_means_basic() {
        let data = vec![[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                        [3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]];
        let means = compute_means(&data);
        assert!((means[0] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_stds_basic() {
        let data = vec![[2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                        [2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]];
        let means = compute_means(&data);
        let stds = compute_stds(&data, &means);
        assert!((stds[0] - 1.0).abs() < 1e-10, "std of identical values should be 1.0, got {}", stds[0]);
    }

    #[test]
    fn test_standard_scale_all_zeros() {
        let features = [0.0; NUM_FEATURES];
        let means = [0.0; NUM_FEATURES];
        let stds = [1.0; NUM_FEATURES];
        let scaled = standard_scale(&features, &means, &stds);
        for s in &scaled {
            assert_eq!(*s, 0.0);
        }
    }

    #[test]
    fn test_compute_stds_constant_floors_to_one() {
        let data = vec![[5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                        [5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]];
        let means = compute_means(&data);
        let stds = compute_stds(&data, &means);
        assert_eq!(stds[0], 1.0, "constant feature should floor std to 1.0");
    }

    #[test]
    fn test_dot_product_all_zeros() {
        let w = [0.0; NUM_FEATURES];
        let x = [0.0; NUM_FEATURES];
        assert_eq!(dot_product(&w, &x), 0.0);
    }

    #[test]
    fn test_dot_product_all_ones() {
        let w = [1.0; NUM_FEATURES];
        let x = [1.0; NUM_FEATURES];
        assert_eq!(dot_product(&w, &x), NUM_FEATURES as f64);
    }

    #[test]
    fn test_train_single_example() {
        let ex = make_example([1.0; NUM_FEATURES], true);
        let params = train(&[ex], 0.1, 0.01, 50);
        for w in &params.weights {
            assert!(!w.is_nan() && !w.is_infinite(), "weight should be valid: {}", w);
        }
        assert!(!params.bias.is_nan() && !params.bias.is_infinite());
    }

    #[test]
    fn test_train_all_identical_examples() {
        let ex = make_example([5.0; NUM_FEATURES], true);
        let examples = vec![ex.clone(), ex.clone(), ex.clone()];
        let params = train(&examples, 0.1, 0.01, 50);
        for w in &params.weights {
            assert!(!w.is_nan() && !w.is_infinite(), "weight should be valid: {}", w);
        }
    }

    #[test]
    fn test_train_all_positive_labels() {
        let examples = vec![
            make_example([3.0; NUM_FEATURES], true),
            make_example([5.0; NUM_FEATURES], true),
            make_example([1.0; NUM_FEATURES], true),
        ];
        let params = train(&examples, 0.1, 0.01, 50);
        for w in &params.weights {
            assert!(!w.is_nan() && !w.is_infinite(), "weight should be valid: {}", w);
        }
    }

    #[test]
    fn test_train_all_negative_labels() {
        let examples = vec![
            make_example([3.0; NUM_FEATURES], false),
            make_example([5.0; NUM_FEATURES], false),
            make_example([1.0; NUM_FEATURES], false),
        ];
        let params = train(&examples, 0.1, 0.01, 50);
        for w in &params.weights {
            assert!(!w.is_nan() && !w.is_infinite(), "weight should be valid: {}", w);
        }
    }
}
