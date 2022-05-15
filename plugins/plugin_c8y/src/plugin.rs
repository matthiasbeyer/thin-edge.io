use async_trait::async_trait;

use tedge_api::address::ReplySenderFor;
use tedge_api::plugin::Handle;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tedge_lib::measurement::Measurement;
use plugin_mqtt::IncomingMessage;

use crate::config::C8yConfig;

#[derive(Debug)]
pub struct C8yPlugin {
    config: C8yConfig,
}

impl C8yPlugin {
    pub(crate) fn new(config: C8yConfig) -> Self {
        Self {
            config,
        }
    }
}

impl tedge_api::plugin::PluginDeclaration for C8yPlugin {
    type HandledMessages = (IncomingMessage, Measurement);
}

#[async_trait]
impl Plugin for C8yPlugin {
    #[tracing::instrument(name = "plugin.c8y.start", skip(self))]
    async fn start(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    #[tracing::instrument(name = "plugin.c8y.shutdown", skip(self))]
    async fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

#[async_trait]
impl Handle<Measurement> for C8yPlugin {
    #[tracing::instrument(name = "plugin.c8y.handle_message", level = "trace")]
    async fn handle_message(
        &self,
        _message: Measurement,
        _sender: ReplySenderFor<Measurement>,
    ) -> Result<(), PluginError> {
        // TODO
        Ok(())
    }
}

#[async_trait]
impl Handle<IncomingMessage> for C8yPlugin {
    #[tracing::instrument(name = "plugin.c8y.handle_message", level = "trace")]
    async fn handle_message(
        &self,
        _message: IncomingMessage,
        _sender: ReplySenderFor<IncomingMessage>,
    ) -> Result<(), PluginError> {
        // TODO
        Ok(())
    }
}
