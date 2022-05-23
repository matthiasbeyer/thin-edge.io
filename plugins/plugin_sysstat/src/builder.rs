use std::sync::Arc;

use async_trait::async_trait;
use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_lib::address::AddressGroup;
use tedge_lib::config::Address;
use tedge_lib::config::OneOrMany;
use tokio_util::sync::CancellationToken;

use crate::config::HasBaseConfig;
use crate::config::SysStatConfig;
use crate::plugin::SysStatPlugin;

pub struct SysStatPluginBuilder;

#[async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for SysStatPluginBuilder {
    fn kind_name() -> &'static str {
        "sysinfo"
    }

    fn kind_configuration() -> Option<tedge_api::ConfigDescription> {
        Some(<SysStatConfig as tedge_api::AsConfig>::as_config())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        SysStatPlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into()
            .map(|_: SysStatConfig| ())
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
            .try_into::<SysStatConfig>()
            .map_err(crate::error::Error::ConfigParseFailed)
            .map_err(PluginError::from)?;

        let build_addr_config =
            |adrs: &OneOrMany<Address>| -> Result<Arc<AddressGroup<_>>, PluginError> {
                AddressGroup::build(plugin_dir, adrs)
                    .map(Arc::new)
                    .map_err(PluginError::from)
            };

        let addr_config = crate::plugin::AddressConfig {
            memory: config
                .memory
                .as_ref()
                .map(|cfg| build_addr_config(cfg.send_to()))
                .transpose()?,
            network: config
                .network
                .as_ref()
                .map(|cfg| build_addr_config(cfg.send_to()))
                .transpose()?,
            cpu: config
                .cpu
                .as_ref()
                .map(|cfg| build_addr_config(cfg.send_to()))
                .transpose()?,
            disk_usage: config
                .disk_usage
                .as_ref()
                .map(|cfg| build_addr_config(cfg.send_to()))
                .transpose()?,
            load: config
                .load
                .as_ref()
                .map(|cfg| build_addr_config(cfg.send_to()))
                .transpose()?,
            process: config
                .process
                .as_ref()
                .map(|cfg| build_addr_config(cfg.send_to()))
                .transpose()?,
        };

        Ok(SysStatPlugin::new(config, addr_config).finish())
    }
}
