use async_trait::async_trait;
use tedge_api::{
    address::{MessageReceiver, ReplySenderFor},
    message::StopCore,
    plugin::{Handle, PluginExt},
    Plugin, PluginError,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace, warn, Instrument};

use crate::errors::Result;

/// Helper type in the crate implementation for handling the actual message passing
///
/// This actually implements [`tedge_api::Plugin`] as well, as this is the representation of the
/// core that can receive messages. These messages are no different than the messages sent between
/// [`tedge_api::Plugin`] implementations, so this must implement that trait as well.
pub struct CoreTask {
    cancellation_token: CancellationToken,
    receiver: MessageReceiver,
    internal_sender: tokio::sync::mpsc::Sender<CoreInternalMessage>,
    internal_receiver: tokio::sync::mpsc::Receiver<CoreInternalMessage>,
}

impl std::fmt::Debug for CoreTask {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("CoreTask").finish_non_exhaustive()
    }
}

impl CoreTask {
    pub fn new(cancellation_token: CancellationToken, receiver: MessageReceiver) -> Self {
        let (internal_sender, internal_receiver) = tokio::sync::mpsc::channel(10);
        Self {
            cancellation_token,
            receiver,
            internal_sender,
            internal_receiver,
        }
    }

    #[tracing::instrument]
    pub(crate) async fn run(mut self) -> Result<()> {
        let running_core = RunningCore {
            sender: self.internal_sender,
        };
        let built_plugin = running_core.finish();
        let mut receiver_closed = false;

        loop {
            tokio::select! {
                _cancel = self.cancellation_token.cancelled() => {
                    debug!("Cancelled main loop");
                    break;
                },

                internal_message = self.internal_receiver.recv() => {
                    trace!("Received message");
                    match internal_message {
                        msg @ None | msg @ Some(CoreInternalMessage::Stop) => {
                            if msg.is_none() {
                                warn!("Internal core communication stopped");
                            }
                            debug!("Cancelling cancellation token to stop plugins");
                            self.cancellation_token.cancel();
                            debug!("Stopping core");
                            break;
                        }
                    }
                },

                next_message = self.receiver.recv(), if !receiver_closed => {
                    trace!("Received message");
                    match next_message {
                        Some(msg) => {
                            let handle_msg_res = built_plugin.handle_message(msg)
                                .instrument(tracing::trace_span!("core.core_task.handle_message"))
                                .await;

                            match handle_msg_res {
                                Ok(_) => debug!("Core handled message successfully"),
                                Err(e) => warn!("Core failed to handle message: {:?}", e),
                            }
                        },

                        None => {
                            receiver_closed = true;
                            debug!("Receiver closed for Core");
                        },
                    }
                }
            }
        }

        Ok(())
    }
}

struct RunningCore {
    sender: tokio::sync::mpsc::Sender<CoreInternalMessage>,
}

impl tedge_api::plugin::PluginDeclaration for RunningCore {
    type HandledMessages = (StopCore,);
}

enum CoreInternalMessage {
    Stop,
}

#[async_trait]
impl Plugin for RunningCore {
    async fn start(&mut self) -> std::result::Result<(), PluginError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> std::result::Result<(), PluginError> {
        Ok(())
    }
}

#[async_trait]
impl Handle<StopCore> for RunningCore {
    async fn handle_message(
        &self,
        _message: StopCore,
        _sender: ReplySenderFor<StopCore>,
    ) -> std::result::Result<(), PluginError> {
        trace!("Received StopCore message, going to stop the core now");
        let _ = self.sender.send(CoreInternalMessage::Stop).await;
        Ok(())
    }
}
