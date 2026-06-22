use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter, Registry};

static TRACING_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initializes structured JSON logging to stdout with an `RUST_LOG`-based filter.
/// Safe to call multiple times — subsequent calls are no-ops.
pub fn init_json_logging() {
    if TRACING_INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = Registry::default()
        .with(filter)
        .with(fmt::layer().json().with_target(true));

    let _ = tracing::subscriber::set_global_default(subscriber);
}

/// Initializes pretty-printed logging (for development).
/// Safe to call multiple times — subsequent calls are no-ops.
pub fn init_pretty_logging() {
    if TRACING_INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = Registry::default()
        .with(filter)
        .with(fmt::layer().pretty().with_target(true));

    let _ = tracing::subscriber::set_global_default(subscriber);
}

/// Initializes logging with no output (for tests).
/// Safe to call multiple times — subsequent calls are no-ops.
pub fn init_null_logging() {
    if TRACING_INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    let subscriber = Registry::default()
        .with(EnvFilter::new("off"))
        .with(fmt::layer().with_writer(std::io::sink));

    let _ = tracing::subscriber::set_global_default(subscriber);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_logging_init_once() {
        init_null_logging();
        tracing::info!("this should not panic or print");
        // second call is no-op
        init_null_logging();
    }

    #[test]
    fn test_logging_at_different_levels() {
        let _guard =
            tracing::subscriber::set_default(
                Registry::default()
                    .with(EnvFilter::new("trace"))
                    .with(fmt::layer().with_writer(std::io::sink)),
            );
        tracing::trace!("trace");
        tracing::debug!("debug");
        tracing::info!("info");
        tracing::warn!("warn");
        tracing::error!("error");
    }

    #[test]
    fn test_logging_with_span_context() {
        let _guard =
            tracing::subscriber::set_default(
                Registry::default()
                    .with(EnvFilter::new("info"))
                    .with(fmt::layer().with_writer(std::io::sink)),
            );
        let span = tracing::info_span!("test_span", engine = "mgtg", count = 42);
        let _enter = span.enter();
        tracing::info!("inside span");
    }

    #[test]
    fn test_env_filter_respected() {
        let _guard =
            tracing::subscriber::set_default(
                Registry::default()
                    .with(EnvFilter::new("error"))
                    .with(fmt::layer().with_writer(std::io::sink)),
            );
        // These should be filtered out (not printed)
        tracing::info!("should be filtered");
        tracing::warn!("should be filtered");
        // This should pass through
        tracing::error!("should pass through");
    }

    #[test]
    fn test_json_formatting_does_not_panic() {
        let _guard =
            tracing::subscriber::set_default(
                Registry::default()
                    .with(EnvFilter::new("off"))
                    .with(fmt::layer().json().with_writer(std::io::sink)),
            );
        tracing::info!(key = "value", num = 42, "json message");
    }
}
