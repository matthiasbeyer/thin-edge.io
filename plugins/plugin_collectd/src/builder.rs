use async_trait::async_trait;
use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tokio_util::sync::CancellationToken;

use crate::config::CollectdConfig;
use crate::plugin::CollectdPlugin;

pub struct CollectdPluginBuilder;

#[async_trait]
impl<PD> PluginBuilder<PD> for CollectdPluginBuilder
where
    PD: PluginDirectory,
{
    fn kind_name() -> &'static str {
        "collectd"
    }

    fn kind_configuration() -> Option<tedge_api::ConfigDescription> {
        Some(<CollectdConfig as tedge_api::AsConfig>::as_config())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        CollectdPlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into()
            .map(|_: CollectdConfig| ())
            .map_err(crate::error::Error::ConfigParseFailed)
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config
            .try_into::<CollectdConfig>()
            .map_err(crate::error::Error::ConfigParseFailed)?;

        let addr = plugin_dir.get_address_for(&config.target)?;
        Ok(CollectdPlugin::new(addr).finish())
    }
}
