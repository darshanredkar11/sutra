use std::time::Duration;

use sutra_common::error::SutraResult;

use crate::types::{LLMConfig, OllamaGenerateRequest, OllamaGenerateResponse, OllamaOptions, ValidationResult};

/// Send a single finding to Ollama for validation and parse the JSON response.
pub fn validate_finding(
    config: &LLMConfig,
    finding_id: &str,
    file_path: &str,
    line: u32,
    message: &str,
    severity: sutra_schema::v1::Severity,
) -> SutraResult<ValidationResult> {
    let prompt = config.build_validation_prompt(file_path, line, message, severity);

    let request_body = OllamaGenerateRequest {
        model: config.model.clone(),
        prompt,
        stream: false,
        options: Some(OllamaOptions {
            temperature: config.temperature,
            num_predict: config.max_tokens,
        }),
    };

    let url = format!("{}/api/generate", config.ollama_url.trim_end_matches('/'));

    let agent = ureq::Agent::new_with_config(
        ureq::config::Config::builder()
            .timeout_global(Some(Duration::from_secs(config.timeout_secs)))
            .build(),
    );

    let http_response = agent
        .post(&url)
        .send_json(&request_body)
        .map_err(|e| {
            sutra_common::error::SutraError::config(format!("ollama request failed: {}", e))
        })?;

    let response_body = http_response
        .into_body()
        .read_to_string()
        .map_err(|e| {
            sutra_common::error::SutraError::config(format!("ollama body read failed: {}", e))
        })?;

    let response: OllamaGenerateResponse = serde_json::from_str(&response_body)
        .map_err(|e| {
            sutra_common::error::SutraError::config(format!("ollama response parse failed: {}", e))
        })?;

    parse_llm_response(finding_id, &response.response)
}

/// Parse the LLM's JSON response into a ValidationResult.
fn parse_llm_response(
    finding_id: &str,
    response: &str,
) -> SutraResult<ValidationResult> {
    // Clean the response: remove markdown code fences, trim whitespace
    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Try direct parsing first
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(cleaned) {
        let is_valid = parsed
            .get("is_valid")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let confidence = parsed
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);

        let explanation = parsed
            .get("explanation")
            .and_then(|v| v.as_str())
            .unwrap_or("No explanation provided")
            .to_string();

        let suggested_fix = parsed
            .get("suggested_fix")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && s != &"null")
            .map(String::from);

        let mut result = ValidationResult::new(finding_id, is_valid, confidence, &explanation);
        if let Some(fix) = suggested_fix {
            result = result.with_fix(&fix);
        }
        return Ok(result);
    }

    Ok(ValidationResult::new(
        finding_id,
        true,
        0.5,
        "Could not parse LLM response as JSON",
    ))
}

/// Validate multiple findings with optional batching.
pub fn validate_findings_batch(
    config: &LLMConfig,
    findings: &[sutra_schema::v1::Finding],
) -> Vec<SutraResult<ValidationResult>> {
    findings
        .iter()
        .map(|f| {
            validate_finding(
                config,
                &f.id,
                &f.file_path,
                f.line,
                &f.message,
                f.severity,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LLMConfig;

    #[test]
    fn test_parse_llm_response_valid_json() {
        let response = r#"{"is_valid": true, "confidence": 0.95, "explanation": "Real bug", "suggested_fix": "Add null check"}"#;
        let result = parse_llm_response("F001", response).unwrap();
        assert!(result.is_valid);
        assert!((result.confidence - 0.95).abs() < 1e-10);
        assert_eq!(result.explanation, "Real bug");
        assert_eq!(result.suggested_fix, Some("Add null check".into()));
    }

    #[test]
    fn test_parse_llm_response_invalid_finding() {
        let response = r#"{"is_valid": false, "confidence": 0.2, "explanation": "False positive, variable is already closed", "suggested_fix": null}"#;
        let result = parse_llm_response("F001", response).unwrap();
        assert!(!result.is_valid);
        assert!((result.confidence - 0.2).abs() < 1e-10);
        assert!(result.suggested_fix.is_none());
    }

    #[test]
    fn test_parse_llm_response_with_code_fence() {
        let response = "```json\n{\"is_valid\": true, \"confidence\": 0.8, \"explanation\": \"Legitimate issue\"}\n```";
        let result = parse_llm_response("F001", response).unwrap();
        assert!(result.is_valid);
        assert!((result.confidence - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_parse_llm_response_garbage() {
        let response = "I'm sorry, I cannot analyze this code.";
        let result = parse_llm_response("F001", response).unwrap();
        assert!(result.is_valid);
        assert!((result.confidence - 0.5).abs() < 1e-10);
        assert!(result.explanation.contains("Could not parse"));
    }

    #[test]
    fn test_parse_llm_response_empty() {
        let result = parse_llm_response("F001", "").unwrap();
        assert!(result.is_valid);
    }

    #[test]
    fn test_parse_llm_response_partial_fields() {
        let response = r#"{"is_valid": false}"#;
        let result = parse_llm_response("F001", response).unwrap();
        assert!(!result.is_valid);
        assert!((result.confidence - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_validate_finding_connect_error() {
        let config = LLMConfig {
            ollama_url: "http://127.0.0.1:1".into(),
            timeout_secs: 1,
            ..LLMConfig::default()
        };
        let result = validate_finding(&config, "F001", "f.rs", 1, "test", sutra_schema::v1::Severity::Error);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_findings_batch_empty() {
        let config = LLMConfig::default();
        let results = validate_findings_batch(&config, &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_llm_response_with_markdown_fence() {
        let response = "```\n{\"is_valid\": true, \"confidence\": 0.9, \"explanation\": \"fix it\"}\n```";
        let result = parse_llm_response("F001", response).unwrap();
        assert!(result.is_valid);
        assert!((result.confidence - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_parse_llm_response_confidence_clamping() {
        let response = r#"{"is_valid": true, "confidence": 2.5, "explanation": "ok"}"#;
        let result = parse_llm_response("F001", response).unwrap();
        assert!((result.confidence - 1.0).abs() < 1e-10);

        let response = r#"{"is_valid": true, "confidence": -1.0, "explanation": "ok"}"#;
        let result = parse_llm_response("F001", response).unwrap();
        assert!((result.confidence - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_llm_response_with_extra_fields() {
        let response = r#"{"is_valid": false, "confidence": 0.3, "explanation": "FP", "extra_field": "ignored", "another_extra": 123}"#;
        let result = parse_llm_response("F001", response).unwrap();
        assert!(!result.is_valid);
        assert!((result.confidence - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_parse_llm_response_confidence_exactly_two() {
        let response = r#"{"is_valid": true, "confidence": 2.0, "explanation": "test"}"#;
        let result = parse_llm_response("F001", response).unwrap();
        assert!((result.confidence - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_llm_response_confidence_exactly_negative_one() {
        let response = r#"{"is_valid": true, "confidence": -1.0, "explanation": "test"}"#;
        let result = parse_llm_response("F001", response).unwrap();
        assert!((result.confidence - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_llm_response_suggested_fix_null_string() {
        let response = r#"{"is_valid": true, "confidence": 0.8, "explanation": "test", "suggested_fix": "null"}"#;
        let result = parse_llm_response("F001", response).unwrap();
        assert!(result.suggested_fix.is_none());
    }

    #[test]
    fn test_parse_llm_response_suggested_fix_empty_string() {
        let response = r#"{"is_valid": true, "confidence": 0.8, "explanation": "test", "suggested_fix": ""}"#;
        let result = parse_llm_response("F001", response).unwrap();
        assert!(result.suggested_fix.is_none());
    }
}
