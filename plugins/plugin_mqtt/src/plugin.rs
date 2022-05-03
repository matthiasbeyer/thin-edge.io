use async_trait::async_trait;

use miette::Context;
use miette::IntoDiagnostic;
use tedge_api::address::Address;
use tedge_api::address::ReplySender;
use tedge_api::plugin::Handle;
use tedge_api::plugin::Message;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tracing::debug;
use tracing::error;

use crate::config::MqttConfig;
use crate::message::MqttMessageReceiver;
use crate::message::OutgoingMessage;

pub struct MqttPlugin {
    config: MqttConfig,

    client: Option<paho_mqtt::AsyncClient>,
    stopper: Option<tedge_lib::mainloop::MainloopStopper>,
    target_addr: Address<MqttMessageReceiver>,
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
        let _ = tokio::spawn(mainloop.run(mqtt_main));

        let connect_opts = Some({
            paho_mqtt::connect_options::ConnectOptionsBuilder::new()
                .connect_timeout(std::time::Duration::from_secs(10))
                .server_uris(&[&self.config.host])
                .finalize()
        });
        client
            .connect(connect_opts)
            .await
            .map_err(|e| miette::miette!("Failed connecting the client: {}", e))?;

        self.config.subscriptions.iter().for_each(|s| {
            let _ = client.subscribe(s.topic.clone(), s.qos.into());
        });

        self.client = Some(client);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down mqtt plugin!");

        // try to shutdown internal mainloop
        let stop_err = if let Some(stopper) = self.stopper.take() {
            stopper
                .stop()
                .map_err(|e| miette::miette!("Failed to stop MQTT mainloop: {:?}", e))
        } else {
            Ok(())
        };

        // try to shutdown mqtt client
        let client_shutdown_err = if let Some(client) = self.client.take() {
            client
                .disconnect(None)
                .await
                .map_err(|e| miette::miette!("Failed to disconnect MQTT client: {:?}", e))
                .map(|_| ())
        } else {
            Ok(())
        };

        crate::error::MqttShutdownError::build_for(client_shutdown_err, stop_err).into_diagnostic()
    }
}

struct State {
    client: paho_mqtt::AsyncClient,
    stream: paho_mqtt::AsyncReceiver<Option<paho_mqtt::message::Message>>,
    target_addr: Address<MqttMessageReceiver>,
}

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
                            if let Err(e) = state.client.reconnect().await {
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

async fn handle_incoming_message(
    state: &State,
    message: paho_mqtt::Message,
) -> Result<(), PluginError> {
    debug!("Received MQTT message");
    let incoming = crate::message::IncomingMessage {
        payload: message.payload().to_vec(),
        qos: message.qos(),
        retain: message.retained(),
        topic: message.topic().to_string(),
    };

    debug!("Sending incoming message to target plugin");
    let _ = state.target_addr.send_and_wait(incoming).await;
    Ok(())
}

#[async_trait]
impl Handle<OutgoingMessage> for MqttPlugin {
    async fn handle_message(
        &self,
        message: OutgoingMessage,
        _sender: ReplySender<<OutgoingMessage as Message>::Reply>,
    ) -> Result<(), PluginError> {
        debug!("Received outgoing message");
        if let Some(client) = self.client.as_ref() {
            let msg = paho_mqtt::Message::new(&message.topic, message.payload, message.qos.into());
            debug!("Publishing message on {}", message.topic);
            client
                .publish(msg)
                .await
                .into_diagnostic()
                .context("Failed to publish message")?;
            debug!("Publishing message succeeded");
            Ok(())
        } else {
            Err(miette::miette!("No client, cannot send messages"))?
        }
    }
}
