use async_trait::async_trait;

use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;
use tedge_api::CancellationToken;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;

use crate::config::ThinEdgeJsonConfig;
use crate::plugin::ThinEdgeJsonPlugin;

pub struct ThinEdgeJsonPluginBuilder;

#[async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for ThinEdgeJsonPluginBuilder {
    fn kind_name() -> &'static str {
        "thin_edge_json"
    }

    fn kind_configuration() -> Option<tedge_api::ConfigDescription> {
        Some(<ThinEdgeJsonConfig as tedge_api::AsConfig>::as_config())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        ThinEdgeJsonPlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into()
            .map(|_: ThinEdgeJsonConfig| ())
            .map_err(crate::error::Error::ConfigParseFailed)
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config: ThinEdgeJsonConfig = config
            .try_into()
            .map_err(crate::error::Error::ConfigParseFailed)?;

        let target_addr = plugin_dir
            .get_address_for::<crate::plugin::ThinEdgeJsonMessageReceiver>(config.target())?;

        Ok(ThinEdgeJsonPlugin::new(target_addr).finish())
    }
}
