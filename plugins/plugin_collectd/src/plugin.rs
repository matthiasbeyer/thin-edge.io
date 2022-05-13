use async_trait::async_trait;

use futures::stream::StreamExt;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tedge_api::address::Address;
use tedge_api::address::ReplySenderFor;
use tedge_api::plugin::Handle;
use tracing::debug;

use tedge_lib::measurement::Measurement;
use plugin_mqtt::IncomingMessage;

use crate::error::Error;
use crate::measurement::CollectdMeasurement;
use crate::payload::CollectdPayload;
use crate::topic::CollectdTopic;

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
        message: IncomingMessage,
        _sender: ReplySenderFor<IncomingMessage>,
    ) -> Result<(), PluginError> {
        let topic = CollectdTopic::parse(message.topic())?;
        let payload = std::str::from_utf8(message.payload())
            .map_err(Error::MessagePayloadNotUtf8)
            .and_then(CollectdPayload::parse)?;

        CollectdMeasurement::new(topic, payload)
            .into_measurements()
            .map(|msmt| async {
                self.target_addr
                    .send_and_wait(msmt)
                    .await
                    .map(|_| ())
                    .map_err(|_| Error::FailedToSendMeasurement)
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Vec<Result<_, Error>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<()>, Error>>()
            .map_err(PluginError::from)
            .map(|_| ())
    }
}
