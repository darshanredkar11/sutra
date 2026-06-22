use thiserror::Error;

#[derive(Error, Debug)]
pub enum SutraError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Engine error (`{engine}`): {message}")]
    EngineError {
        engine: &'static str,
        message: String,
    },

    #[error("Engine timeout (`{engine}`) after {duration_ms}ms")]
    EngineTimeout {
        engine: &'static str,
        duration_ms: u64,
    },

    #[error("Schema validation error: {0}")]
    Schema(#[from] sutra_schema::SchemaError),

    #[error("{0}")]
    Other(String),
}

impl SutraError {
    pub fn config(msg: impl Into<String>) -> Self {
        SutraError::Config(msg.into())
    }

    pub fn engine(engine: &'static str, message: impl Into<String>) -> Self {
        SutraError::EngineError {
            engine,
            message: message.into(),
        }
    }

    pub fn timeout(engine: &'static str, duration_ms: u64) -> Self {
        SutraError::EngineTimeout { engine, duration_ms }
    }

    pub fn other(msg: impl Into<String>) -> Self {
        SutraError::Other(msg.into())
    }
}

pub type SutraResult<T> = Result<T, SutraError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error() {
        let err = SutraError::config("missing field");
        assert_eq!(err.to_string(), "Configuration error: missing field");
    }

    #[test]
    fn test_engine_error() {
        let err = SutraError::engine("mgtg", "parse failed");
        assert_eq!(err.to_string(), "Engine error (`mgtg`): parse failed");
    }

    #[test]
    fn test_timeout_error() {
        let err = SutraError::timeout("ml", 30000);
        assert_eq!(err.to_string(), "Engine timeout (`ml`) after 30000ms");
    }

    #[test]
    fn test_other_error() {
        let err = SutraError::other("something went wrong");
        assert_eq!(err.to_string(), "something went wrong");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: SutraError = io_err.into();
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_error_from_schema() {
        let schema_err = sutra_schema::SchemaError::RiskOutOfBounds(1.5);
        let err: SutraError = schema_err.into();
        assert!(err.to_string().contains("risk score out of bounds"));
    }

    #[test]
    fn test_sutra_result_type() {
        fn ok_fn() -> SutraResult<i32> {
            Ok(42)
        }
        fn err_fn() -> SutraResult<i32> {
            Err(SutraError::other("fail"))
        }
        assert_eq!(ok_fn().unwrap(), 42);
        assert!(err_fn().is_err());
    }
}
