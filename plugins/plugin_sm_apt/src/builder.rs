use async_trait::async_trait;

use tedge_api::CancellationToken;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;

use crate::config::Config;
use crate::plugin::SmAptPlugin;

pub struct SmAptPluginBuilder;

#[async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for SmAptPluginBuilder {
    fn kind_name() -> &'static str {
        "sm_apt"
    }

    fn kind_configuration() -> Option<tedge_api::ConfigDescription> {
        Some(<Config as tedge_api::AsConfig>::as_config())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        SmAptPlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into::<Config>()
            .map_err(crate::error::Error::ConfigParseFailed)
            .map_err(tedge_api::error::PluginError::from)
            .map(|_| ())
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        _plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config
            .try_into::<Config>()
            .map_err(crate::error::Error::ConfigParseFailed)?;

        Ok(SmAptPlugin::new(config.apt_binary.as_ref().map(std::path::Path::new)).finish())
    }
}

