use std::{collections::HashMap, num::{NonZeroUsize, NonZeroU64}, path::{PathBuf, Path}};

use tedge_api::PluginBuilder;

use crate::communication::PluginDirectory;
use crate::errors::TedgeApplicationError;

#[derive(serde::Deserialize, Debug)]
pub struct TedgeConfiguration {
    communication_buffer_size: NonZeroUsize,
    plugin_shutdown_timeout_ms: NonZeroU64,
    plugins: HashMap<String, PluginInstanceConfiguration>,
}

#[derive(serde::Deserialize, Debug)]
pub struct PluginInstanceConfiguration {
    kind: PluginKind,
    configuration: InstanceConfiguration,
}

impl PluginInstanceConfiguration {
    pub fn kind(&self) -> &PluginKind {
        &self.kind
    }

    pub fn configuration(&self) -> &InstanceConfiguration {
        &self.configuration
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(transparent)]
pub struct PluginKind(String);

impl AsRef<str> for PluginKind {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl TedgeConfiguration {
    pub fn communication_buffer_size(&self) -> std::num::NonZeroUsize {
        self.communication_buffer_size
    }

    /// Get the tedge configuration's plugin shutdown timeout.
    pub fn plugin_shutdown_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.plugin_shutdown_timeout_ms.get())
    }

    pub fn plugins(&self) -> &HashMap<String, PluginInstanceConfiguration> {
        &self.plugins
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum InstanceConfiguration {
    ConfigFilePath(PathBuf),
    Config(tedge_api::PluginConfiguration),
}

impl InstanceConfiguration {
    pub async fn verify_with_builder(&self, builder: &Box<dyn PluginBuilder<PluginDirectory>>) -> crate::errors::Result<toml::Spanned<toml::Value>> {
        match self {
            InstanceConfiguration::Config(cfg) => {
                builder
                    .verify_configuration(&cfg)
                    .await
                    .map_err(TedgeApplicationError::from)
                    .map(|_| cfg.to_owned())
            },
            InstanceConfiguration::ConfigFilePath(path) => {
                async fn inner(builder: &Box<dyn PluginBuilder<PluginDirectory>>, path: &Path) -> crate::errors::Result<toml::Spanned<toml::Value>> {
                    let file_contents = tokio::fs::read_to_string(path).await
                        .map_err(|_| TedgeApplicationError::PluginConfigReadFailed(path.to_path_buf()))?;

                    let cfg = toml::from_str(&file_contents)?;

                    builder
                        .verify_configuration(&cfg)
                        .await
                        .map_err(TedgeApplicationError::from)
                        .map(|_| cfg)
                }

                inner(builder, &path).await
            }
        }
    }
}
