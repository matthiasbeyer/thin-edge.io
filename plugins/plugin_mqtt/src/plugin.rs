use async_trait::async_trait;

use miette::IntoDiagnostic;
use tedge_api::address::Address;
use tedge_api::address::ReplySenderFor;
use tedge_api::plugin::Handle;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tracing::debug;
use tracing::error;
use tracing::Instrument;

use crate::config::MqttConfig;
use crate::message::MqttMessageReceiver;
use crate::message::OutgoingMessage;

pub struct MqttPlugin {
    config: MqttConfig,

    client: Option<paho_mqtt::AsyncClient>,
    stopper: Option<tedge_lib::mainloop::MainloopStopper>,
    target_addr: Address<MqttMessageReceiver>,
}

impl std::fmt::Debug for MqttPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttPlugin")
            .field("config", &self.config)
            .field("stopper", &self.stopper)
            .field("target_addr", &self.target_addr)
            .finish_non_exhaustive()
    }
}

impl MqttPlugin {
    pub(crate) fn new(config: MqttConfig, target_addr: Address<MqttMessageReceiver>) -> Self {
        Self {
            config,

            client: None,
            stopper: None,
            target_addr,
        }
    }
}

impl tedge_api::plugin::PluginDeclaration for MqttPlugin {
    type HandledMessages = (OutgoingMessage,);
}

#[async_trait]
impl Plugin for MqttPlugin {
    #[tracing::instrument(name = "plugin.mqtt.start", skip(self))]
    async fn start(&mut self) -> Result<(), PluginError> {
        debug!("Setting up mqtt plugin!");
        let mut client = paho_mqtt::AsyncClient::new(self.config.host.clone())
            .map_err(|e| miette::miette!("Error creating the client: {}", e))?;

        let state = State {
            client: client.clone(),         // cheap, as this is internally just an Arc<_>
            stream: client.get_stream(100), // TODO: Specify buffer size in config
            target_addr: self.target_addr.clone(),
        };

        debug!("Starting mqtt plugin mainloop!");
        let (stopper, mainloop) = tedge_lib::mainloop::Mainloop::detach(state);
        self.stopper = Some(stopper);
        let _ = tokio::spawn(
            mainloop
                .run(mqtt_main)
                .instrument(tracing::debug_span!("plugin.mqtt.mainloop")),
        );

        let connect_opts = Some({
            paho_mqtt::connect_options::ConnectOptionsBuilder::new()
                .connect_timeout(std::time::Duration::from_secs(10))
                .server_uris(&[&self.config.host])
                .finalize()
        });
        client
            .connect(connect_opts)
            .instrument(tracing::debug_span!("plugin.mqtt.client.connect"))
            .await
            .map_err(|e| miette::miette!("Failed connecting the client: {}", e))?;

        self.config.subscriptions.iter().for_each(|s| {
            let _ = client.subscribe(s.topic.clone(), s.qos.into());
        });

        self.client = Some(client);
        Ok(())
    }

    #[tracing::instrument(name = "plugin.mqtt.shutdown", skip(self))]
    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down mqtt plugin!");

        // try to shutdown internal mainloop
        if let Some(stopper) = self.stopper.take() {
            stopper
                .stop()
                .map_err(|_| crate::error::Error::FailedToStopMqttMainloop)?;
        }

        // try to shutdown mqtt client
        if let Some(client) = self.client.take() {
            client
                .disconnect(None)
                .instrument(tracing::debug_span!("plugin.mqtt.client.disconnect"))
                .await
                .map_err(|e| crate::error::Error::FailedToDisconnectMqttClient(e))?;
        }

        Ok(())
    }
}

struct State {
    client: paho_mqtt::AsyncClient,
    stream: paho_mqtt::AsyncReceiver<Option<paho_mqtt::message::Message>>,
    target_addr: Address<MqttMessageReceiver>,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("target_addr", &self.target_addr)
            .finish_non_exhaustive()
    }
}

#[tracing::instrument(name = "plugin.mqtt.main", skip_all)]
async fn mqtt_main(
    mut state: State,
    stopper: tedge_api::CancellationToken,
) -> Result<(), PluginError> {
    use futures::stream::StreamExt;

    loop {
        tokio::select! {
            next_event = state.stream.next() => {
                match next_event {
                    Some(Some(message)) => {
                        match handle_incoming_message(&state, message).await {
                            Err(e) => error!("Handling message failed: {:?}", e),
                            Ok(_) => debug!("Handling message succeded"),
                        }
                    }

                    Some(None) => {
                        // client disconnected, connect again
                        debug!("Client disconnected, reconnecting...");
                        let op = || async {
                            let reconnect_res = state.client
                                .reconnect()
                                .instrument(tracing::debug_span!("plugin.mqtt.main.client.reconnect"))
                                .await;

                            if let Err(e) = reconnect_res {
                                error!("Reconnecting failed: {}", e);
                                Err(backoff::Error::transient(()))
                            } else {
                                Ok(())
                            }
                        };

                        let backoff = backoff::ExponentialBackoffBuilder::new()
                            .with_initial_interval(std::time::Duration::from_millis(100))
                            .with_multiplier(2.0)
                            .with_max_interval(std::time::Duration::from_secs(5))
                            .build();

                        let _ = backoff::future::retry(backoff, op).await;
                    }

                    None => {
                        // What now?
                    }
                }
            }

            _cancel = stopper.cancelled() => {
                break
            }
        }
    }

    Ok(())
}

#[tracing::instrument(name = "plugin.mqtt.main.handle_incoming_message", skip(state))]
async fn handle_incoming_message(
    state: &State,
    message: paho_mqtt::Message,
) -> Result<(), PluginError> {
    debug!(?message, "Received MQTT message");
    let incoming = crate::message::IncomingMessage {
        payload: message.payload().to_vec(),
        qos: message.qos(),
        retain: message.retained(),
        topic: message.topic().to_string(),
    };

    debug!("Sending incoming message to target plugin");
    let _ = state
        .target_addr
        .send_and_wait(incoming)
        .instrument(tracing::debug_span!(
            "plugin.mqtt.main.handle_incoming_message.send_and_wait"
        ))
        .await;
    Ok(())
}

#[async_trait]
impl Handle<OutgoingMessage> for MqttPlugin {
    #[tracing::instrument(name = "plugin.mqtt.handle_message", level = "trace")]
    async fn handle_message(
        &self,
        message: OutgoingMessage,
        _sender: ReplySenderFor<OutgoingMessage>,
    ) -> Result<(), PluginError> {
        debug!("Received outgoing message");
        if let Some(client) = self.client.as_ref() {
            let msg = paho_mqtt::Message::new(&message.topic, message.payload, message.qos.into());
            debug!(?message.topic, "Publishing message");
            client
                .publish(msg)
                .instrument(tracing::debug_span!("plugin.mqtt.handle_message.publish"))
                .await
                .map_err(crate::error::Error::FailedToPublish)
                .into_diagnostic()?;

            debug!("Publishing message succeeded");
        } else {
            Err(crate::error::Error::NoClient).into_diagnostic()?;
        };

        Ok(())
    }
}
