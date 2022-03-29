#[derive(serde::Deserialize, Debug)]
pub struct MqttConfig {
    /// MQTT host to connect to
    ///
    /// Default: "localhost"
    #[serde(default = "default_host")]
    pub host: String,

    /// MQTT port to connect to
    ///
    /// Default: 1883
    #[serde(default = "default_port")]
    pub port: u16,

    /// The session name to be use on connect
    ///
    /// If no session name is provided, a random one will be created on connect.
    ///
    /// Default: None
    pub session_name: Option<String>,

    /// The list of topics to subscribe to on connect
    ///
    /// Default: An empty topic list
    #[serde(default)]
    pub subscriptions: TopicFilter,

    /// Clean the MQTT session upon connect if set to `true`.
    ///
    /// Default: `false`.
    #[serde(default = "clean_session_default")]
    pub clean_session: bool,

    /// Capacity of the internal message queues
    ///
    /// Default: `1024`.
    ///
    #[serde(default = "queue_capacity_default")]
    pub queue_capacity: usize,

    /// Maximum size for a message payload
    ///
    /// Default: `1024 * 1024`.
    #[serde(default = "max_packet_size_default")]
    pub max_packet_size: usize,

    pub topic: String,
    pub qos: QoS,
    pub retain: bool,
}

fn default_host() -> String {
    "localhost".to_string()
}

fn default_port() -> u16 {
    1883
}

fn clean_session_default() -> bool {
    false
}

fn queue_capacity_default() -> usize {
    1024
}

fn max_packet_size_default() -> usize {
    1024 * 1024
}


/// An MQTT topic filter
#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize)]
pub struct TopicFilter {
    pub patterns: Vec<String>,
    pub qos: QoS,
}

impl Default for TopicFilter {
    fn default() -> Self {
        Self {
            patterns: Vec::new(),
            qos: QoS::AtMostOnce,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, serde::Deserialize)]
pub enum QoS {
    #[serde(rename = "at_most_once")]
    AtMostOnce,

    #[serde(rename = "at_least_once")]
    AtLeastOnce,

    #[serde(rename = "exactly_once")]
    ExactlyOnce,
}

impl Into<rumqttc::QoS> for QoS {
    fn into(self) -> rumqttc::QoS {
        match self {
            QoS::AtMostOnce => rumqttc::QoS::AtMostOnce,
            QoS::AtLeastOnce => rumqttc::QoS::AtLeastOnce,
            QoS::ExactlyOnce => rumqttc::QoS::ExactlyOnce,
        }
    }
}

