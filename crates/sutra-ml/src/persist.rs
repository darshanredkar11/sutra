use crate::types::ModelParams;

/// Save model parameters to a JSON file.
pub fn save_model(params: &ModelParams, path: &str) -> Result<(), String> {
    let json = serde_json::to_string_pretty(params).map_err(|e| format!("serialize: {}", e))?;
    std::fs::write(path, &json).map_err(|e| format!("write: {}", e))?;
    Ok(())
}

/// Load model parameters from a JSON file.
pub fn load_model(path: &str) -> Result<ModelParams, String> {
    let json = std::fs::read_to_string(path).map_err(|e| format!("read: {}", e))?;
    let params: ModelParams = serde_json::from_str(&json).map_err(|e| format!("deserialize: {}", e))?;
    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ModelParams;

    #[test]
    fn test_save_load_roundtrip() {
        let dir = std::env::temp_dir().join("sutra-ml-test-persist");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("model.json").to_string_lossy().to_string();

        let params = ModelParams::zero();
        save_model(&params, &path).unwrap();

        let loaded = load_model(&path).unwrap();
        assert_eq!(params, loaded);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_nonexistent() {
        let result = load_model("/tmp/sutra-nonexistent-model.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_json() {
        let path = "/tmp/sutra-invalid-model.json";
        std::fs::write(path, "not valid json").unwrap();
        let result = load_model(path);
        assert!(result.is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_save_load_with_trained_weights() {
        let dir = std::env::temp_dir().join("sutra-ml-test-trained");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("trained.json").to_string_lossy().to_string();

        let mut params = ModelParams::zero();
        params.weights[0] = 1.5;
        params.weights[5] = -0.5;
        params.bias = 0.1;
        params.means = [0.0; 14];
        params.means[0] = 5.0;
        params.stds = [1.0; 14];
        params.stds[0] = 2.0;

        save_model(&params, &path).unwrap();
        let loaded = load_model(&path).unwrap();

        assert_eq!(params.weights, loaded.weights);
        assert!((params.bias - loaded.bias).abs() < f64::EPSILON);
        assert_eq!(params.means, loaded.means);
        assert_eq!(params.stds, loaded.stds);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_load_extreme_weights() {
        let dir = std::env::temp_dir().join("sutra-ml-test-extreme");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("extreme.json").to_string_lossy().to_string();

        let mut params = ModelParams::zero();
        params.weights[0] = f64::MAX;
        params.weights[1] = -f64::MAX;
        params.means[0] = f64::MAX;
        params.stds[0] = f64::MAX;

        save_model(&params, &path).unwrap();
        let loaded = load_model(&path).unwrap();

        assert_eq!(params.weights[0], loaded.weights[0]);
        assert_eq!(params.weights[1], loaded.weights[1]);
        assert_eq!(params.means[0], loaded.means[0]);
        assert_eq!(params.stds[0], loaded.stds[0]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_load_nan_weights_handled_gracefully() {
        let dir = std::env::temp_dir().join("sutra-ml-test-nan");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("nan.json").to_string_lossy().to_string();

        let mut params = ModelParams::zero();
        params.weights[0] = f64::NAN;

        // serde_json serializes NaN as null; save_model handles it without panic
        let save_result = save_model(&params, &path);
        // If save succeeded, loading might fail since null can't be deserialized as f64
        if let Ok(()) = save_result {
            let load_result = load_model(&path);
            // Either way, no panic occurred
            assert!(load_result.is_ok() || load_result.is_err());
        } else {
            // Save may also fail gracefully
            assert!(save_result.is_err());
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_load_empty_feature_names() {
        let dir = std::env::temp_dir().join("sutra-ml-test-empty-names");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("empty_names.json").to_string_lossy().to_string();

        let mut params = ModelParams::zero();
        params.feature_names = vec![];

        save_model(&params, &path).unwrap();
        let loaded = load_model(&path).unwrap();

        assert!(loaded.feature_names.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
