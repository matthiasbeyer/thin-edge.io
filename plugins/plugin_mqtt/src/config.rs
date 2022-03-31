#[derive(serde::Deserialize, Debug)]
pub struct MqttConfig {
    /// MQTT host to connect to
    ///
    /// Default: "tpc://localhost:1883"
    #[serde(default = "default_host")]
    pub host: String,

    /// The list of topics to subscribe to on connect
    ///
    /// Default: An empty topic list
    pub subscriptions: Vec<Subscription>,

    /// Name of the plugin to send messages to
    pub target: String,
}

fn default_host() -> String {
    "tcp://localhost:1883".to_string()
}


#[derive(Debug, serde::Deserialize)]
pub struct Subscription {
    pub(crate) topic: String,
    pub(crate) qos: QoS,
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

impl Into<i32> for QoS {
    fn into(self) -> i32 {
        match self {
            QoS::AtMostOnce => paho_mqtt::QOS_0,
            QoS::AtLeastOnce => paho_mqtt::QOS_1,
            QoS::ExactlyOnce => paho_mqtt::QOS_2,
        }
    }
}

impl TryFrom<i32> for QoS {
    type Error = tedge_api::PluginError;

    fn try_from(i: i32) -> Result<Self, Self::Error> {
        match i {
            paho_mqtt::QOS_0 => Ok(QoS::AtMostOnce),
            paho_mqtt::QOS_1 => Ok(QoS::AtLeastOnce),
            paho_mqtt::QOS_2 => Ok(QoS::ExactlyOnce),
            _ => Err(tedge_api::PluginError::from(anyhow::anyhow!("Failed to interpret '{}' as QOS", i)))
        }
    }
}

