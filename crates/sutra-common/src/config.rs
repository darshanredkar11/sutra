use crate::error::{SutraError, SutraResult};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

/// Loads and deserializes a TOML config file with `${VAR}` and `${VAR:default}` env substitution.
pub fn load_config<T: DeserializeOwned>(path: impl AsRef<Path>) -> SutraResult<T> {
    let raw = fs::read_to_string(path.as_ref()).map_err(|e| {
        SutraError::config(format!(
            "cannot read config `{}`: {e}",
            path.as_ref().display()
        ))
    })?;
    let substituted = substitute_env_vars(&raw);
    let config: T = toml::from_str(&substituted)?;
    Ok(config)
}

/// Substitute `${VAR}` and `${VAR:default}` patterns in a string with environment variable values.
/// Unknown variables (without default) resolve to empty string.
fn substitute_env_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
            let start = i + 2;
            if let Some(end) = chars[start..].iter().position(|&c| c == '}') {
                let var_expr: String = chars[start..start + end].iter().collect();
                let colon_pos = var_expr.find(':');
                let (var_name, default) = match colon_pos {
                    Some(pos) => (&var_expr[..pos], Some(&var_expr[pos + 1..])),
                    None => (&var_expr[..], None),
                };
                let value = env::var(var_name)
                    .ok()
                    .or_else(|| default.map(|s| s.to_owned()))
                    .unwrap_or_default();
                result.push_str(&value);
                i = start + end + 1;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// A generic, weakly-typed config that preserves the full TOML structure.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct RawConfig {
    #[serde(flatten)]
    pub inner: BTreeMap<String, toml::Value>,
}

impl RawConfig {
    pub fn load(path: impl AsRef<Path>) -> SutraResult<Self> {
        load_config(path)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> SutraResult<Self> {
        let substituted = substitute_env_vars(s);
        Ok(toml::from_str(&substituted)?)
    }

    pub fn get(&self, key: &str) -> Option<&toml::Value> {
        self.inner.get(key)
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.inner.get(key).and_then(|v| v.as_str().map(String::from))
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.inner.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.inner.get(key).and_then(|v| v.as_float())
    }

    pub fn get_table(&self, key: &str) -> Option<&toml::value::Table> {
        self.inner.get(key).and_then(|v| v.as_table())
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.inner.keys().map(String::as_str)
    }

    pub fn merge(&mut self, other: RawConfig) {
        self.inner.extend(other.inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // ── Env substitution ──────────────────────────────────────────

    #[test]
    fn test_substitute_simple_var() {
        env::set_var("SUTRA_TEST_KEY", "secret-value");
        let result = substitute_env_vars("api_key = \"${SUTRA_TEST_KEY}\"");
        assert_eq!(result, "api_key = \"secret-value\"");
    }

    #[test]
    fn test_substitute_with_default() {
        env::remove_var("SUTRA_MISSING_VAR");
        let result = substitute_env_vars("host = \"${SUTRA_MISSING_VAR:localhost}\"");
        assert_eq!(result, "host = \"localhost\"");
    }

    #[test]
    fn test_substitute_with_default_unused_when_var_exists() {
        env::set_var("SUTRA_EXISTS", "actual");
        let result = substitute_env_vars("val = \"${SUTRA_EXISTS:fallback}\"");
        assert_eq!(result, "val = \"actual\"");
    }

    #[test]
    fn test_substitute_unknown_var_without_default_empties() {
        env::remove_var("SUTRA_UNDEFINED");
        let result = substitute_env_vars("key = \"${SUTRA_UNDEFINED}\"");
        assert_eq!(result, "key = \"\"");
    }

    #[test]
    fn test_substitute_no_var_in_string() {
        let result = substitute_env_vars("plain string");
        assert_eq!(result, "plain string");
    }

    #[test]
    fn test_substitute_multiple_vars() {
        env::set_var("SUTRA_A", "hello");
        env::set_var("SUTRA_B", "world");
        let result = substitute_env_vars("${SUTRA_A} ${SUTRA_B}");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_substitute_unclosed_brace() {
        let result = substitute_env_vars("unclosed ${VAR");
        assert_eq!(result, "unclosed ${VAR");
    }

    #[test]
    fn test_substitute_empty_braces() {
        let result = substitute_env_vars("${}");
        assert_eq!(result, "");
    }

    #[test]
    fn test_substitute_with_colon_in_default() {
        env::remove_var("SUTRA_PORT");
        let result = substitute_env_vars("port = \"${SUTRA_PORT:5432}\"");
        assert_eq!(result, "port = \"5432\"");
    }

    #[test]
    fn test_substitute_multiple_occurrences_same_var() {
        env::set_var("SUTRA_TOKEN", "tok_abc");
        let result = substitute_env_vars("${SUTRA_TOKEN}:${SUTRA_TOKEN}");
        assert_eq!(result, "tok_abc:tok_abc");
    }

    #[test]
    fn test_substitute_dollar_without_brace_preserved() {
        let result = substitute_env_vars("$VAR not substituted");
        assert_eq!(result, "$VAR not substituted");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(substitute_env_vars(""), "");
    }

    #[test]
    fn test_substitute_with_url_in_default() {
        env::remove_var("SUTRA_URL");
        let result = substitute_env_vars("url=\"${SUTRA_URL:https://api.example.com:443/path}\"");
        assert_eq!(result, "url=\"https://api.example.com:443/path\"");
    }

    #[test]
    fn test_substitute_special_chars_in_value() {
        env::set_var("SUTRA_SPECIAL", "a=b&c=d");
        let result = substitute_env_vars("${SUTRA_SPECIAL}");
        assert_eq!(result, "a=b&c=d");
    }

    #[test]
    fn test_substitute_numeric_env_var() {
        env::set_var("SUTRA_NUM", "8080");
        let result = substitute_env_vars("port = ${SUTRA_NUM}");
        assert_eq!(result, "port = 8080");
    }

    // ── Config loading ────────────────────────────────────────────

    #[test]
    fn test_load_config_from_toml_string() {
        let substituted = substitute_env_vars(
            r#"
            version = "1.0"
            debug = true
            count = 42
        "#,
        );
        let val: toml::Value = toml::from_str(&substituted).unwrap();
        assert_eq!(val["version"].as_str(), Some("1.0"));
        assert_eq!(val["debug"].as_bool(), Some(true));
        assert_eq!(val["count"].as_integer(), Some(42));
    }

    #[test]
    fn test_load_config_with_env_substitution() {
        env::set_var("SUTRA_DB_HOST", "prod-db.example.com");
        let substituted = substitute_env_vars(
            r#"
            [database]
            host = "${SUTRA_DB_HOST}"
            port = 5432
        "#,
        );
        let val: toml::Value = toml::from_str(&substituted).unwrap();
        assert_eq!(val["database"]["host"].as_str(), Some("prod-db.example.com"));
        assert_eq!(val["database"]["port"].as_integer(), Some(5432));
    }

    // ── RawConfig ─────────────────────────────────────────────────

    #[test]
    fn test_raw_config_get_string() {
        let raw = RawConfig::from_str(r#"name = "sutra""#).unwrap();
        assert_eq!(raw.get_string("name"), Some("sutra".into()));
    }

    #[test]
    fn test_raw_config_get_nonexistent() {
        let raw = RawConfig::default();
        assert!(raw.get("missing").is_none());
        assert!(raw.get_string("missing").is_none());
    }

    #[test]
    fn test_raw_config_get_bool() {
        let raw = RawConfig::from_str("enabled = true").unwrap();
        assert_eq!(raw.get_bool("enabled"), Some(true));
    }

    #[test]
    fn test_raw_config_get_f64() {
        let raw = RawConfig::from_str("threshold = 0.85").unwrap();
        assert!((raw.get_f64("threshold").unwrap() - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_raw_config_keys() {
        let raw = RawConfig::from_str("a = 1\nb = 2\n").unwrap();
        let keys: Vec<&str> = raw.keys().collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"a"));
        assert!(keys.contains(&"b"));
    }

    #[test]
    fn test_raw_config_merge() {
        let mut raw1 = RawConfig::from_str(r#"a = "1""#).unwrap();
        let raw2 = RawConfig::from_str(r#"b = "2""#).unwrap();
        raw1.merge(raw2);
        assert_eq!(raw1.inner.len(), 2);
        assert_eq!(raw1.get_string("a"), Some("1".into()));
        assert_eq!(raw1.get_string("b"), Some("2".into()));
    }

    #[test]
    fn test_raw_config_get_table() {
        let raw = RawConfig::from_str(
            r#"[engine]
            enabled = true
        "#,
        )
        .unwrap();
        let table = raw.get_table("engine");
        assert!(table.is_some());
        assert_eq!(
            table.unwrap().get("enabled").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn test_raw_config_with_env_substitution() {
        env::set_var("SUTRA_LOG_LEVEL", "debug");
        let raw = RawConfig::from_str(
            r#"log_level = "${SUTRA_LOG_LEVEL}"
            "#,
        )
        .unwrap();
        assert_eq!(raw.get_string("log_level"), Some("debug".into()));
    }

    // ── File loading ──────────────────────────────────────────────

    #[test]
    fn test_load_config_file_not_found() {
        let result = load_config::<RawConfig>("/tmp/nonexistent_sutra_test_config.toml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot read config"));
    }

    #[test]
    fn test_load_config_invalid_toml() {
        let path = "/tmp/sutra_test_invalid.toml";
        fs::write(path, "invalid {{{ toml").unwrap();
        let result = load_config::<RawConfig>(path);
        assert!(result.is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_load_typed_config_from_str() {
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct AppConfig {
            version: String,
            debug: bool,
            #[serde(default)]
            port: u16,
        }

        let substituted = substitute_env_vars(
            r#"version = "2.0"
debug = true
port = 8080
"#,
        );
        let cfg: AppConfig = toml::from_str(&substituted).unwrap();
        assert_eq!(
            cfg,
            AppConfig {
                version: "2.0".into(),
                debug: true,
                port: 8080
            }
        );
    }

    #[test]
    fn test_raw_config_serde_roundtrip() {
        let raw = RawConfig::from_str(
            r#"a = 1
b = "hello"
c = true
"#,
        )
        .unwrap();
        let json = serde_json::to_string(&raw).unwrap();
        let back: RawConfig = serde_json::from_str(&json).unwrap();
        // Compare string fields (integer `a` may serialize differently)
        assert_eq!(raw.get_string("b"), back.get_string("b"));
        assert_eq!(raw.get_bool("c"), back.get_bool("c"));
    }

    // ── Edge cases ─────────────────────────────────────────────────

    #[test]
    fn test_substitute_var_with_dashes() {
        env::set_var("SUTRA-MY-KEY", "dash-value");
        let result = substitute_env_vars("${SUTRA-MY-KEY}");
        assert_eq!(result, "dash-value");
    }

    #[test]
    fn test_substitute_var_with_underscores() {
        env::set_var("SUTRA_MY_LONG_VAR_NAME", "val");
        let result = substitute_env_vars("${SUTRA_MY_LONG_VAR_NAME}");
        assert_eq!(result, "val");
    }

    #[test]
    fn test_raw_config_default_is_empty() {
        let raw = RawConfig::default();
        assert!(raw.inner.is_empty());
    }
}
