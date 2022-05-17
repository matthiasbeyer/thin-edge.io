#[derive(serde::Deserialize, Debug, tedge_api::Config)]
pub struct C8yConfig {
    /// Name of the plugin that handles software-management operations
    pub(crate) sm_plugin_name: String,
}

