use async_trait::async_trait;
use tedge_api::CancellationToken;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;

use crate::config::C8yConfig;
use crate::plugin::C8yPlugin;

pub struct C8yPluginBuilder;

#[async_trait]
impl<PD> PluginBuilder<PD> for C8yPluginBuilder
where
    PD: PluginDirectory,
{
    fn kind_name() -> &'static str {
        "c8y"
    }

    fn kind_configuration() -> Option<tedge_api::ConfigDescription> {
        Some(<C8yConfig as tedge_api::AsConfig>::as_config())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        C8yPlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into()
            .map(|_: C8yConfig| ())
            .map_err(crate::error::Error::ConfigParseFailed)
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        _plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config
            .try_into::<C8yConfig>()
            .map_err(crate::error::Error::ConfigParseFailed)?;

        Ok(C8yPlugin::new(config).finish())
    }
}

