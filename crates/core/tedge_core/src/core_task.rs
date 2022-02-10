use std::collections::HashMap;

use tedge_api::address::EndpointKind;
use tracing::trace;

use crate::task::Task;

type Receiver = tokio::sync::mpsc::Receiver<tedge_api::message::Message>;
type Sender = tokio::sync::mpsc::Sender<tedge_api::message::Message>;

pub struct CoreTask {
    recv: Receiver,
    plugin_senders: HashMap<String, Sender>,
}

impl std::fmt::Debug for CoreTask {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let plugin_senders = f.debug_set().entries(self.plugin_senders.keys()).finish();
        f.debug_struct("CoreTask")
            .field("plugin_senders", &plugin_senders)
            .finish()
    }
}

impl CoreTask {
    pub fn new(recv: Receiver, plugin_senders: HashMap<String, Sender>) -> Self {
        Self {
            recv,
            plugin_senders,
        }
    }
}

#[async_trait::async_trait]
impl Task for CoreTask {
    #[tracing::instrument]
    async fn run(mut self) -> crate::errors::Result<()> {
        while let Some(message) = self.recv.recv().await {
            match message.destination().endpoint_kind() {
                EndpointKind::Plugin { id } => {
                    trace!("Received message in core, routing to {}", id);
                    if let Some(sender) = self.plugin_senders.get(id) {
                        match sender.send(message).await {
                            Ok(()) => trace!("Sent successfully"),
                            Err(e) => trace!("Error sending message: {:?}", e),
                        }
                    }
                }

                EndpointKind::Core => {
                    trace!("Received message in core");

                    // TODO: Implement
                }
            }
        }

        Ok(())
    }
}
