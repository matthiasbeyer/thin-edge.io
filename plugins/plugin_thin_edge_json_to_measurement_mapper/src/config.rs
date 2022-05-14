#[derive(Debug, serde::Deserialize, tedge_api::Config)]
pub struct ThinEdgeJsonToMeasurementMapperConfig {
    /// The name of the plugin to send the parsed ThinEdgeJsonToMeasurementMapperMessage to
    target: String,
}

impl ThinEdgeJsonToMeasurementMapperConfig {
    pub(crate) fn target(&self) -> &str {
        &self.target
    }
}
