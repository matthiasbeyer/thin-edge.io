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

    let registry = tedge_cli::Registry::default();
    info!("Building application");

    let registry = {
        cfg_table::cfg_table! {
            [not(feature = "mqtt")] => {
                tedge_cli::register_plugin!(
                    if feature "builtin_plugin_log" is enabled then
                    register on registry
                    builder of type plugin_log::LogPluginBuilder<(Measurement, Notification)>,
                    with instance {
                        plugin_log::LogPluginBuilder::<(Measurement, Notification)>::new()
                    }
                )
            },

            [feature = "mqtt"] => {
                tedge_cli::register_plugin!(
                    if feature "builtin_plugin_log" is enabled then
                    register on registry
                    builder of type plugin_log::LogPluginBuilder<(Measurement, Notification, plugin_mqtt::IncomingMessage)>,
                    with instance {
                        plugin_log::LogPluginBuilder::<(Measurement, Notification, plugin_mqtt::IncomingMessage)>::new()
                    }
                )
            },
        }
    };

    let registry = tedge_cli::register_plugin!(
        if feature "builtin_plugin_avg" is enabled then
        register on registry
        builder of type plugin_avg::AvgPluginBuilder,
        with instance {
            plugin_avg::AvgPluginBuilder
        }
    );
    let registry = tedge_cli::register_plugin!(
        if feature "builtin_plugin_sysstat" is enabled then
        register on registry
        builder of type plugin_sysstat::SysStatPluginBuilder,
        with instance {
            plugin_sysstat::SysStatPluginBuilder
        }
    );
    let registry = tedge_cli::register_plugin!(
        if feature "builtin_plugin_inotify" is enabled then
        register on registry
        builder of type plugin_inotify::InotifyPluginBuilder,
        with instance {
            plugin_inotify::InotifyPluginBuilder
        }
    );
    let registry = tedge_cli::register_plugin!(
        if feature "builtin_plugin_httpstop" is enabled then
        register on registry
        builder of type plugin_httpstop::HttpStopPluginBuilder,
        with instance {
            plugin_httpstop::HttpStopPluginBuilder
        }
    );
    let registry = tedge_cli::register_plugin!(
        if feature "builtin_plugin_measurement_filter" is enabled then
        register on registry
        builder of type plugin_measurement_filter::MeasurementFilterPluginBuilder,
        with instance {
            plugin_measurement_filter::MeasurementFilterPluginBuilder
        }
    );
    let registry = tedge_cli::register_plugin!(
        if feature "mqtt" is enabled then
        register on registry
        builder of type plugin_mqtt::MqttPluginBuilder,
        with instance {
            plugin_mqtt::MqttPluginBuilder::new()
        }
    );
    let registry = tedge_cli::register_plugin!(
        if feature "mqtt" is enabled then
        register on registry
        builder of type plugin_mqtt_measurement_bridge::MqttMeasurementBridgePluginBuilder,
        with instance {
            plugin_mqtt_measurement_bridge::MqttMeasurementBridgePluginBuilder::new()
        }
    );
    let registry = tedge_cli::register_plugin!(
        if feature "builtin_plugin_notification" is enabled then
        register on registry
        builder of type plugin_notification::NotificationPluginBuilder,
        with instance {
            plugin_notification::NotificationPluginBuilder
        }
    );

    tedge_cli::run_app(args, registry).await
}
