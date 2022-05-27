#![doc = include_str!("../README.md")]

use std::collections::HashMap;
use std::collections::HashSet;

use miette::IntoDiagnostic;
use tracing::debug;
use tracing::error;
use tracing::info;

use tedge_core::TedgeApplication;
use tedge_core::TedgeApplicationBuilder;
use tedge_core::TedgeApplicationCancelSender;

pub mod cli;
pub mod config;
pub mod logging;

/// Helper type for registering PluginBuilder instances and doc-printing functions
pub struct Registry {
    pub app_builder: TedgeApplicationBuilder,
    pub plugin_kinds: HashSet<String>,
    pub doc_printers: HashMap<String, Box<dyn FnOnce() -> Result<(), miette::Error>>>,
}

#[macro_export]
macro_rules! register_plugin {
    ($registry:ident, $cfg:tt, $pluginbuilder:ty, $pbinstance:expr) => {{
        cfg_if::cfg_if! {
            if #[cfg(feature = $cfg)] {
                let kind_name: &'static str = <$pluginbuilder as tedge_api::PluginBuilder<tedge_core::PluginDirectory>>::kind_name();
                info!(%kind_name, "Registering plugin builder");
                let mut registry = $registry;
                if !registry.plugin_kinds.insert(kind_name.to_string()) {
                    miette::bail!("Plugin kind '{}' was already registered, cannot register!", kind_name)
                }

                let kind_name_str = kind_name.to_string();
                registry.doc_printers.insert(kind_name.to_string(), Box::new(move || {
                    let mut stdout = std::io::stdout();
                    if let Some(config_desc) = <$pluginbuilder as tedge_api::PluginBuilder<tedge_core::PluginDirectory>>::kind_configuration() {
                        let terminal_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
                        let arena = Arena::new();

                        let rendered_doc = tedge_cli::config::as_terminal_doc(&config_desc, &arena);

                        let mut output = String::new();
                        rendered_doc.render_fmt(terminal_width, &mut output).into_diagnostic()?;

                        writeln!(stdout, " ----- Documentation for plugin '{}'", kind_name_str)
                                .into_diagnostic()?;

                        writeln!(stdout, "{}", output).into_diagnostic()?;
                    } else {
                        let msg = format!(" Documentation for plugin '{}' is unavailable", kind_name);
                        writeln!(stdout, "{}", nu_ansi_term::Color::Red.bold().paint(msg))
                            .into_diagnostic()?;
                    }
                    Ok(())
                }));

                Registry {
                    app_builder: registry.app_builder.with_plugin_builder($pbinstance),
                    plugin_kinds: registry.plugin_kinds,
                    doc_printers: registry.doc_printers,
                }
            } else {
                tracing::trace!("Not supporting plugins of type {}", std::stringify!($pluginbuilder));
                $registry
            }
        }
    }}
}

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
