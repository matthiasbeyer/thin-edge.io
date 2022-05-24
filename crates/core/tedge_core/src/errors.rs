use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum TedgeApplicationError {
    #[error("Could not complete building the tedge application because of the following errors")]
    ApplicationBuilderErrors {
        #[related]
        errors: Vec<TedgeApplicationBuilderError>,
    },
    #[error("Could not verify the configuration of one or more plugins")]
    PluginConfigVerificationsError {
        #[related]
        errors: Vec<PluginConfigurationError>,
    },
    #[error("Could not instantiate one or more plugins")]
    PluginInstantiationsError {
        #[related]
        errors: Vec<PluginInstantiationError>,
    },
    #[error("Could not shutdown one or more plugins")]
    PluginLifecycleErrors {
        #[related]
        errors: Vec<PluginLifecycleError>,
    },
}

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum TedgeApplicationBuilderError {
    #[error("A plugin builder with the duplicate name '{name}' was registered")]
    #[diagnostic(help("The duplicate name was registered by the builder name '{builder_name}'"))]
    DuplicateKind {
        name: String,
        builder_name: &'static str,
    },
    #[error("Could not read configuration at {:?}", path)]
    PathNotReadable {
        path: PathBuf,
        #[source]
        error: std::io::Error,
    },
    #[error("Could not parse configuration at {:?}", path)]
    ConfigNotParseable {
        path: PathBuf,
        #[source]
        error: toml::de::Error,
    },
}

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum PluginConfigurationError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Verification(PluginConfigVerificationError),
    #[error("Given path is not a filepath: {:?}", path)]
    PathNotAFilePath { path: PathBuf },
    #[error("Could not read from path: {:?}", path)]
    PathNotReadable {
        path: PathBuf,
        #[source]
        error: std::io::Error,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    UnknownKind(PluginKindUnknownError),
}

#[derive(miette::Diagnostic, Debug, thiserror::Error)]
#[error("No plugin with kind '{name}' is currently registered with the application")]
pub struct PluginKindUnknownError {
    pub name: String,
    #[diagnostic(help)]
    pub alternatives: Option<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to instantiate plugin '{name}'")]
pub struct PluginConfigVerificationError {
    pub name: String,
    pub error: tedge_api::error::PluginError,
}

impl miette::Diagnostic for PluginConfigVerificationError {
    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        Some(Box::new(std::iter::once(self.error.as_ref())))
    }
}

#[derive(miette::Diagnostic, Debug, thiserror::Error)]
#[error("No configuration for plugin '{name}' was found.")]
#[diagnostic(help("Add a configuration block for plugin '{name}'"))]
pub struct PluginConfigurationNotFoundError {
    pub name: String,
}

#[derive(miette::Diagnostic, Debug, thiserror::Error)]
pub enum PluginInstantiationError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    BuilderInstantiation(PluginBuilderInstantiationError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    KindNotFound(PluginKindUnknownError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    ConfigurationNotFound(PluginConfigurationNotFoundError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    ConfigurationVerificationFailed(PluginConfigVerificationError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to instantiate plugin '{name}'")]
pub struct PluginBuilderInstantiationError {
    pub name: String,
    pub error: tedge_api::error::PluginError,
}

impl miette::Diagnostic for PluginBuilderInstantiationError {
    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        Some(Box::new(std::iter::once(self.error.as_ref())))
    }
}

#[derive(miette::Diagnostic, Debug, thiserror::Error)]
pub enum PluginLifecycleError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    PluginStartPanicked(PluginStartPanicked),
    #[error(transparent)]
    #[diagnostic(transparent)]
    PluginStartFailed(PluginStartFailed),
    #[error(transparent)]
    #[diagnostic(transparent)]
    PluginMainPanicked(PluginMainPanicked),
    #[error(transparent)]
    #[diagnostic(transparent)]
    PluginMainFailed(PluginMainFailed),
    #[error(transparent)]
    #[diagnostic(transparent)]
    PluginMessageHandlerPanicked(PluginMessageHandlerPanicked),
    #[error(transparent)]
    #[diagnostic(transparent)]
    PluginMessageHandlerFailed(PluginMessageHandlerFailed),
    #[error(transparent)]
    #[diagnostic(transparent)]
    PluginStopPanicked(PluginStopPanicked),
    #[error(transparent)]
    #[diagnostic(transparent)]
    PluginStopTimeout(PluginStopTimeout),
    #[error(transparent)]
    #[diagnostic(transparent)]
    PluginStopFailed(PluginStopFailed),
}

#[derive(miette::Diagnostic, Debug, thiserror::Error)]
#[error("Plugin '{name}' panicked while starting up")]
pub struct PluginStartPanicked {
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Plugin '{name}' encountered an error while starting up")]
pub struct PluginStartFailed {
    pub name: String,
    pub error: tedge_api::PluginError,
}

impl miette::Diagnostic for PluginStartFailed {
    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        Some(Box::new(std::iter::once(self.error.as_ref())))
    }
}

#[derive(miette::Diagnostic, Debug, thiserror::Error)]
#[error("Plugin '{name}' panicked while running main")]
pub struct PluginMainPanicked {
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Plugin '{name}' encountered an error while running main")]
pub struct PluginMainFailed {
    pub name: String,
    pub error: tedge_api::PluginError,
}

impl miette::Diagnostic for PluginMainFailed {
    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        Some(Box::new(std::iter::once(self.error.as_ref())))
    }
}

#[derive(miette::Diagnostic, Debug, thiserror::Error)]
#[error("Plugin '{name}' panicked while handling messages of type '{handled_message_type}'")]
pub struct PluginMessageHandlerPanicked {
    pub name: String,
    pub handled_message_type: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Plugin '{name}' panicked while starting up")]
pub struct PluginMessageHandlerFailed {
    pub name: String,
    pub handled_message_type: String,
    pub error: tedge_api::PluginError,
}

impl miette::Diagnostic for PluginMessageHandlerFailed {
    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        Some(Box::new(std::iter::once(self.error.as_ref())))
    }
}

#[derive(miette::Diagnostic, Debug, thiserror::Error)]
#[error("Plugin '{name}' panicked while stopping")]
pub struct PluginStopPanicked {
    pub name: String,
}

#[derive(miette::Diagnostic, Debug, thiserror::Error)]
#[error("Plugin '{name}' did not stop after a timeout of '{}'")]
pub struct PluginStopTimeout {
    pub name: String,
    pub timeout_duration: Duration,
}

#[derive(Debug, thiserror::Error)]
#[error("Plugin '{name}' encountered an error while stopping")]
pub struct PluginStopFailed {
    pub name: String,
    pub error: tedge_api::PluginError,
}

impl miette::Diagnostic for PluginStopFailed {
    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        Some(Box::new(std::iter::once(self.error.as_ref())))
    }
}
