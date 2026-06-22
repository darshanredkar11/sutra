use sutra_schema::v1::Severity;

/// Configuration for connecting to an LLM backend (Ollama).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LLMConfig {
    pub model: String,
    pub ollama_url: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub timeout_secs: u64,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            model: "llama3.2".into(),
            ollama_url: "http://localhost:11434".into(),
            temperature: 0.1,
            max_tokens: 512,
            timeout_secs: 30,
        }
    }
}

/// Structured result from LLM validation of a single finding.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    pub finding_id: String,
    pub is_valid: bool,
    pub confidence: f64,
    pub explanation: String,
    pub suggested_fix: Option<String>,
}

impl ValidationResult {
    pub fn new(
        finding_id: &str,
        is_valid: bool,
        confidence: f64,
        explanation: &str,
    ) -> Self {
        Self {
            finding_id: finding_id.to_owned(),
            is_valid,
            confidence,
            explanation: explanation.to_owned(),
            suggested_fix: None,
        }
    }

    pub fn with_fix(mut self, fix: &str) -> Self {
        self.suggested_fix = Some(fix.to_owned());
        self
    }
}

/// Ollama API request body for /api/generate
#[derive(Debug, Clone, serde::Serialize)]
pub struct OllamaGenerateRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OllamaOptions>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OllamaOptions {
    pub temperature: f64,
    #[serde(rename = "num_predict")]
    pub num_predict: u32,
}

/// Ollama API response body from /api/generate
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OllamaGenerateResponse {
    pub response: String,
    pub done: bool,
}

impl LLMConfig {
    pub fn build_validation_prompt(
        &self,
        file_path: &str,
        line: u32,
        message: &str,
        severity: Severity,
    ) -> String {
        format!(
            r#"You are a code review assistant validating static analysis findings.
Analyze this finding and respond with ONLY a JSON object (no markdown, no extra text):

{{
  "is_valid": true or false,
  "confidence": 0.0 to 1.0,
  "explanation": "brief reason",
  "suggested_fix": "fix suggestion or null"
}}

Finding details:
- File: {file_path}
- Line: {line}
- Message: {message}
- Severity: {severity:?}

Is this a genuine code defect? Respond with JSON only."#,
            file_path = file_path,
            line = line,
            message = message,
            severity = severity,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_config_default() {
        let cfg = LLMConfig::default();
        assert_eq!(cfg.model, "llama3.2");
        assert_eq!(cfg.ollama_url, "http://localhost:11434");
        assert!((cfg.temperature - 0.1).abs() < f64::EPSILON);
        assert_eq!(cfg.max_tokens, 512);
    }

    #[test]
    fn test_llm_config_serde_roundtrip() {
        let cfg = LLMConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: LLMConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn test_validation_result_new() {
        let vr = ValidationResult::new("MGTG-001", true, 0.95, "Legitimate resource leak");
        assert_eq!(vr.finding_id, "MGTG-001");
        assert!(vr.is_valid);
        assert!(vr.suggested_fix.is_none());
    }

    #[test]
    fn test_validation_result_with_fix() {
        let vr = ValidationResult::new("MGTG-001", true, 0.9, "Real bug")
            .with_fix("Add close() call");
        assert_eq!(vr.suggested_fix, Some("Add close() call".into()));
    }

    #[test]
    fn test_validation_result_serde() {
        let vr = ValidationResult::new("F001", false, 0.3, "False positive");
        let json = serde_json::to_string(&vr).unwrap();
        let back: ValidationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(vr, back);
    }

    #[test]
    fn test_build_validation_prompt() {
        let cfg = LLMConfig::default();
        let prompt = cfg.build_validation_prompt("src/main.rs", 42, "Resource leak", Severity::Error);
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("42"));
        assert!(prompt.contains("Resource leak"));
        assert!(prompt.contains("Error"));
        assert!(prompt.contains("is_valid"));
        assert!(prompt.contains("suggested_fix"));
    }

    #[test]
    fn test_ollama_generate_request_serde() {
        let req = OllamaGenerateRequest {
            model: "llama3.2".into(),
            prompt: "test".into(),
            stream: false,
            options: Some(OllamaOptions {
                temperature: 0.1,
                num_predict: 256,
            }),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("llama3.2"));
        assert!(json.contains("temperature"));
        assert!(json.contains("num_predict"));
    }

    #[test]
    fn test_llm_config_empty_model() {
        let cfg = LLMConfig {
            model: "".into(),
            ..LLMConfig::default()
        };
        assert_eq!(cfg.model, "");
        let json = serde_json::to_string(&cfg).unwrap();
        let back: LLMConfig = serde_json::from_str(&json).unwrap();
        assert!(back.model.is_empty());
    }

    #[test]
    fn test_llm_config_empty_url() {
        let cfg = LLMConfig {
            ollama_url: "".into(),
            ..LLMConfig::default()
        };
        assert_eq!(cfg.ollama_url, "");
        let json = serde_json::to_string(&cfg).unwrap();
        let back: LLMConfig = serde_json::from_str(&json).unwrap();
        assert!(back.ollama_url.is_empty());
    }

    #[test]
    fn test_llm_config_temperature_zero() {
        let cfg = LLMConfig {
            temperature: 0.0,
            ..LLMConfig::default()
        };
        assert!((cfg.temperature - 0.0).abs() < f64::EPSILON);
        let json = serde_json::to_string(&cfg).unwrap();
        let back: LLMConfig = serde_json::from_str(&json).unwrap();
        assert!((back.temperature - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_llm_config_temperature_two() {
        let cfg = LLMConfig {
            temperature: 2.0,
            ..LLMConfig::default()
        };
        assert!((cfg.temperature - 2.0).abs() < f64::EPSILON);
        let json = serde_json::to_string(&cfg).unwrap();
        let back: LLMConfig = serde_json::from_str(&json).unwrap();
        assert!((back.temperature - 2.0).abs() < f64::EPSILON);
    }
}
