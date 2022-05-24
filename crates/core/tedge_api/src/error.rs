use miette::Diagnostic;
use thiserror::Error;

/// Errors as orginating from [`Plugin`](crate::Plugin) and [`PluginBuilder`](crate::PluginBuilder)
pub type PluginError = Box<dyn Diagnostic + Send + Sync + 'static>;

#[derive(Error, Debug, Diagnostic)]
/// An error occured while interfacing with the [`PluginDirectory`](crate::plugin::PluginDirectory)
pub enum DirectoryError {
    /// The given plugin name does not exist in the configuration
    #[error("Plugin named '{}' not found", .0)]
    #[diagnostic(help("Please double check the name of your plugin is correct"))]
    PluginNameNotFound(String),

    /// The given plugin does not support all requested message types
    #[error("Plugin '{}' does not support the following message types: {}", .0 ,.1.join(","))]
    PluginDoesNotSupport(String, Vec<&'static str>),
}
