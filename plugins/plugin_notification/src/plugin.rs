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
impl Plugin for NotificationPlugin {
    async fn start(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        trace!("Shutdown");
        Ok(())
    }
}

#[async_trait]
impl Handle<Measurement> for NotificationPlugin {
    async fn handle_message(
        &self,
        message: Measurement,
        _sender: tedge_api::address::ReplySenderFor<Measurement>,
    ) -> Result<(), PluginError> {
        trace!("Received measurement = {:?}", message);
        trace!("Sending notification for measurement = {:?}", message);
        let _ = self.notify_addr.send_and_wait(self.raise.clone().into_notification(self.raise_msg.to_string())).await;

        trace!("Forwarding measurement = {:?}", message);
        let _ = self.target_addr.send_and_wait(message).await;
        Ok(())
    }
}
