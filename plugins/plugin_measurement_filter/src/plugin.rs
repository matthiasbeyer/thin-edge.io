use async_trait::async_trait;
use tracing::debug;

use tedge_api::address::Address;
use tedge_api::address::ReplySenderFor;
use tedge_api::error::PluginError;
use tedge_api::plugin::Handle;
use tedge_api::plugin::Plugin;
use tedge_lib::measurement::Measurement;
use tracing::trace;

use crate::builder::MeasurementReceiver;
use crate::extractor::Extractable;
use crate::filter::Filterable;

#[derive(Debug)]
pub struct MeasurementFilterPlugin {
    target: Address<MeasurementReceiver>,
    filtered_target: Option<Address<MeasurementReceiver>>,

    extractor: crate::extractor::Extractor,
    filter: crate::filter::Filter,
}

impl MeasurementFilterPlugin {
    pub fn new(
        target: Address<MeasurementReceiver>,
        filtered_target: Option<Address<MeasurementReceiver>>,
        extractor: crate::extractor::Extractor,
        filter: crate::filter::Filter,
    ) -> Self {
        Self {
            target,
            filtered_target,
            extractor,
            filter,
        }
    }
}

impl tedge_api::plugin::PluginDeclaration for MeasurementFilterPlugin {
    type HandledMessages = (Measurement,);
}

#[async_trait]
impl Plugin for MeasurementFilterPlugin {
    async fn start(&mut self) -> Result<(), PluginError> {
        debug!("Setting up filter plugin");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down filter plugin!");
        Ok(())
    }
}

#[async_trait]
impl Handle<Measurement> for MeasurementFilterPlugin {
    #[tracing::instrument(name = "plugin.measurement_filter.handle_message", level = "trace")]
    async fn handle_message(
        &self,
        message: Measurement,
        _sender: ReplySenderFor<Measurement>,
    ) -> Result<(), PluginError> {
        trace!(plugin.extractor = ?self.extractor, ?message, "Extracting from message");
        if let Some(value) = message.extract(&self.extractor.0) {
            trace!(plugin.filter = ?self.filter, ?value, "Applying filter");
            if value.apply_filter(&self.filter) {
                let _ = self.target.send_and_wait(message).await;
            } else {
                if let Some(ftarget) = self.filtered_target.as_ref() {
                    let _ = ftarget.send_and_wait(message).await;
                }
            }
        }
        Ok(())
    }
}
