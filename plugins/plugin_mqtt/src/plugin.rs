use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;

use tedge_api::address::Address;
use tedge_api::address::ReplySender;
use tedge_api::plugin::Handle;
use tedge_api::plugin::Message;
use tedge_api::plugin::MessageBundle;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tracing::debug;
use tracing::error;

use crate::config::MqttConfig;
use crate::message::MqttMessageReceiver;

pub struct MqttPlugin<MB> {
    _pd: PhantomData<MB>,
    config: MqttConfig,

    client: Option<rumqttc::AsyncClient>,
    stopper: Option<tedge_lib::mainloop::MainloopStopper>,
    target_addr: Address<MqttMessageReceiver>,
}

impl<MB> MqttPlugin<MB>
where
    MB: MessageBundle + Sync + Send + 'static,
{
    pub(crate) fn new(config: MqttConfig, target_addr: Address<MqttMessageReceiver>) -> Self {
        Self {
            _pd: PhantomData,
            config,

            client: None,
            stopper: None,
            target_addr,
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
        let (mqtt_client, event_loop) =
            rumqttc::AsyncClient::new(mqtt_options, self.config.queue_capacity);
        self.client = Some(mqtt_client);

        let state = State {
            event_loop,
            target_addr: self.target_addr.clone(),
        };

        let (stopper, mainloop) = tedge_lib::mainloop::Mainloop::detach(state);
        self.stopper = Some(stopper);
        let _ = tokio::spawn(mainloop.run(mqtt_main));

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down mqtt plugin!");

        // try to shutdown internal mainloop
        let stop_err = if let Some(stopper) = self.stopper.take() {
            stopper
                .stop()
                .map_err(|e| anyhow::anyhow!("Failed to stop MQTT mainloop: {:?}", e))
        } else {
            Ok(())
        };

        // try to shutdown mqtt client
        let client_shutdown_err = if let Some(client) = self.client.take() {
            client
                .disconnect()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to disconnect MQTT client: {:?}", e))
        } else {
            Ok(())
        };

        match (client_shutdown_err, stop_err) {
            (Err(e), _) => Err(e).map_err(PluginError::from),
            (_, Err(e)) => Err(e).map_err(PluginError::from),
            _ => Ok(()),
        }
    }
}

struct State {
    event_loop: rumqttc::EventLoop,
    target_addr: Address<MqttMessageReceiver>,
}

async fn mqtt_main(
    mut state: State,
    mut stopper: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), PluginError> {
    use rumqttc::Event;
    use rumqttc::Incoming;
    use rumqttc::Outgoing;
    use rumqttc::Packet;

    loop {
        tokio::select! {
            next_event = state.event_loop.poll() => {
                match next_event {
                    Ok(Event::Incoming(Packet::Publish(msg))) => {
                        let message = serde_json::from_slice(&msg.payload)
                            .map_err(|e| anyhow::anyhow!("Could not deserialize message '{:?}': {}", msg, e))?;

                        let message = crate::message::IncomingMessage {
                            dup: msg.dup,
                            payload: message,
                            pkid: msg.pkid,
                            qos: msg.qos,
                            retain: msg.retain,
                            topic: msg.topic,
                        };

                        let _ = state.target_addr.send(message).await;
                    }

                    Ok(Event::Incoming(Incoming::Disconnect)) | Ok(Event::Outgoing(Outgoing::Disconnect)) => {
                        // The connection has been closed
                        break;
                    }

                    Err(e) => {
                        error!("Error received: {:?}", e);
                        // what to do on error?
                        unimplemented!()
                    }

                    _ => {
                        // ignore other events
                    }
                }
            }

            _cancel = &mut stopper => {
                break
            }
        }
    }

    Ok(())
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
