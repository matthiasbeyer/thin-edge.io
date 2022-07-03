use async_trait::async_trait;

use tedge_api::plugin::Handle;
use tedge_api::Address;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tedge_lib::measurement::Measurement;
use tracing::trace;

use crate::builder::MeasurementReceiver;
use crate::builder::NotificationReceiver;
use crate::config::NotificationType;

#[derive(Debug)]
pub struct NotificationPlugin {
    target_addr: Address<MeasurementReceiver>,
    notify_addr: Address<NotificationReceiver>,

    raise: NotificationType,
    raise_msg: String,
}

impl tedge_api::plugin::PluginDeclaration for NotificationPlugin {
    type HandledMessages = (Measurement,);
}

impl NotificationPlugin {
    pub fn new(
        target_addr: Address<MeasurementReceiver>,
        notify_addr: Address<NotificationReceiver>,
        raise: NotificationType,
        raise_msg: String,
    ) -> Self {
        Self {
            target_addr,
            notify_addr,
            raise,
            raise_msg,
        }
    }
}

#[async_trait]
impl Plugin for NotificationPlugin {}

#[async_trait]
impl Handle<Measurement> for NotificationPlugin {
    #[tracing::instrument(name = "plugin.notification.handle_message", skip(self, _sender))]
    async fn handle_message(
        &self,
        message: Measurement,
        _sender: tedge_api::address::ReplySenderFor<Measurement>,
    ) -> Result<(), PluginError> {
        trace!(?message, "Received measurement");
        trace!(?message, "Sending notification for measurement");
        let _ = self
            .notify_addr
            .send_and_wait(
                self.raise
                    .clone()
                    .into_notification(self.raise_msg.to_string()),
            )
            .await;

        trace!(?message, "Forwarding measurement");
        let _ = self.target_addr.send_and_wait(message).await;
        Ok(())
    }
}