use async_trait::async_trait;

use tedge_api::Message;
use tedge_api::MessageKind;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginError;
use tracing::debug;
use tracing::event;

pub struct LogPluginBuilder;

#[derive(serde::Deserialize, Debug)]
struct LogConfig {
    level: log::Level,
    acknowledge: bool,
}

#[async_trait]
impl PluginBuilder for LogPluginBuilder {
    fn kind_name(&self) -> &'static str {
        "log"
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .get_ref()
            .clone()
            .try_into()
            .map(|_: LogConfig| ())
            .map_err(|_| anyhow::anyhow!("Failed to parse log configuration"))
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
            .map_err(|_| anyhow::anyhow!("Failed to parse log configuration"))?;

        Ok(Box::new(LogPlugin::new(comms, config)))
    }
}

struct LogPlugin {
    comms: tedge_api::plugin::CoreCommunication,
    config: LogConfig,
}

impl LogPlugin {
    fn new(comms: tedge_api::plugin::CoreCommunication, config: LogConfig) -> Self {
        Self { comms, config }
    }
}

#[async_trait]
impl Plugin for LogPlugin {
    async fn setup(&mut self) -> Result<(), PluginError> {
        debug!(
            "Setting up log plugin with default level = {}, acknowledge = {}!",
            self.config.level, self.config.acknowledge
        );

        Ok(())
    }

    async fn handle_message(&self, message: Message) -> Result<(), PluginError> {
        match self.config.level {
            log::Level::Trace => {
                event!(
                    tracing::Level::TRACE,
                    "Received Message: {id} from {origin:?}: {kind:?}",
                    id = message.id(),
                    origin = message.origin(),
                    kind = message.kind(),
                );
            }
            log::Level::Debug => {
                event!(
                    tracing::Level::DEBUG,
                    "Received Message: {id} from {origin:?}: {kind:?}",
                    id = message.id(),
                    origin = message.origin(),
                    kind = message.kind(),
                );
            }
            log::Level::Info => event!(tracing::Level::INFO, "Received Message: {}", message.id()),
            log::Level::Warn => event!(tracing::Level::WARN, "Received Message: {}", message.id()),
            log::Level::Error => {
                event!(tracing::Level::ERROR, "Received Message: {}", message.id())
            }
        }

        if self.config.acknowledge {
            let addr = message.origin().clone();
            let kind = MessageKind::Reply {
                message_id: message.id().clone(),
                content: Box::new(MessageKind::CheckReadyness), // TODO: Send a decent message here
            };
            let _ = self.comms.send(kind, addr).await?;
        }

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down log plugin!");
        Ok(())
    }
}
