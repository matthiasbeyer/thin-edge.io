#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Failed to parse configuration")]
    ConfigParseFailed(toml::de::Error),

    #[error("Failed to publish message")]
    FailedToPublish(paho_mqtt::errors::Error),

    #[error("No client, cannot send messages")]
    NoClient,

    #[error("Failed to stop MQTT mainloop")]
    FailedToStopMqttMainloop,

    #[error("Failed to disconnect MQTT client")]
    FailedToDisconnectMqttClient(paho_mqtt::errors::Error),
}

