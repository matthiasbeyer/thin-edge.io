use std::marker::PhantomData;

use async_trait::async_trait;

use tedge_api::address::ReplySender;
use tedge_api::plugin::Handle;
use tedge_api::plugin::Message;
use tedge_api::plugin::MessageBundle;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tracing::debug;

use crate::config::MqttConfig;

pub struct MqttPlugin<MB> {
    _pd: PhantomData<MB>,
    config: MqttConfig,
}

impl<MB> MqttPlugin<MB>
where
    MB: MessageBundle + Sync + Send + 'static,
{
    pub(crate) fn new(config: MqttConfig) -> Self {
        Self { _pd: PhantomData, config }
    }
}

#[async_trait]
impl<MB> Plugin for MqttPlugin<MB>
where
    MB: MessageBundle + Sync + Send + 'static,
{
    async fn setup(&mut self) -> Result<(), PluginError> {
        debug!("Setting up mqtt plugin!");
        unimplemented!()
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down mqtt plugin!");
        unimplemented!()
    }
}

#[async_trait]
impl<M, MB> Handle<M> for MqttPlugin<MB>
where
    M: Message + serde::Serialize,
    M::Reply: serde::de::DeserializeOwned,
    MB: MessageBundle + Sync + Send + 'static,
{
    async fn handle_message(
        &self,
        _message: M,
        _sender: ReplySender<M::Reply>,
    ) -> Result<(), PluginError> {
        unimplemented!()
    }
}

