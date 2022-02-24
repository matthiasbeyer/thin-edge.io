use async_trait::async_trait;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginError;

use crate::config::SysStatConfig;
use crate::plugin::SysStatPlugin;

pub struct SysStatPluginBuilder;

#[async_trait]
impl PluginBuilder for SysStatPluginBuilder {
    fn kind_name(&self) -> &'static str {
        "sysinfo"
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .get_ref()
            .clone()
            .try_into()
            .map(|_: SysStatConfig| ())
            .map_err(|e| anyhow::anyhow!("Failed to parse sysinfo configuration: {:?}", e))
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        comms: tedge_api::plugin::CoreCommunication,
    ) -> Result<Box<dyn Plugin>, PluginError> {
        let config = config
            .into_inner()
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to parse sysinfo configuration: {:?}", e))?;

        Ok(Box::new(SysStatPlugin::new(comms, config)))
    }
}
