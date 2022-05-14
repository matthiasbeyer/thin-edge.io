use async_trait::async_trait;

use tedge_api::address::ReplySenderFor;
use tedge_api::plugin::Handle;
use tedge_api::Address;
use tedge_api::Plugin;
use tedge_api::PluginError;

use plugin_mqtt::IncomingMessage;

use crate::error::Error;
use crate::message::ThinEdgeJsonMessage;

tedge_api::make_receiver_bundle!(pub struct ThinEdgeJsonMessageReceiver(ThinEdgeJsonMessage));

#[derive(Debug)]
pub struct ThinEdgeJsonPlugin {
    target_addr: Address<ThinEdgeJsonMessageReceiver>,
}

impl tedge_api::plugin::PluginDeclaration for ThinEdgeJsonPlugin {
    type HandledMessages = (IncomingMessage,);
}

impl ThinEdgeJsonPlugin {
    pub(crate) fn new(target_addr: Address<ThinEdgeJsonMessageReceiver>) -> Self {
        Self { target_addr }
    }
}

#[async_trait]
impl Plugin for ThinEdgeJsonPlugin {
    async fn start(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

#[async_trait]
impl Handle<IncomingMessage> for ThinEdgeJsonPlugin {
    async fn handle_message(
        &self,
        message: IncomingMessage,
        _sender: ReplySenderFor<IncomingMessage>,
    ) -> Result<(), PluginError> {
        let payload = std::str::from_utf8(message.payload()).map_err(Error::from)?;

        let payload = {
            let mut visitor = thin_edge_json::builder::ThinEdgeJsonBuilder::new();
            thin_edge_json::parser::parse_str(payload, &mut visitor).map_err(Error::from)?;

            visitor.done().map_err(Error::from)?
        };

        let message = ThinEdgeJsonMessage::from(payload);
        let _ = self
            .target_addr
            .send_and_wait(message)
            .await
            .map_err(|_| Error::FailedToSend)?;

        Ok(())
    }
}
