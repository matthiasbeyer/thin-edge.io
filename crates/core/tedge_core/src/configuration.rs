use std::collections::HashMap;

#[derive(serde::Deserialize, Debug)]
pub struct TedgeConfiguration {
    communication_buffer_size: std::num::NonZeroUsize,
    plugins: HashMap<String, PluginInstanceConfiguration>,
}

#[derive(serde::Deserialize, Debug)]
pub struct PluginInstanceConfiguration {
    kind: PluginKind,
    configuration: tedge_api::PluginConfiguration,
}

impl PluginInstanceConfiguration {
    pub fn kind(&self) -> &PluginKind {
        &self.kind
    }

    pub fn configuration(&self) -> &tedge_api::PluginConfiguration {
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

    pub fn plugins(&self) -> &HashMap<String, PluginInstanceConfiguration> {
        &self.plugins
    }
}
