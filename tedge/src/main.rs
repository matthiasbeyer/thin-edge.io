use std::collections::HashSet;

use clap::Parser;
use miette::IntoDiagnostic;

use tedge_api::PluginBuilder;
use tedge_core::configuration::TedgeConfiguration;
use tedge_core::TedgeApplication;
use tedge_core::TedgeApplicationCancelSender;
use tedge_lib::measurement::Measurement;
use tracing::debug;
use tracing::error;
use tracing::info;

mod cli;
mod logging;

#[tokio::main]
#[tracing::instrument]
async fn main() -> miette::Result<()> {
    #[cfg(feature = "core_debugging")]
    {
        console_subscriber::init();
    }
    let args = crate::cli::Cli::parse();
    crate::logging::setup_logging(args.verbose, args.debug)?;
    info!("Tedge booting...");
    debug!("Tedge CLI: {:?}", args);
    let config = match args.command {
        cli::CliCommand::Run { ref config } => config,
        cli::CliCommand::ValidateConfig { ref config } => config,
    };

    let configuration = tokio::fs::read_to_string(config).await.into_diagnostic()?;

    let config: TedgeConfiguration = toml::de::from_str(&configuration).into_diagnostic()?;
    info!("Configuration loaded.");

    let application = TedgeApplication::builder();
    let mut plugin_kinds = HashSet::new();
    info!("Building application");

    macro_rules! register_plugin {
        ($app:ident, $cfg:tt, $pluginbuilder:ty, $pbinstance:expr) => {{
            cfg_if::cfg_if! {
                if #[cfg(feature = $cfg)] {
                    let kind_name: &'static str = <$pluginbuilder as PluginBuilder<tedge_core::PluginDirectory>>::kind_name();
                    info!("Registering plugin builder for plugins of type {}", kind_name);
                    if !plugin_kinds.insert(kind_name) {
                        miette::bail!("Plugin kind '{}' was already registered, cannot register!", kind_name)
                    }
                    $app.with_plugin_builder($pbinstance).into_diagnostic()?
                } else {
                    tracing::trace!("Not supporting plugins of type {}", std::stringify!($pluginbuilder));
                    $app
                }
            }
        }}
    }

    let application = register_plugin!(
        application,
        "builtin_plugin_avg",
        plugin_avg::AvgPluginBuilder,
        plugin_avg::AvgPluginBuilder
    );
    let application = register_plugin!(
        application,
        "builtin_plugin_log",
        plugin_log::LogPluginBuilder<(Measurement,)>,
        plugin_log::LogPluginBuilder::<(Measurement,)>::new()
    );
    let application = register_plugin!(
        application,
        "builtin_plugin_sysstat",
        plugin_sysstat::SysStatPluginBuilder,
        plugin_sysstat::SysStatPluginBuilder
    );
    let application = register_plugin!(
        application,
        "builtin_plugin_inotify",
        plugin_inotify::InotifyPluginBuilder,
        plugin_inotify::InotifyPluginBuilder
    );
    let application = register_plugin!(
        application,
        "builtin_plugin_httpstop",
        plugin_httpstop::HttpStopPluginBuilder,
        plugin_httpstop::HttpStopPluginBuilder
    );
    let application = register_plugin!(
        application,
        "builtin_plugin_measurement_filter",
        plugin_measurement_filter::MeasurementFilterPluginBuilder,
        plugin_measurement_filter::MeasurementFilterPluginBuilder
    );

    let (cancel_sender, application) = application.with_config(config).into_diagnostic()?;
    info!("Application built");

    match args.command {
        cli::CliCommand::Run { .. } => {
            debug!("Going to run the application");
            run(cancel_sender, application).await
        }
        cli::CliCommand::ValidateConfig { .. } => {
            debug!("Only going to validate the configuration");
            validate_config(&application).await?;
            info!("Configuration validated");
            Ok(())
        }
    }
}

async fn run(
    cancel_sender: TedgeApplicationCancelSender,
    application: TedgeApplication,
) -> miette::Result<()> {
    info!("Booting app now.");
    let mut run_fut = Box::pin(application.run());

    let kill_app = |fut| -> miette::Result<()> {
        error!("Killing application");
        drop(fut);
        miette::bail!("Application killed")
    };

    let res = tokio::select! {
        res = &mut run_fut => {
            res.into_diagnostic()
        },

        _int = tokio::signal::ctrl_c() => {
            if !cancel_sender.is_cancelled() {
                info!("Shutting down...");
                cancel_sender.cancel_app();
                tokio::select! {
                    res = &mut run_fut => res.into_diagnostic(),
                    _ = tokio::signal::ctrl_c() => kill_app(run_fut),
                }
            } else {
                kill_app(run_fut)
            }
        },
    };

    info!("Bye");
    res
}

async fn validate_config(application: &TedgeApplication) -> miette::Result<()> {
    let mut any_err = false;
    for (plugin_name, res) in application.verify_configurations().await {
        match res {
            Err(e) => {
                error!("Error in Plugin '{}' configuration: {:?}", plugin_name, e);
                any_err = true;
            }
            Ok(_) => {
                info!("Plugin '{}' configured correctly", plugin_name);
            }
        }
    }

    if any_err {
        Err(miette::miette!("Plugin configuration error"))
    } else {
        Ok(())
    }
}
