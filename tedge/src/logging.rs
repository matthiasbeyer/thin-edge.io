use std::path::PathBuf;

use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;

use crate::cli::LoggingSpec;

pub fn setup_logging(
    spec: Option<LoggingSpec>,
    chrome_logging: Option<&PathBuf>,
    tracy_logging: bool,
) -> miette::Result<LogGuard> {
    let env_filter = match spec {
        None => {
            let filter = EnvFilter::builder()
                .with_default_directive(LevelFilter::WARN.into())
                .from_env_lossy();
            Some(filter)
        }

        Some(LoggingSpec::Off) | Some(LoggingSpec::False) => None,

        Some(LoggingSpec::Trace) => {
            let filter = EnvFilter::from_default_env().add_directive(LevelFilter::TRACE.into());
            Some(filter)
        }

        Some(LoggingSpec::Debug) => {
            let filter = EnvFilter::from_default_env().add_directive(LevelFilter::DEBUG.into());
            Some(filter)
        }

        Some(LoggingSpec::Info) => {
            let filter = EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());
            Some(filter)
        }

        Some(LoggingSpec::Warn) => {
            let filter = EnvFilter::from_default_env().add_directive(LevelFilter::WARN.into());
            Some(filter)
        }

        Some(LoggingSpec::Error) => {
            let filter = EnvFilter::from_default_env().add_directive(LevelFilter::ERROR.into());
            Some(filter)
        }
    };

    let (opt_chrome_layer, chrome_log_guard) = chrome_logging
        .map(|path| {
            tracing_chrome::ChromeLayerBuilder::new()
                .file(path)
                .include_args(true)
                .include_locations(true)
                .build()
        })
        .map(|(layer, guard)| (Some(layer), Some(guard)))
        .unwrap_or_default();

    let opt_tracy_layer = tracy_logging.then(|| tracing_tracy::TracyLayer::new());

    let stdout_log = if let Some(env_filter) = env_filter {
        Some(tracing_subscriber::fmt::layer().with_filter(env_filter))
    } else {
        None
    };

    let subscriber = tracing_subscriber::registry::Registry::default()
        .with(opt_chrome_layer)
        .with(opt_tracy_layer)
        .with(stdout_log);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| miette::miette!("Failed to set global subscriber: {:?}", e))?;

    if spec.is_none() {
        tracing::warn!("Logging level set to WARN. Only warnings and errors will be reported.");
    }

    Ok(LogGuard { chrome_log_guard })
}

/// Helper type for keeping the tracing_chrome::FlushGuard alive if we trace with tracing_chrome
///
/// Only there for its Drop implementation
pub struct LogGuard {
    #[allow(dead_code)]
    chrome_log_guard: Option<tracing_chrome::FlushGuard>,
}
