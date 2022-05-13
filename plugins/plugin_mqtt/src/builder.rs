use async_trait::async_trait;
use tedge_api::PluginError;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tokio_util::sync::CancellationToken;
use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;

use crate::config::MqttConfig;
use crate::plugin::MqttPlugin;

pub struct MqttPluginBuilder;

impl MqttPluginBuilder {
    pub fn new() -> Self {
        MqttPluginBuilder
    }
}

#[async_trait]
impl<PD> PluginBuilder<PD> for MqttPluginBuilder
where
    PD: PluginDirectory,
{
    fn kind_name() -> &'static str {
        "mqtt"
    }

    fn kind_configuration() -> Option<tedge_api::ConfigDescription> {
        Some(<MqttConfig as tedge_api::AsConfig>::as_config())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        MqttPlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into()
            .map(|_: MqttConfig| ())
            .map_err(crate::error::Error::ConfigParseFailed)
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config
            .try_into::<MqttConfig>()
            .map_err(crate::error::Error::ConfigParseFailed)?;

        let addr = plugin_dir.get_address_for(&config.target)?;
        Ok(MqttPlugin::new(config, addr).finish())
    }
}

