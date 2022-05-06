use std::{
    collections::HashMap,
    num::{NonZeroU64, NonZeroUsize},
    path::{Path, PathBuf},
};

use tedge_api::PluginBuilder;
use tracing::debug;

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

    #[serde(flatten)]
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
#[cfg_attr(test, derive(PartialEq))]
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
#[cfg_attr(test, derive(PartialEq))]
pub enum InstanceConfiguration {
    #[serde(rename = "configuration_file")]
    ConfigFilePath(PathBuf),

    #[serde(rename = "configuration")]
    Config(tedge_api::PluginConfiguration),
}

impl InstanceConfiguration {
    pub async fn verify_with_builder(
        &self,
        builder: &Box<dyn PluginBuilder<PluginDirectory>>,
        root_config_path: &Path,
    ) -> crate::errors::Result<toml::Value> {
        match self {
            InstanceConfiguration::Config(cfg) => builder
                .verify_configuration(&cfg)
                .await
                .map_err(TedgeApplicationError::PluginConfigVerificationFailed)
                .map(|_| cfg.to_owned()),
            InstanceConfiguration::ConfigFilePath(path) => {
                async fn inner(
                    builder: &Box<dyn PluginBuilder<PluginDirectory>>,
                    root_config_path: &Path,
                    path: &Path,
                ) -> crate::errors::Result<toml::Value> {
                    let file_path = root_config_path
                        .parent()
                        .ok_or_else(|| {
                            TedgeApplicationError::PathNotAFilePath(root_config_path.to_path_buf())
                        })?
                        .join(path);

                    debug!("Reading config file: {}", file_path.display());
                    let file_contents =
                        tokio::fs::read_to_string(file_path).await.map_err(|_| {
                            TedgeApplicationError::PluginConfigReadFailed(path.to_path_buf())
                        })?;

                    let cfg = toml::from_str(&file_contents)?;

                    builder
                        .verify_configuration(&cfg)
                        .await
                        .map_err(TedgeApplicationError::PluginConfigVerificationFailed)
                        .map(|_| cfg)
                }

                inner(builder, root_config_path, &path).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_plugin_instance_config_path() {
        let s = r#"
            kind = "foo"
            configuration_file = "path/to/file.toml"
        "#;

        let c: PluginInstanceConfiguration = toml::from_str(s).unwrap();
        assert_eq!(c.kind, PluginKind("foo".to_string()));
        assert_eq!(c.configuration, InstanceConfiguration::ConfigFilePath(PathBuf::from("path/to/file.toml")));
    }

    #[test]
    fn test_deserialize_plugin_instance_config_table() {
        let s = r#"
            kind = "foo"
            [configuration]
        "#;

        let c: PluginInstanceConfiguration = toml::from_str(s).unwrap();
        assert_eq!(c.kind, PluginKind("foo".to_string()));
        assert!(std::matches!(c.configuration, InstanceConfiguration::Config(_)));
    }
}

