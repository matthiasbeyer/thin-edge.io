use async_trait::async_trait;

use tedge_api::address::Address;
use tedge_api::address::ReplySenderFor;
use tedge_api::plugin::Handle;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tracing::debug;

use tedge_lib::measurement::Measurement;
use plugin_mqtt::IncomingMessage;


tedge_api::make_receiver_bundle!(pub struct MeasurementReceiver(Measurement));

#[derive(Debug)]
pub struct CollectdPlugin {
    target_addr: Address<MeasurementReceiver>,
}


impl CollectdPlugin {
    pub(crate) fn new(target_addr: Address<MeasurementReceiver>) -> Self {
        Self {
            target_addr,
        }
    }
}

#[async_trait]
impl Plugin for CollectdPlugin {
    #[tracing::instrument(name = "plugin.collectd.start", skip(self))]
    async fn start(&mut self) -> Result<(), PluginError> {
        debug!("Setting up collectd plugin!");
        Ok(())
    }

    #[tracing::instrument(name = "plugin.collectd.shutdown", skip(self))]
    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down collectd plugin!");
        Ok(())
    }
}

impl tedge_api::plugin::PluginDeclaration for CollectdPlugin {
    type HandledMessages = (IncomingMessage,);
}

#[async_trait]
impl Handle<IncomingMessage> for CollectdPlugin {
    #[tracing::instrument(name = "plugin.collectd.handle_message", level = "trace")]
    async fn handle_message(
        &self,
        _message: IncomingMessage,
        _sender: ReplySenderFor<IncomingMessage>,
    ) -> Result<(), PluginError> {
        Ok(())
    }
}
