use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use tedge_api::address::Address;
use tedge_api::error::PluginError;
use tedge_api::message::Message;
use tedge_api::message::MessageKind;
use tedge_api::plugin::CoreCommunication;

pub type ReplySender = tokio::sync::oneshot::Sender<Message>;
pub type ReplyReceiver = tokio::sync::oneshot::Receiver<Message>;

pub trait IntoReplyable {
    fn with_replies(self) -> ReplyableCoreCommunication;
}

impl IntoReplyable for CoreCommunication {
    fn with_replies(self) -> ReplyableCoreCommunication {
        ReplyableCoreCommunication::new(self)
    }
}

pub struct ReplyableCoreCommunication {
    comms: CoreCommunication,
    replymap: Arc<RwLock<HashMap<uuid::Uuid, ReplySender>>>,
}

impl ReplyableCoreCommunication {
    fn new(comms: CoreCommunication) -> Self {
        Self {
            comms,
            replymap: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn send(
        &self,
        msg_kind: MessageKind,
        destination: Address,
    ) -> Result<uuid::Uuid, PluginError> {
        self.comms.send(msg_kind, destination).await
    }

    pub async fn send_and_wait_for_reply(
        &self,
        msg_kind: MessageKind,
        destination: Address,
    ) -> Result<ReplyReceiver, PluginError> {
        let msg_id = self.send(msg_kind, destination).await?;
        let mut map = self.replymap.write().await;
        let (tx, rx) = tokio::sync::oneshot::channel();
        map.insert(msg_id, tx);
        Ok(rx)
    }

    /// Process a message that could be a reply
    ///
    /// # Returns
    ///
    /// * Ok(Some(Message)) if the message was not handled
    /// * Ok(None) if the message was handled
    /// * Err(_) in case of error
    ///
    pub async fn handle_reply(&self, msg: Message) -> Result<Option<Message>, PluginError> {
        if let Some(sender) = self.replymap.write().await.remove(msg.id()) {
            match sender.send(msg) {
                Ok(()) => Ok(None),
                Err(msg) => Ok(Some(msg)), // TODO: Is this the right way?
            }
        } else {
            Ok(Some(msg))
        }
    }
}
