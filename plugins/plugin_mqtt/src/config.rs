#[derive(serde::Deserialize, Debug, tedge_api::Config)]
pub struct MqttConfig {
    /// MQTT host to connect to
    pub host: String,

    /// The list of topics to subscribe to on connect
    ///
    /// Default: An empty topic list
    pub subscriptions: Vec<Subscription>,

    /// Name of the plugin to send messages to
    pub target: tedge_lib::config::Address,
}

#[derive(Debug, serde::Deserialize, tedge_api::Config)]
pub struct Subscription {
    /// The topic to connect to
    pub(crate) topic: String,

    /// The Quality of Service to use for the subscribed topic
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

impl tedge_api::AsConfig for QoS {
    fn as_config() -> tedge_api::ConfigDescription {
        tedge_api::ConfigDescription::new(
            "Qos".to_string(),
            tedge_api::ConfigKind::Enum(
                tedge_api::config::ConfigEnumKind::Untagged,
                vec![
                    (
                        "String",
                        Some("QOS 0"),
                        tedge_api::config::EnumVariantRepresentation::String("at_most_once"),
                    ),
                    (
                        "String",
                        Some("QOS 1"),
                        tedge_api::config::EnumVariantRepresentation::String("at_least_once"),
                    ),
                    (
                        "String",
                        Some("QOS 2"),
                        tedge_api::config::EnumVariantRepresentation::String("exactly_once"),
                    ),
                ],
            ),
            None,
        )
    }
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
            _ => Err(miette::miette!("Failed to interpret '{}' as QOS", i)),
        }
    }
}
