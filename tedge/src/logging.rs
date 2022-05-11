pub(crate) fn setup_logging(verbose: bool, debugging: bool) -> miette::Result<()> {
    use tracing_subscriber::filter::EnvFilter;
    use tracing_subscriber::filter::LevelFilter;

    let filter = if verbose && !debugging {
        EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into())
    } else if debugging {
        EnvFilter::from_default_env().add_directive(LevelFilter::DEBUG.into())
    } else {
        EnvFilter::from_default_env()
    };

    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(filter)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| miette::miette!("Failed to set global subscriber: {:?}", e))
}
