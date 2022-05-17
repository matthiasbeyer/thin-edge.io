use async_trait::async_trait;

use futures::stream::StreamExt;
use plugin_thin_edge_json::ThinEdgeJsonMessage;
use tedge_api::address::ReplySenderFor;
use tedge_api::plugin::Handle;
use tedge_api::Address;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tedge_lib::iter::IntoSendAll;
use tedge_lib::measurement::Measurement;
use tedge_lib::measurement::MeasurementValue;
use thin_edge_json::data::ThinEdgeValue;
use tracing::Instrument;

use crate::error::Error;

tedge_api::make_receiver_bundle!(pub struct MeasurementReceiver(Measurement));

#[derive(Debug)]
pub struct ThinEdgeJsonToMeasurementMapperPlugin {
    target_addr: Address<MeasurementReceiver>,
}

impl tedge_api::plugin::PluginDeclaration for ThinEdgeJsonToMeasurementMapperPlugin {
    type HandledMessages = (ThinEdgeJsonMessage,);
}

impl ThinEdgeJsonToMeasurementMapperPlugin {
    pub(crate) fn new(target_addr: Address<MeasurementReceiver>) -> Self {
        Self { target_addr }
    }
}

#[async_trait]
impl Plugin for ThinEdgeJsonToMeasurementMapperPlugin {
    async fn start(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

#[async_trait]
impl Handle<ThinEdgeJsonMessage> for ThinEdgeJsonToMeasurementMapperPlugin {
    async fn handle_message(
        &self,
        message: ThinEdgeJsonMessage,
        _sender: ReplySenderFor<ThinEdgeJsonMessage>,
    ) -> Result<(), PluginError> {
        message
            .into_inner()
            .values
            .into_iter()
            .map(|value| match value {
                ThinEdgeValue::Single(s) => vec![s],
                ThinEdgeValue::Multi(m) => m.values, // TODO: We ignore the `MultiValueMeasurement::name` here
            })
            .map(Vec::into_iter)
            .flatten()
            .map(|msmt| Measurement::new(msmt.name, MeasurementValue::Float(msmt.value)))
            .map(|msmt| (msmt, &self.target_addr))
            .send_all()
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Vec<Result<_, _>>>()
            .instrument(tracing::debug_span!(
                "plugin.plugin_thin_edge_json_to_measurement_mapper.handle.send_all"
            ))
            .await
            .into_iter()
            .map(|r| r.map(|_| ()).map_err(|_| Error::FailedToSend)) // Ignore result, turn error into Error::FailedToSend
            .collect::<Result<Vec<()>, Error>>()
            .map(|_| ())
            .map_err(PluginError::from)
    }
}
