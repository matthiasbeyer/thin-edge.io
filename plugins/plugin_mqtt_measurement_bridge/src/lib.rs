use async_trait::async_trait;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_api::address::Address;
use tedge_api::address::ReplySender;
use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::Handle;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::Message;
use tedge_api::plugin::PluginExt;
use tokio_util::sync::CancellationToken;
use tracing::debug;

pub struct MqttMeasurementBridgePluginBuilder;

impl MqttMeasurementBridgePluginBuilder {
    pub fn new() -> Self {
        MqttMeasurementBridgePluginBuilder
    }
}

#[async_trait]
impl<PD> PluginBuilder<PD> for MqttMeasurementBridgePluginBuilder
where
    PD: PluginDirectory,
{
    fn kind_name() -> &'static str {
        "mqtt_measurement_bridge"
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        HandleTypes::declare_handlers_for::<(tedge_lib::measurement::Measurement,), MqttMeasurementBridgePlugin>()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .get_ref()
            .clone()
            .try_into()
            .map(|_: MqttMeasurementBridgeConfig| ())
            .map_err(|_| anyhow::anyhow!("Failed to parse mqtt-measurement-bridge configuration"))
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config
            .into_inner()
            .try_into::<MqttMeasurementBridgeConfig>()
            .map_err(|_| anyhow::anyhow!("Failed to parse mqtt configuration"))?;

        let addr = plugin_dir.get_address_for(&config.mqtt_plugin_name)?;
        Ok(MqttMeasurementBridgePlugin::new(addr, config.topic.clone()).into_untyped::<(tedge_lib::measurement::Measurement,)>())
    }
}

#[derive(serde::Deserialize, Debug)]
struct MqttMeasurementBridgeConfig {
    mqtt_plugin_name: String,
    topic: String,
}

tedge_api::make_receiver_bundle!(struct OutgoingMessageReceiver(plugin_mqtt::OutgoingMessage));

struct MqttMeasurementBridgePlugin {
    mqtt_plugin_addr: Address<OutgoingMessageReceiver>,
    topic: String,
}

impl MqttMeasurementBridgePlugin {
    fn new(mqtt_plugin_addr: Address<OutgoingMessageReceiver>, topic: String) -> Self {
        Self {
            mqtt_plugin_addr,
            topic,
        }
    }
}

#[async_trait]
impl Plugin for MqttMeasurementBridgePlugin {
    async fn setup(&mut self) -> Result<(), PluginError> {
        debug!("Setting up mqtt-measurement-bridge plugin!");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down mqtt-measurement-bridge plugin!");
        Ok(())
    }
}

#[async_trait]
impl Handle<tedge_lib::measurement::Measurement> for MqttMeasurementBridgePlugin {
    async fn handle_message(
        &self,
        message: tedge_lib::measurement::Measurement,
        _sender: ReplySender<<tedge_lib::measurement::Measurement as Message>::Reply>,
    ) -> Result<(), PluginError> {
        let outgoing = plugin_mqtt::OutgoingMessage::for_payload(&message, self.topic.clone())?;
        let _ = self.mqtt_plugin_addr.send(outgoing).await;
        Ok(())
    }
}

