#![doc = include_str!("../README.md")]

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use itertools::Itertools;
use tedge_api::plugin::HandleTypes;
use tedge_api::PluginBuilder;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use tracing::event;

use tracing::Level;

mod communication;
pub mod configuration;
mod core_task;
pub mod errors;
mod message_handler;
mod plugin_task;
mod reactor;
mod utils;

pub use crate::communication::PluginDirectory;
use crate::configuration::PluginInstanceConfiguration;
use crate::configuration::TedgeConfiguration;

use crate::errors::PluginConfigurationError;
use crate::errors::PluginKindUnknownError;
use crate::errors::TedgeApplicationBuilderError;
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
            errors: vec![],
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
    ///
    /// # Note
    ///
    /// This function makes sure that the configuration is verified before the plugins are started.
    /// So there is no need to call [TedgeApplication::verify_configuration] before this.
    pub async fn run(self) -> Result<(), TedgeApplicationError> {
        crate::reactor::Reactor(self).run().await
    }

    /// Check whether all configured plugin kinds exist (are available in registered plugins)
    /// and that the configurations for the individual plugins are actually valid.
    #[tracing::instrument(skip(self))]
    pub async fn verify_configurations(&self) -> Result<(), TedgeApplicationError> {
        use futures::stream::StreamExt;

        debug!("Verifying configurations");
        let results = self
            .config()
            .plugins()
            .iter()
            .map(
                |(plugin_name, plugin_cfg): (&String, &PluginInstanceConfiguration)| {
                    let plugin_name = plugin_name.to_string();
                    async move {
                        if let Some((_, builder)) =
                            self.plugin_builders().get(plugin_cfg.kind().as_ref())
                        {
                            debug!("Verifying {}", plugin_cfg.kind().as_ref());
                            let res = plugin_cfg
                                .configuration()
                                .verify_with_builder(&plugin_name, builder, self.config_path())
                                .await;

                            Ok(res?)
                        } else {
                            Err(PluginConfigurationError::UnknownKind(
                                PluginKindUnknownError {
                                    name: plugin_cfg.kind().as_ref().to_string(),
                                    alternatives: None,
                                },
                            ))
                        }
                    }
                },
            )
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Vec<Result<_, _>>>()
            .await;

        let (_oks, errors): (Vec<_>, Vec<_>) = results.into_iter().partition_result();

        if !errors.is_empty() {
            return Err(TedgeApplicationError::PluginConfigVerificationsError { errors });
        }

        Ok(())
    }
}

/// Helper type for constructing a [`TedgeApplication`]
pub struct TedgeApplicationBuilder {
    cancellation_token: CancellationToken,
    plugin_builders: HashMap<String, (HandleTypes, Box<dyn PluginBuilder<PluginDirectory>>)>,
    errors: Vec<TedgeApplicationBuilderError>,
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
    pub fn with_plugin_builder<PB: PluginBuilder<PluginDirectory>>(mut self, builder: PB) -> Self {
        let handle_types = PB::kind_message_types();
        let kind_name = PB::kind_name();
        event!(
            Level::INFO,
            plugin.kind = kind_name,
            plugin.handled_types = ?handle_types,
            "Registered plugin builder"
        );

        if self.plugin_builders.contains_key(kind_name) {
            self.errors
                .push(TedgeApplicationBuilderError::DuplicateKind {
                    name: kind_name.to_string(),
                    builder_name: std::any::type_name::<PB>(),
                });
            return self;
        }

        self.plugin_builders
            .insert(kind_name.to_string(), (handle_types, Box::new(builder)));
        self
    }

    /// Finalize the [`TedgeApplication`] by instantiating it with a `TedgeConfiguration`]
    ///
    /// This instantiates the application object, but does not run it.
    pub async fn with_config_from_path(
        self,
        config_path: impl AsRef<Path>,
    ) -> Result<(TedgeApplicationCancelSender, TedgeApplication), TedgeApplicationError> {
        if !self.errors.is_empty() {
            return Err(TedgeApplicationError::ApplicationBuilderErrors {
                errors: self.errors,
            });
        }
        let config_path = config_path.as_ref();
        debug!(?config_path, "Loading config from path");

        let config_str = tokio::fs::read_to_string(&config_path).await.map_err(|e| {
            TedgeApplicationError::ApplicationBuilderErrors {
                errors: vec![TedgeApplicationBuilderError::PathNotReadable {
                    path: config_path.to_path_buf(),
                    error: e,
                }],
            }
        })?;
        let config = toml::de::from_str(&config_str).map_err(|e| {
            TedgeApplicationError::ApplicationBuilderErrors {
                errors: vec![TedgeApplicationBuilderError::ConfigNotParseable {
                    path: config_path.to_path_buf(),
                    error: e,
                }],
            }
        })?;
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
