use std::collections::HashMap;
use std::collections::HashSet;

use clap::Parser;
use miette::IntoDiagnostic;

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
    doc_printers: HashMap<String, Box<dyn FnOnce()>>,
}

macro_rules! register_plugin {
    ($registry:ident, $cfg:tt, $pluginbuilder:ty, $pbinstance:expr) => {{
        cfg_if::cfg_if! {
            if #[cfg(feature = $cfg)] {
                let kind_name: &'static str = <$pluginbuilder as PluginBuilder<tedge_core::PluginDirectory>>::kind_name();
                info!("Registering plugin builder for plugins of type {}", kind_name);
                let mut registry = $registry;
                if !registry.plugin_kinds.insert(kind_name.to_string()) {
                    miette::bail!("Plugin kind '{}' was already registered, cannot register!", kind_name)
                }

                Registry {
                    app_builder: registry.app_builder.with_plugin_builder($pbinstance)?,
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
    #[cfg(feature = "core_debugging")]
    {
        console_subscriber::init();
    }
    let args = crate::cli::Cli::parse();
    crate::logging::setup_logging(args.verbose, args.debug)?;
    info!("Tedge booting...");
    debug!("Tedge CLI: {:?}", args);

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
            let (cancel_sender, application) = registry.app_builder.with_config_from_path(config).await?;
            info!("Application built");

            debug!("Verifying the configuration");
            validate_config(&application).await?;

            debug!("Going to run the application");
            run(cancel_sender, application).await
        }
        cli::CliCommand::ValidateConfig { config } => {
            let (_, application) = registry.app_builder.with_config_from_path(config).await?;
            info!("Application built");

            debug!("Only going to validate the configuration");
            validate_config(&application).await?;
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
