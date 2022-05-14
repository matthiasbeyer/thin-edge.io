#[derive(Debug, serde::Deserialize, tedge_api::Config)]
pub struct ThinEdgeJsonConfig {
    /// The name of the plugin to send the parsed ThinEdgeJsonMessage to
    target: String,
}

impl ThinEdgeJsonConfig {
    pub(crate) fn target(&self) -> &str {
        &self.target
    }
}
