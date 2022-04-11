use std::marker::PhantomData;

use async_trait::async_trait;

use tedge_api::address::ReceiverBundle;
use tedge_api::address::ReplySender;
use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::DoesHandle;
use tedge_api::plugin::Handle;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::Message;
use tedge_api::plugin::MessageBundle;
use tedge_api::plugin::PluginExt;
use tedge_api::Address;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::event;

pub struct LogPluginBuilder<MB>
where
    MB: MessageBundle + ReceiverBundle,
{
    _pd: PhantomData<MB>,
}

impl<MB: MessageBundle + ReceiverBundle> LogPluginBuilder<MB> {
    pub fn new() -> Self {
        LogPluginBuilder { _pd: PhantomData }
    }
}

#[derive(serde::Deserialize, Debug)]
struct LogConfig {
    level: log::Level,
    acknowledge: bool,
    forward_to: Option<String>,
}

#[async_trait]
impl<PD, MB> PluginBuilder<PD> for LogPluginBuilder<MB>
where
    PD: PluginDirectory,
    MB: MessageBundle + ReceiverBundle + Sync + Send + 'static,
    LogPlugin<MB>: DoesHandle<MB>,
{
    fn kind_name() -> &'static str {
        "log"
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        HandleTypes::declare_handlers_for::<MB, LogPlugin<MB>>()
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
        _cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config
            .into_inner()
            .try_into::<LogConfig>()
            .map_err(|_| anyhow::anyhow!("Failed to parse log configuration"))?;

        let forward_to = config
            .forward_to
            .as_ref()
            .map(|addr| plugin_dir.get_address_for(addr))
            .transpose()?;

        Ok(LogPlugin::<MB>::new(config, forward_to).into_untyped::<MB>())
    }
}

struct LogPlugin<MB>
where
    MB: MessageBundle + ReceiverBundle + Sync + Send + 'static,
{
    _pd: PhantomData<MB>,
    config: LogConfig,
    forward_to: Option<Address<MB>>,
}

impl<MB> LogPlugin<MB>
where
    MB: MessageBundle + ReceiverBundle + Sync + Send + 'static,
{
    fn new(config: LogConfig, forward_to: Option<Address<MB>>) -> Self {
        Self {
            _pd: PhantomData,
            config,
            forward_to,
        }
    }
}

#[async_trait]
impl<MB> Plugin for LogPlugin<MB>
where
    MB: MessageBundle + ReceiverBundle + Sync + Send + 'static,
{
    async fn setup(&mut self) -> Result<(), PluginError> {
        debug!(
            "Setting up log plugin with default level = {}, acknowledge = {}!",
            self.config.level, self.config.acknowledge
        );

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down log plugin!");
        Ok(())
    }
}

#[async_trait]
impl<M, MB> Handle<M> for LogPlugin<MB>
where
    M: Message + std::fmt::Debug,
    MB: MessageBundle + ReceiverBundle + Sync + Send + 'static,
    MB: tedge_api::address::Contains<M>,
{
    async fn handle_message(
        &self,
        message: M,
        mut sender: ReplySender<M::Reply>,
    ) -> Result<(), PluginError> {
        match self.config.level {
            log::Level::Trace => {
                event!(tracing::Level::TRACE, "Received Message: {:?}", message);
            }
            log::Level::Debug => {
                event!(tracing::Level::DEBUG, "Received Message: {:?}", message);
            }
            log::Level::Info => event!(tracing::Level::INFO, "Received Message: {:?}", message),
            log::Level::Warn => event!(tracing::Level::WARN, "Received Message: {:?}", message),
            log::Level::Error => {
                event!(tracing::Level::ERROR, "Received Message: {:?}", message)
            }
        }

        if let Some(fwd) = self.forward_to.as_ref() {
            match fwd.send(message).await {
                Ok(reply_recv) => {
                    tokio::select! {
                        reply = reply_recv.wait_for_reply(std::time::Duration::MAX) => {
                            match reply {
                                Ok(m) => {
                                    let _ = sender.reply(m);
                                },

                                Err(e) => {
                                    debug!("Failed to receive reply: {:?}", e);
                                }
                            }
                        }

                        _ = sender.closed() => {
                            debug!("Reply-channel was closed, we cannot do anything");
                        }
                    }
                },

                Err(_msg) => {
                    debug!(
                        "Failed to forward message to {}",
                        self.config
                            .forward_to
                            .as_ref()
                            .expect("Address exists but not config for it")
                    );
                    // drop the message
                }
            }
        }

        Ok(())
    }
}
