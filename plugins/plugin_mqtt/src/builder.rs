use std::marker::PhantomData;

use async_trait::async_trait;
use tedge_api::PluginError;
use tedge_api::plugin::MessageBundle;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tokio_util::sync::CancellationToken;
use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::DoesHandle;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;

use crate::plugin::MqttPlugin;
use crate::config::MqttConfig;

pub struct MqttPluginBuilder<MB: MessageBundle> {
    _pd: PhantomData<MB>,
}

impl<MB: MessageBundle> MqttPluginBuilder<MB> {
    pub fn new() -> Self {
        MqttPluginBuilder {
            _pd: PhantomData,
        }
    }
}

#[async_trait]
impl<PD, MB> PluginBuilder<PD> for MqttPluginBuilder<MB>
where
    PD: PluginDirectory,
    MB: MessageBundle + Sync + Send + 'static,
    MqttPlugin<MB>: DoesHandle<MB>,
{
    fn kind_name() -> &'static str {
        "mqtt"
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        HandleTypes::declare_handlers_for::<MB, MqttPlugin<MB>>()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .get_ref()
            .clone()
            .try_into()
            .map(|_: MqttConfig| ())
            .map_err(|_| anyhow::anyhow!("Failed to parse mqtt configuration"))
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        _plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config
            .into_inner()
            .try_into::<MqttConfig>()
            .map_err(|_| anyhow::anyhow!("Failed to parse mqtt configuration"))?;

        Ok(MqttPlugin::<MB>::new(config).into_untyped::<MB>())
    }
}

