use std::sync::Arc;

use async_trait::async_trait;
use tedge_api::error::DirectoryError;
use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::HandleTypes;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_api::plugin::PluginExt;
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

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        HandleTypes::empty()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into()
            .map(|_: SysStatConfig| ())
            .map_err(|e| anyhow::anyhow!("Failed to parse sysinfo configuration: {:?}", e))
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
            .map_err(|e| anyhow::anyhow!("Failed to parse sysinfo configuration: {:?}", e))?;

        let build_addr_config = |adrs: &[String]| {
            adrs.iter()
                .map(|adr| plugin_dir.get_address_for(adr))
                .collect::<Result<Vec<_>, DirectoryError>>()
                .map(Arc::new)
        };

        let addr_config = crate::plugin::AddressConfig {
            memory: config.memory.as_ref().map(|cfg| build_addr_config(cfg.send_to())).unwrap_or_else(|| Ok(Arc::new(Vec::new())))?,
            network: config.network.as_ref().map(|cfg| build_addr_config(cfg.send_to())).unwrap_or_else(|| Ok(Arc::new(Vec::new())))?,
            cpu: config.cpu.as_ref().map(|cfg| build_addr_config(cfg.send_to())).unwrap_or_else(|| Ok(Arc::new(Vec::new())))?,
            disk_usage: config.disk_usage.as_ref().map(|cfg| build_addr_config(cfg.send_to())).unwrap_or_else(|| Ok(Arc::new(Vec::new())))?,
            load: config.load.as_ref().map(|cfg| build_addr_config(cfg.send_to())).unwrap_or_else(|| Ok(Arc::new(Vec::new())))?,
            process: config.process.as_ref().map(|cfg| build_addr_config(cfg.send_to())).unwrap_or_else(|| Ok(Arc::new(Vec::new())))?,
        };

        Ok(SysStatPlugin::new(config, addr_config).into_untyped())
    }
}
