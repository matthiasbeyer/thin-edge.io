use async_trait::async_trait;
use tedge_api::{address::ReplySenderFor, message::StopCore, plugin::Handle, Plugin, PluginError};
use tracing::trace;

#[derive(Clone)]
pub struct CorePlugin {
    sender: tokio::sync::mpsc::Sender<CoreInternalMessage>,
}

impl CorePlugin {
    pub fn new(sender: tokio::sync::mpsc::Sender<CoreInternalMessage>) -> Self {
        Self { sender }
    }
}

impl tedge_api::plugin::PluginDeclaration for CorePlugin {
    type HandledMessages = (StopCore,);
}

pub enum CoreInternalMessage {
    Stop,
}

#[async_trait]
impl Plugin for CorePlugin {
    async fn start(&mut self) -> std::result::Result<(), PluginError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> std::result::Result<(), PluginError> {
        Ok(())
    }
}

#[async_trait]
impl Handle<StopCore> for CorePlugin {
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
