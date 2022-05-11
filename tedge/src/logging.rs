use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;

use crate::cli::LoggingSpec;

pub(crate) fn setup_logging(spec: Option<LoggingSpec>) -> miette::Result<()> {
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

    let stdout_log = if let Some(env_filter) = env_filter {
        Some(tracing_subscriber::fmt::layer().with_filter(env_filter))
    } else {
        None
    };

    let subscriber = tracing_subscriber::registry::Registry::default()
        .with(stdout_log);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| miette::miette!("Failed to set global subscriber: {:?}", e))?;

    if spec.is_none() {
        tracing::warn!("Logging level set to WARN. Only warnings and errors will be reported.");
    }

    Ok(())
}

