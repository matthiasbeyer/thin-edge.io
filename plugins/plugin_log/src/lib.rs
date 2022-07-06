use std::marker::PhantomData;

use async_trait::async_trait;

use tedge_api::address::ReplySenderFor;
use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::DoesHandle;
use tedge_api::plugin::Handle;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::MessageBundle;
use tedge_api::plugin::PluginExt;
use tedge_api::Message;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::event;

pub struct LogPluginBuilder<MB: MessageBundle> {
    _pd: PhantomData<MB>,
}

impl<MB: MessageBundle> Default for LogPluginBuilder<MB> {
    fn default() -> Self {
        LogPluginBuilder { _pd: PhantomData }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(transparent)]
struct LogLevel(log::Level);

impl tedge_api::AsConfig for LogLevel {
    fn as_config() -> tedge_api::ConfigDescription {
        tedge_api::ConfigDescription::new(
            String::from("LogLevel"),
            tedge_api::ConfigKind::Enum(
                tedge_api::config::ConfigEnumKind::Untagged,
                vec![
                    (
                        "String",
                        Some("Log level 'trace'"),
                        tedge_api::config::EnumVariantRepresentation::String("trace"),
                    ),
                    (
                        "String",
                        Some("Log level 'debug'"),
                        tedge_api::config::EnumVariantRepresentation::String("debug"),
                    ),
                    (
                        "String",
                        Some("Log level 'info'"),
                        tedge_api::config::EnumVariantRepresentation::String("info"),
                    ),
                    (
                        "String",
                        Some("Log level 'warn'"),
                        tedge_api::config::EnumVariantRepresentation::String("warn"),
                    ),
                    (
                        "String",
                        Some("Log level 'error'"),
                        tedge_api::config::EnumVariantRepresentation::String("error"),
                    ),
                ],
            ),
            None,
        )
    }
}

#[derive(serde::Deserialize, Debug, tedge_api::Config)]
struct LogConfig {
    /// The level to log with
    level: LogLevel,

    /// Whether to send an acknowlegement that the message was logged
    acknowledge: bool,
}

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
enum Error {
    #[error("Failed to parse configuration")]
    ConfigParseFailed(#[from] toml::de::Error),
}

#[async_trait]
impl<PD, MB> PluginBuilder<PD> for LogPluginBuilder<MB>
where
    PD: PluginDirectory,
    MB: MessageBundle + Sync + Send + 'static,
    LogPlugin<MB>: DoesHandle<MB>,
{
    fn kind_name() -> &'static str {
        "log"
    }

    fn kind_configuration() -> Option<tedge_api::ConfigDescription> {
        Some(<LogConfig as tedge_api::AsConfig>::as_config())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        LogPlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into()
            .map(|_: LogConfig| ())
            .map_err(Error::from)
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        _plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config.try_into::<LogConfig>().map_err(Error::from)?;

        Ok(LogPlugin::<MB>::new(config).finish())
    }
}

struct LogPlugin<MB> {
    _pd: PhantomData<MB>,
    config: LogConfig,
}

impl<MB> tedge_api::plugin::PluginDeclaration for LogPlugin<MB>
where
    MB: MessageBundle + Sync + Send + 'static,
{
    type HandledMessages = MB;
}

impl<MB> LogPlugin<MB>
where
    MB: MessageBundle + Sync + Send + 'static,
{
    fn new(config: LogConfig) -> Self {
        Self {
            _pd: PhantomData,
            config,
        }
    }
}

#[async_trait]
impl<MB> Plugin for LogPlugin<MB>
where
    MB: MessageBundle + Sync + Send + 'static,
{
    async fn start(&mut self) -> Result<(), PluginError> {
        debug!(
            "Setting up log plugin with default level = {}, acknowledge = {}!",
            self.config.level.0, self.config.acknowledge
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
    MB: MessageBundle + Sync + Send + 'static,
{
    async fn handle_message(
        &self,
        message: M,
        _sender: ReplySenderFor<M>,
    ) -> Result<(), PluginError> {
        match self.config.level.0 {
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

        Ok(())
    }
}
