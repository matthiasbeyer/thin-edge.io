#[derive(Debug, serde::Deserialize, tedge_api::Config)]
pub struct Config {
    /// The name of the plugin to send SM requests to
    pub(crate) target: String,

    /// The name of the mqtt plugin to send SM responses to
    pub(crate) mqtt_plugin_addr: String,

    /// The MQTT topic to send responses to
    pub(crate) result_topic: String,
}

