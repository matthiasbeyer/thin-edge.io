use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_lib::measurement::Measurement;

use crate::config::MeasurementFilterConfig;
use crate::plugin::MeasurementFilterPlugin;

tedge_api::make_receiver_bundle!(pub struct MeasurementReceiver(Measurement));

pub struct MeasurementFilterPluginBuilder;

#[async_trait]
impl<PD> PluginBuilder<PD> for MeasurementFilterPluginBuilder
where
    PD: PluginDirectory,
{
    fn kind_name() -> &'static str {
        "measurement_filter"
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        MeasurementFilterPlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into()
            .map(|_: MeasurementFilterConfig| ())
            .map_err(|_| miette::miette!("Failed to parse filter configuration"))
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config
            .try_into::<MeasurementFilterConfig>()
            .map_err(|_| miette::miette!("Failed to parse filter configuration"))?;

        let main_addr = plugin_dir.get_address_for(&config.target)?;
        let filtered_addr = config
            .filtered_target
            .as_ref()
            .map(|filtered| plugin_dir.get_address_for(filtered))
            .transpose()?;

        Ok({
            MeasurementFilterPlugin::new(main_addr, filtered_addr, config.extractor, config.filter)
                .finish()
        })
    }
}
