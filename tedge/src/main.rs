use std::collections::HashSet;

use clap::Parser;

use tedge_api::PluginBuilder;
use tedge_core::configuration::TedgeConfiguration;
use tedge_core::TedgeApplication;
use tedge_core::TedgeApplicationCancelSender;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;

mod cli;
mod logging;

#[tokio::main]
#[tracing::instrument]
async fn main() -> anyhow::Result<()> {
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

    let configuration = tokio::fs::read_to_string(config).await?;

    let config: TedgeConfiguration = toml::de::from_str(&configuration)?;
    info!("Configuration loaded.");

    let application = TedgeApplication::builder();
    let mut plugin_kinds = HashSet::new();
    info!("Building application");

    macro_rules! register_plugin {
        ($app:ident, $cfg:tt, $pluginbuilder:expr) => {{
            cfg_if::cfg_if! {
                if #[cfg(feature = $cfg)] {
                    info!("Registering plugin builder for plugins of type {}", $pluginbuilder.kind_name());
                    if !plugin_kinds.insert($pluginbuilder.kind_name()) {
                        anyhow::bail!("Plugin kind '{}' was already registered, cannot register!", $pluginbuilder.kind_name())
                    }
                    $app.with_plugin_builder(Box::new($pluginbuilder))?
                } else {
                    trace!("Not supporting plugins of type {}", std::stringify!($pluginbuilder));
                    $app
                }
            }
        }}
    }

    let application = register_plugin!(
        application,
        "builtin_plugin_avg",
        plugin_avg::AvgPluginBuilder
    );
    let application = register_plugin!(
        application,
        "builtin_plugin_sysstat",
        plugin_sysstat::SysStatPluginBuilder
    );
    let application = register_plugin!(
        application,
        "builtin_plugin_inotify",
        plugin_inotify::InotifyPluginBuilder
    );

    let (cancel_sender, application) = application.with_config(config)?;
    info!("Application built");

    match args.command {
        cli::CliCommand::Run { .. } => {
            debug!("Going to validate the configuration and run the application");

            validate_config(&application).await?;
            info!("Configuration validated");

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
) -> anyhow::Result<()> {
    info!("Booting app now.");
    let mut run_fut = Box::pin(application.run());

    let kill_app = |fut| -> anyhow::Result<()> {
        error!("Killing application");
        drop(fut);
        anyhow::bail!("Application killed")
    };

    let res = tokio::select! {
        res = &mut run_fut => {
            res.map_err(anyhow::Error::from)
        },

        _int = tokio::signal::ctrl_c() => {
            if !cancel_sender.is_cancelled() {
                info!("Shutting down...");
                cancel_sender.cancel_app();
                tokio::select! {
                    res = &mut run_fut => res.map_err(anyhow::Error::from),
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

async fn validate_config(application: &TedgeApplication) -> anyhow::Result<()> {
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
        Err(anyhow::anyhow!("Plugin configuration error"))
    } else {
        Ok(())
    }
}
