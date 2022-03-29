use std::marker::PhantomData;

use async_trait::async_trait;

use tedge_api::address::ReplySender;
use tedge_api::plugin::Handle;
use tedge_api::plugin::Message;
use tedge_api::plugin::MessageBundle;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tracing::debug;

use crate::config::MqttConfig;

pub struct MqttPlugin<MB> {
    _pd: PhantomData<MB>,
    config: MqttConfig,

    client: Option<rumqttc::AsyncClient>,
}

impl<MB> MqttPlugin<MB>
where
    MB: MessageBundle + Sync + Send + 'static,
{
    pub(crate) fn new(config: MqttConfig) -> Self {
        Self {
            _pd: PhantomData,
            config,

            client: None,
        }
    }
}

#[async_trait]
impl<MB> Plugin for MqttPlugin<MB>
where
    MB: MessageBundle + Sync + Send + 'static,
{
    async fn setup(&mut self) -> Result<(), PluginError> {
        debug!("Setting up mqtt plugin!");
        let mqtt_options = mqtt_options(&self.config);
        let (mqtt_client, _event_loop) =
            rumqttc::AsyncClient::new(mqtt_options, self.config.queue_capacity);
        self.client = Some(mqtt_client);
        unimplemented!()
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down mqtt plugin!");
        if let Some(client) = self.client.take() {
            client
                .disconnect()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to disconnect MQTT client: {:?}", e))?;
        }
        unimplemented!()
    }
}

fn mqtt_options(config: &MqttConfig) -> rumqttc::MqttOptions {
    let id = config.session_name.as_ref().cloned().unwrap_or_else(|| {
        std::iter::repeat_with(fastrand::lowercase)
            .take(10)
            .collect()
    });

    let mut mqtt_options = rumqttc::MqttOptions::new(id, &config.host, config.port);
    mqtt_options.set_clean_session(config.clean_session);
    mqtt_options.set_max_packet_size(config.max_packet_size, config.max_packet_size);

    mqtt_options
}

#[async_trait]
impl<M, MB> Handle<M> for MqttPlugin<MB>
where
    M: Message + serde::Serialize + std::fmt::Debug,
    M::Reply: serde::de::DeserializeOwned,
    MB: MessageBundle + Sync + Send + 'static,
{
    async fn handle_message(
        &self,
        message: M,
        _sender: ReplySender<M::Reply>,
    ) -> Result<(), PluginError> {
        if let Some(client) = self.client.as_ref() {
            let payload = serde_json::to_vec(&message).map_err(|e| {
                anyhow::anyhow!("Failed to serialize message '{:?}': {}", message, e)
            })?;

            client
                .publish(
                    &self.config.topic,
                    self.config.qos.into(),
                    self.config.retain,
                    payload,
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send message '{:?}': {}", message, e))
                .map_err(PluginError::from)
        } else {
            Err(anyhow::anyhow!("No client, cannot send messages"))?
        }
    }
}
