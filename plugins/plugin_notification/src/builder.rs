use async_trait::async_trait;
use miette::IntoDiagnostic;
use tokio_util::sync::CancellationToken;

use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_lib::measurement::Measurement;
use tedge_lib::notification::Notification;

use crate::config::Config;
use crate::plugin::NotificationPlugin;

pub struct NotificationPluginBuilder;

tedge_api::make_receiver_bundle!(pub struct MeasurementReceiver(Measurement));
tedge_api::make_receiver_bundle!(pub struct NotificationReceiver(Notification));

#[async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for NotificationPluginBuilder {
    fn kind_name() -> &'static str {
        "notification"
    }

    fn kind_configuration() -> Option<tedge_api::ConfigDescription> {
        Some(<Config as tedge_api::AsConfig>::as_config())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        NotificationPlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        Ok(config
            .clone()
            .try_into::<Config>()
            .map(|_| ())
            .into_diagnostic()?)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config.try_into::<Config>().into_diagnostic()?;

        let forward_addr = config.forward_to.build(plugin_dir)?;
        let notify_addr = config.notify.build(plugin_dir)?;
        Ok(NotificationPlugin::new(
            forward_addr,
            notify_addr,
            config.raise,
            config.raise_message,
        )
        .finish())
    }
}
