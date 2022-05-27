use clap::Parser;
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

    let registry = tedge_cli::Registry::new();
    info!("Building application");

    let registry = {
        cfg_table::cfg_table! {
            [not(feature = "mqtt")] => {
                tedge_cli::register_plugin!(
                    in registry:            registry,
                    if feature enabled:     "builtin_plugin_log",
                    with builder type:      plugin_log::LogPluginBuilder<(Measurement, Notification)>,
                    with builder instance:  plugin_log::LogPluginBuilder::<(Measurement, Notification)>::new()
                )
            },

            [feature = "mqtt"] => {
                tedge_cli::register_plugin!(
                    in registry:            registry,
                    if feature enabled:     "builtin_plugin_log",
                    with builder type:      plugin_log::LogPluginBuilder<(Measurement, Notification, plugin_mqtt::IncomingMessage)>,
                    with builder instance:  plugin_log::LogPluginBuilder::<(Measurement, Notification, plugin_mqtt::IncomingMessage)>::new()
                )
            },
        }
    };

    let registry = tedge_cli::register_plugin!(
        in registry:            registry,
        if feature enabled:     "builtin_plugin_avg",
        with builder type:      plugin_avg::AvgPluginBuilder,
        with builder instance:  plugin_avg::AvgPluginBuilder
    );
    let registry = tedge_cli::register_plugin!(
        in registry:            registry,
        if feature enabled:     "builtin_plugin_sysstat",
        with builder type:      plugin_sysstat::SysStatPluginBuilder,
        with builder instance:  plugin_sysstat::SysStatPluginBuilder
    );
    let registry = tedge_cli::register_plugin!(
        in registry:            registry,
        if feature enabled:     "builtin_plugin_inotify",
        with builder type:      plugin_inotify::InotifyPluginBuilder,
        with builder instance:  plugin_inotify::InotifyPluginBuilder
    );
    let registry = tedge_cli::register_plugin!(
        in registry:            registry,
        if feature enabled:     "builtin_plugin_httpstop",
        with builder type:      plugin_httpstop::HttpStopPluginBuilder,
        with builder instance:  plugin_httpstop::HttpStopPluginBuilder
    );
    let registry = tedge_cli::register_plugin!(
        in registry:            registry,
        if feature enabled:     "builtin_plugin_measurement_filter",
        with builder type:      plugin_measurement_filter::MeasurementFilterPluginBuilder,
        with builder instance:  plugin_measurement_filter::MeasurementFilterPluginBuilder
    );
    let registry = tedge_cli::register_plugin!(
        in registry:            registry,
        if feature enabled:     "mqtt",
        with builder type:      plugin_mqtt::MqttPluginBuilder,
        with builder instance:  plugin_mqtt::MqttPluginBuilder::new()
    );
    let registry = tedge_cli::register_plugin!(
        in registry:            registry,
        if feature enabled:     "mqtt",
        with builder type:      plugin_mqtt_measurement_bridge::MqttMeasurementBridgePluginBuilder,
        with builder instance:  plugin_mqtt_measurement_bridge::MqttMeasurementBridgePluginBuilder::new()
    );
    let registry = tedge_cli::register_plugin!(
        in registry:            registry,
        if feature enabled:     "builtin_plugin_notification",
        with builder type:      plugin_notification::NotificationPluginBuilder,
        with builder instance:  plugin_notification::NotificationPluginBuilder
    );

    tedge_cli::run_app(args, registry).await
}
