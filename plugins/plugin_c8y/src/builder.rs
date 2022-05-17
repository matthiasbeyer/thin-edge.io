use async_trait::async_trait;
use tedge_api::CancellationToken;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;
use tedge_lib::sm::request::Install;
use tedge_lib::sm::request::List;
use tedge_lib::sm::request::Uninstall;
use tedge_lib::sm::request::Update;

use crate::config::C8yConfig;
use crate::plugin::C8yPlugin;

tedge_api::make_receiver_bundle!(pub struct SmReceiver(List, Install, Update, Uninstall));

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
        plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config
            .try_into::<C8yConfig>()
            .map_err(crate::error::Error::ConfigParseFailed)?;

        let sm_addr = plugin_dir.get_address_for::<SmReceiver>(&config.sm_plugin_name)?;
        Ok(C8yPlugin::new(config, sm_addr).finish())
    }
}

