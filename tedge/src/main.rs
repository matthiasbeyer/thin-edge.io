use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;

use clap::Parser;
use miette::IntoDiagnostic;
use pretty::Arena;
use tracing::debug;
use tracing::info;

use tedge_cli::Registry;
use tedge_lib::measurement::Measurement;
use tedge_lib::notification::Notification;

#[tokio::main]
#[tracing::instrument]
async fn main() -> miette::Result<()> {
    let args = tedge_cli::cli::Cli::parse();
    let _guard = tedge_cli::logging::setup_logging(
        args.logging,
        args.chrome_logging.as_ref(),
        args.tracy_logging,
    )?;
    info!("Tedge booting...");
    debug!(?args, "Tedge CLI");

    let registry = tedge_cli::Registry {
        app_builder: tedge_core::TedgeApplication::builder(),
        plugin_kinds: HashSet::new(),
        doc_printers: HashMap::new(),
    };
    info!("Building application");

    let registry = {
        cfg_table::cfg_table! {
            [not(feature = "mqtt")] => tedge_cli::register_plugin!(
                registry,
                "builtin_plugin_log",
                plugin_log::LogPluginBuilder<(Measurement, Notification)>,
                plugin_log::LogPluginBuilder::<(Measurement, Notification)>::new()
            ),

            [feature = "mqtt"] => tedge_cli::register_plugin!(
                registry,
                "builtin_plugin_log",
                plugin_log::LogPluginBuilder<(Measurement, Notification, plugin_mqtt::IncomingMessage)>,
                plugin_log::LogPluginBuilder::<(Measurement, Notification, plugin_mqtt::IncomingMessage)>::new()
            ),
        }
    };

    let registry = tedge_cli::register_plugin!(
        registry,
        "builtin_plugin_avg",
        plugin_avg::AvgPluginBuilder,
        plugin_avg::AvgPluginBuilder
    );
    let registry = tedge_cli::register_plugin!(
        registry,
        "builtin_plugin_sysstat",
        plugin_sysstat::SysStatPluginBuilder,
        plugin_sysstat::SysStatPluginBuilder
    );
    let registry = tedge_cli::register_plugin!(
        registry,
        "builtin_plugin_inotify",
        plugin_inotify::InotifyPluginBuilder,
        plugin_inotify::InotifyPluginBuilder
    );
    let registry = tedge_cli::register_plugin!(
        registry,
        "builtin_plugin_httpstop",
        plugin_httpstop::HttpStopPluginBuilder,
        plugin_httpstop::HttpStopPluginBuilder
    );
    let registry = tedge_cli::register_plugin!(
        registry,
        "builtin_plugin_measurement_filter",
        plugin_measurement_filter::MeasurementFilterPluginBuilder,
        plugin_measurement_filter::MeasurementFilterPluginBuilder
    );
    let registry = tedge_cli::register_plugin!(
        registry,
        "mqtt",
        plugin_mqtt::MqttPluginBuilder,
        plugin_mqtt::MqttPluginBuilder::new()
    );
    let registry = tedge_cli::register_plugin!(
        registry,
        "mqtt",
        plugin_mqtt_measurement_bridge::MqttMeasurementBridgePluginBuilder,
        plugin_mqtt_measurement_bridge::MqttMeasurementBridgePluginBuilder::new()
    );
    let registry = tedge_cli::register_plugin!(
        registry,
        "builtin_plugin_notification",
        plugin_notification::NotificationPluginBuilder,
        plugin_notification::NotificationPluginBuilder
    );

    tedge_cli::run_app(args, registry).await
}
