use async_trait::async_trait;

use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_api::CancellationToken;
use tedge_lib::sm::request::List;
use tedge_lib::sm::request::Install;
use tedge_lib::sm::request::Update;
use tedge_lib::sm::request::Uninstall;

use crate::config::Config;
use crate::plugin::MqttSMPlugin;

pub struct MqttSMPluginBuilder;

tedge_api::make_receiver_bundle!(pub struct SMReceiver(List, Install, Update, Uninstall));

#[async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for MqttSMPluginBuilder {
    fn kind_name() -> &'static str {
        "mqtt_sm"
    }

    fn kind_configuration() -> Option<tedge_api::ConfigDescription> {
        Some(<Config as tedge_api::AsConfig>::as_config())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        MqttSMPlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into::<Config>()
            .map_err(crate::error::Error::ConfigParseFailed)?;
        Ok(())
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config
            .try_into::<Config>()
            .map_err(crate::error::Error::ConfigParseFailed)?;

        let target = plugin_dir.get_address_for(&config.target)?;
        let mqtt_addr = plugin_dir.get_address_for(&config.mqtt_plugin_addr)?;
        let result_topic = config.result_topic;
        Ok(MqttSMPlugin::new(target, mqtt_addr, result_topic).finish())
    }
}

