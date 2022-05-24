#![doc = include_str!("../README.md")]

use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;

use clap::Parser;
use miette::IntoDiagnostic;

use pretty::Arena;
use tedge_api::PluginBuilder;
use tedge_core::TedgeApplication;
use tedge_core::TedgeApplicationBuilder;
use tedge_core::TedgeApplicationCancelSender;
use tedge_lib::measurement::Measurement;
use tedge_lib::notification::Notification;
use tracing::debug;
use tracing::error;
use tracing::info;

mod cli;
mod config;
mod logging;

/// Helper type for registering PluginBuilder instances and doc-printing functions
struct Registry {
    app_builder: TedgeApplicationBuilder,
    plugin_kinds: HashSet<String>,
    doc_printers: HashMap<String, Box<dyn FnOnce() -> Result<(), miette::Error>>>,
}

macro_rules! register_plugin {
    ($registry:ident, $cfg:tt, $pluginbuilder:ty, $pbinstance:expr) => {{
        cfg_if::cfg_if! {
            if #[cfg(feature = $cfg)] {
                let kind_name: &'static str = <$pluginbuilder as PluginBuilder<tedge_core::PluginDirectory>>::kind_name();
                info!(%kind_name, "Registering plugin builder");
                let mut registry = $registry;
                if !registry.plugin_kinds.insert(kind_name.to_string()) {
                    miette::bail!("Plugin kind '{}' was already registered, cannot register!", kind_name)
                }

                let kind_name_str = kind_name.to_string();
                registry.doc_printers.insert(kind_name.to_string(), Box::new(move || {
                    let mut stdout = std::io::stdout();
                    if let Some(config_desc) = <$pluginbuilder as PluginBuilder<tedge_core::PluginDirectory>>::kind_configuration() {
                        let terminal_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
                        let arena = Arena::new();

                        let rendered_doc = crate::config::as_terminal_doc(&config_desc, &arena);

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

#[tokio::main]
#[tracing::instrument]
async fn main() -> miette::Result<()> {
    let args = crate::cli::Cli::parse();
    let _guard = crate::logging::setup_logging(
        args.logging,
        args.chrome_logging.as_ref(),
        args.tracy_logging,
    )?;
    info!("Tedge booting...");
    debug!(?args, "Tedge CLI");

    let registry = Registry {
        app_builder: TedgeApplication::builder(),
        plugin_kinds: HashSet::new(),
        doc_printers: HashMap::new(),
    };
    info!("Building application");

    let registry = {
        cfg_table::cfg_table! {
            [not(feature = "mqtt")] => register_plugin!(
                registry,
                "builtin_plugin_log",
                plugin_log::LogPluginBuilder<(Measurement, Notification)>,
                plugin_log::LogPluginBuilder::<(Measurement, Notification)>::new()
            ),

            [feature = "mqtt"] => register_plugin!(
                registry,
                "builtin_plugin_log",
                plugin_log::LogPluginBuilder<(Measurement, Notification, plugin_mqtt::IncomingMessage)>,
                plugin_log::LogPluginBuilder::<(Measurement, Notification, plugin_mqtt::IncomingMessage)>::new()
            ),
        }
    };

    let registry = register_plugin!(
        registry,
        "builtin_plugin_avg",
        plugin_avg::AvgPluginBuilder,
        plugin_avg::AvgPluginBuilder
    );
    let registry = register_plugin!(
        registry,
        "builtin_plugin_sysstat",
        plugin_sysstat::SysStatPluginBuilder,
        plugin_sysstat::SysStatPluginBuilder
    );
    let registry = register_plugin!(
        registry,
        "builtin_plugin_inotify",
        plugin_inotify::InotifyPluginBuilder,
        plugin_inotify::InotifyPluginBuilder
    );
    let registry = register_plugin!(
        registry,
        "builtin_plugin_httpstop",
        plugin_httpstop::HttpStopPluginBuilder,
        plugin_httpstop::HttpStopPluginBuilder
    );
    let registry = register_plugin!(
        registry,
        "builtin_plugin_measurement_filter",
        plugin_measurement_filter::MeasurementFilterPluginBuilder,
        plugin_measurement_filter::MeasurementFilterPluginBuilder
    );
    let registry = register_plugin!(
        registry,
        "mqtt",
        plugin_mqtt::MqttPluginBuilder,
        plugin_mqtt::MqttPluginBuilder::new()
    );
    let registry = register_plugin!(
        registry,
        "mqtt",
        plugin_mqtt_measurement_bridge::MqttMeasurementBridgePluginBuilder,
        plugin_mqtt_measurement_bridge::MqttMeasurementBridgePluginBuilder::new()
    );
    let registry = register_plugin!(
        registry,
        "builtin_plugin_notification",
        plugin_notification::NotificationPluginBuilder,
        plugin_notification::NotificationPluginBuilder
    );

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
