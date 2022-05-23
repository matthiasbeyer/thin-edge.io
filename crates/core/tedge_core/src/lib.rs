#![doc = include_str!("../README.md")]

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use miette::IntoDiagnostic;
use tedge_api::plugin::HandleTypes;
use tedge_api::PluginBuilder;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::debug_span;
use tracing::event;
use tracing::Instrument;
use tracing::Level;

mod communication;
pub mod configuration;
mod core_task;
pub mod errors;
mod plugin_task;
mod reactor;
mod utils;
mod message_handler;

pub use crate::communication::PluginDirectory;
use crate::configuration::PluginInstanceConfiguration;
use crate::configuration::TedgeConfiguration;
use crate::errors::Result;
use crate::errors::TedgeApplicationError;

/// A TedgeApplication
///
/// This is the main entry point for building a thin-edge application. It provides functions for
/// setting up the application and then run it.
///
/// # Details
///
/// This type implements only the setup functionality, how to construct an application object. The
/// implementation of the orchestration and lifecycle of the application is implemented in
/// [`crate::reactor::Reactor`]. Note that this is solely for code seperation.
pub struct TedgeApplication {
    config_path: PathBuf,
    config: TedgeConfiguration,
    cancellation_token: CancellationToken,
    plugin_builders: HashMap<String, (HandleTypes, Box<dyn PluginBuilder<PluginDirectory>>)>,
}

impl std::fmt::Debug for TedgeApplication {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("TedgeApplication").finish()
    }
}

impl TedgeApplication {
    /// Retrieve a [`TedgeApplicationBuilder`] object that can be used to construct a
    /// [`TedgeApplication`] object easily.
    pub fn builder() -> TedgeApplicationBuilder {
        TedgeApplicationBuilder {
            cancellation_token: CancellationToken::new(),
            plugin_builders: HashMap::new(),
        }
    }

    pub(crate) fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub(crate) fn config(&self) -> &TedgeConfiguration {
        &self.config
    }

    pub(crate) fn plugin_builders(
        &self,
    ) -> &HashMap<String, (HandleTypes, Box<dyn PluginBuilder<PluginDirectory>>)> {
        &self.plugin_builders
    }

    pub(crate) fn cancellation_token(&self) -> &CancellationToken {
        &self.cancellation_token
    }

    /// Run the TedgeApplication that has been setup for running
    ///
    /// This function runs as long as there is no shutdown signalled to the application.
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn run(self) -> Result<()> {
        // This `Reactor` type is only used to seperate the public-interface implementing parts of
        // this crate from the orchestration and lifecycle management code bits.
        crate::reactor::Reactor(self).run().await
    }

    /// Check whether all configured plugin kinds exist (are available in registered plugins)
    /// and that the configurations for the individual plugins are actually valid.
    pub async fn verify_configurations(&self) -> Vec<(String, Result<()>)> {
        use futures::stream::StreamExt;

        debug!("Verifying configurations");
        self.config()
            .plugins()
            .iter()
            .map(
                |(plugin_name, plugin_cfg): (&String, &PluginInstanceConfiguration)| {
                    async {
                        if let Some((_, builder)) =
                            self.plugin_builders().get(plugin_cfg.kind().as_ref())
                        {
                            debug!("Verifying {}", plugin_cfg.kind().as_ref());
                            let res = plugin_cfg
                                .configuration()
                                .verify_with_builder(builder, self.config_path())
                                .await
                                .into_diagnostic()
                                .map_err(TedgeApplicationError::PluginConfigVerificationFailed)
                                .map(|_| ());
                            (plugin_name.to_string(), res)
                        } else {
                            (
                                plugin_name.to_string(),
                                Err(TedgeApplicationError::UnknownPluginKind(
                                    plugin_cfg.kind().as_ref().to_string(),
                                )),
                            )
                        }
                    }
                    .instrument(debug_span!("verify configuration", plugin.name = %plugin_name))
                },
            )
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Vec<(String, Result<()>)>>()
            .await
    }
}

/// Helper type for constructing a [`TedgeApplication`]
pub struct TedgeApplicationBuilder {
    cancellation_token: CancellationToken,
    plugin_builders: HashMap<String, (HandleTypes, Box<dyn PluginBuilder<PluginDirectory>>)>,
}

impl TedgeApplicationBuilder {
    /// Register a [`tedge_api::PluginBuilder`]
    ///
    /// This function can be used to register a [`tedge_api::PluginBuilder`] within the
    /// [`TedgeApplication`] which is about to be built.
    ///
    /// Registering a [`PluginBuilder`] does not mean that a plugin from this builder will be
    /// running once the application starts up, but merely that the application _knows_ about this
    /// plugin builder and is able to construct a plugin with this builder, if necessary (e.g. if
    /// configured in a configuration file).
    pub fn with_plugin_builder<PB: PluginBuilder<PluginDirectory>>(
        mut self,
        builder: PB,
    ) -> Result<Self> {
        let handle_types = PB::kind_message_types();
        let kind_name = PB::kind_name();
        event!(
            Level::INFO,
            plugin.kind = kind_name,
            plugin.handled_types = ?handle_types,
            "Registered plugin builder"
        );

        if self.plugin_builders.contains_key(kind_name) {
            return Err(TedgeApplicationError::PluginKindExists(
                kind_name.to_string(),
            ));
        }

        self.plugin_builders
            .insert(kind_name.to_string(), (handle_types, Box::new(builder)));
        Ok(self)
    }

    /// Finalize the [`TedgeApplication`] by instantiating it with a `TedgeConfiguration`]
    ///
    /// This instantiates the application object, but does not run it.
    pub async fn with_config_from_path(
        self,
        config_path: impl AsRef<Path>,
    ) -> Result<(TedgeApplicationCancelSender, TedgeApplication)> {
        let config_path = config_path.as_ref();
        debug!(?config_path, "Loading config from path");

        let config_str = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(TedgeApplicationError::ConfigReadFailed)?;
        let config = toml::de::from_str(&config_str)?;
        let cancellation = TedgeApplicationCancelSender(self.cancellation_token.clone());
        let app = TedgeApplication {
            config_path: config_path.to_path_buf(),
            config,
            cancellation_token: self.cancellation_token,
            plugin_builders: self.plugin_builders,
        };

        Ok((cancellation, app))
    }

    /// Fetch the currently registered plugin kind names from the TedgeApplicationBuilder instance
    pub fn plugin_kind_names(&self) -> impl Iterator<Item = &str> {
        self.plugin_builders.keys().map(String::as_ref)
    }

    #[cfg(test)]
    pub fn plugin_builders(
        &self,
    ) -> &HashMap<String, (HandleTypes, Box<dyn PluginBuilder<PluginDirectory>>)> {
        &self.plugin_builders
    }
}

#[derive(Clone, Debug)]
pub struct TedgeApplicationCancelSender(CancellationToken);

impl TedgeApplicationCancelSender {
    pub fn cancel_app(&self) {
        debug!("Cancelling application");
        self.0.cancel()
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_deser_empty_plugin_config() {
        let s = "";
        let _: tedge_api::PluginConfiguration = toml::de::from_str(s).unwrap();
    }
}
