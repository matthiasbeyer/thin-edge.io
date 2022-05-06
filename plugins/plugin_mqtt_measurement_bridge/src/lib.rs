use async_trait::async_trait;
use miette::Context;
use miette::IntoDiagnostic;
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
use tracing::trace;

pub struct MqttMeasurementBridgePluginBuilder;

impl MqttMeasurementBridgePluginBuilder {
    pub fn new() -> Self {
        MqttMeasurementBridgePluginBuilder
    }
}

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
enum Error {
    #[error("Failed to parse configuration")]
    ConfigParseFailed(toml::de::Error),

    #[error("Failed to send message")]
    FailedToSendMessage,
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
        MqttMeasurementBridgePlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into()
            .map(|_: MqttMeasurementBridgeConfig| ())
            .map_err(Error::ConfigParseFailed)
            .map_err(tedge_api::error::PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config
            .try_into::<MqttMeasurementBridgeConfig>()
            .map_err(Error::ConfigParseFailed)?;

        let addr = plugin_dir.get_address_for(&config.mqtt_plugin_name)?;
        Ok(MqttMeasurementBridgePlugin::new(addr, config.topic.clone()).finish())
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

impl tedge_api::plugin::PluginDeclaration for MqttMeasurementBridgePlugin {
    type HandledMessages = (tedge_lib::measurement::Measurement,);
}

#[async_trait]
impl Plugin for MqttMeasurementBridgePlugin {
    async fn start(&mut self) -> Result<(), PluginError> {
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
        let json_msg = serde_json::to_string(&message)
            .into_diagnostic()
            .context("Cannot transform Measurement to JSON")?;

        let outgoing =
            plugin_mqtt::OutgoingMessage::new(json_msg.as_bytes().to_vec(), self.topic.clone());

        match self.mqtt_plugin_addr.send_and_wait(outgoing).await {
            Ok(_) => trace!("Message forwarded to MQTT plugin"),
            Err(_) => {
                trace!("Message not send");
                return Err(Error::FailedToSendMessage).into_diagnostic()
            },
        }
        Ok(())
    }
}
