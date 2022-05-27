#![doc = include_str!("../README.md")]

use miette::IntoDiagnostic;
use tracing::debug;
use tracing::error;
use tracing::info;

use tedge_core::TedgeApplication;
use tedge_core::TedgeApplicationCancelSender;

pub mod cli;
pub mod config;
pub mod logging;
mod registry;

pub use crate::registry::Registry;

pub async fn run_app(args: crate::cli::Cli, registry: Registry) -> miette::Result<()> {
    match args.command {
        cli::CliCommand::Run { config } => {
            let (cancel_sender, application) =
                registry.app_builder.with_config_from_path(config).await?;
            info!("Application built");

            debug!("Verifying the configuration");
            application.verify_configurations().await?;

            debug!("Going to run the application");
            run(cancel_sender, application).await
        }
        cli::CliCommand::ValidateConfig { config } => {
            let (_, application) = registry.app_builder.with_config_from_path(config).await?;
            info!("Application built");

            debug!("Only going to validate the configuration");
            application.verify_configurations().await?;
            info!("Configuration validated");
            Ok(())
        }
        cli::CliCommand::GetPluginKinds => {
            use std::io::Write;

            let mut out = std::io::stdout();
            for name in registry.app_builder.plugin_kind_names() {
                writeln!(out, "{}", name).into_diagnostic()?;
            }
            Ok(())
        }
        cli::CliCommand::Doc { plugin_name } => {
            let mut registry = registry;
            if let Some(plugin_name) = plugin_name {
                let printer = registry
                    .doc_printers
                    .remove(&plugin_name)
                    .ok_or_else(|| miette::miette!("Plugin named '{}' not found", plugin_name))?;

                (printer)()?;
            } else {
                for printer in registry.doc_printers.into_values() {
                    printer()?;
                }
            }

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
            res
        },

        _int = tokio::signal::ctrl_c() => {
            if !cancel_sender.is_cancelled() {
                info!("Shutting down...");
                cancel_sender.cancel_app();
                tokio::select! {
                    res = &mut run_fut => res,
                    _ = tokio::signal::ctrl_c() => return kill_app(run_fut),
                }
            } else {
                return kill_app(run_fut);
            }
        },
    };

    info!("Bye");
    Ok(res?)
}
