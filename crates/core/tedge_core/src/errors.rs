use std::path::PathBuf;

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum TedgeApplicationError {
    #[error("Plugin instantiation failed")]
    PluginInstantiationFailed(tedge_api::error::PluginError),

    #[error("Plugin errored during startup")]
    PluginErroredOnStart(tedge_api::error::PluginError),

    #[error("Config verification failed")]
    PluginConfigVerificationFailed(tedge_api::error::PluginError),

    #[error("Failed to deserialize configuration")]
    PluginConfigurationDeserializationFailed(#[from] toml::de::Error),

    #[error("Failed to read configuration file: {0}")]
    PluginConfigReadFailed(PathBuf),

    #[error("Plugin kind exists already: {0}")]
    PluginKindExists(String),

    #[error("The following Plugin kind are not covered in the configuration: {0}")]
    UnconfiguredPlugins(crate::utils::CommaSeperatedString),

    #[error("The following Plugin has no configuration: {0}")]
    PluginConfigMissing(String),

    #[error("Unknown Plugin kind: {0}")]
    UnknownPluginKind(String),

    #[error("Plugin '{0}' shutdown timeouted")]
    PluginShutdownTimeout(String),

    #[error("Plugin '{0}' shutdown errored")]
    PluginShutdownError(String),

    #[error("Plugin '{0}' setup paniced")]
    PluginSetupPaniced(String),

    #[error("Plugin '{0}' setup failed")]
    PluginSetupFailed(String, tedge_api::error::PluginError),

    #[error("Plugin '{0}' paniced in message handler")]
    PluginMessageHandlerPaniced(String),

    #[error("Plugin message handling for plugin '{0}' failed")]
    PluginMessageHandlingFailed(String),

    #[error("Message handling scheduling failed for plugin '{0}'")]
    MessageHandlingJobFailed(String, tokio::task::JoinError),
}

pub(crate) type Result<T> = std::result::Result<T, TedgeApplicationError>;

