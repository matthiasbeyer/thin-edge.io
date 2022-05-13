#[derive(serde::Deserialize, Debug, tedge_api::Config)]
pub struct CollectdConfig {
    /// Name of the plugin to send measurements received from collectd to
    pub target: String,
}

