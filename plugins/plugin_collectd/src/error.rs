#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum Error {
    #[error("Failed to parse configuration")]
    ConfigParseFailed(toml::de::Error),

    #[error("Failed to send Measurement")]
    FailedToSendMeasurement,

    #[error("Collectd topic name is invalid: {}", .0)]
    InvalidCollectdTopicName(String),

    #[error("Invalid payload: {0}. Expected payload format: <timestamp>:<value>")]
    InvalidMeasurementPayloadFormat(String),

    #[error("Invalid measurement timestamp: {0}. Epoch time value expected")]
    InvalidMeasurementTimestamp(String),

    #[error("Invalid measurement value: {0}. Must be a number")]
    InvalidMeasurementValue(String),

    #[error("Message payload UTF8 parsing error")]
    MessagePayloadNotUtf8(std::str::Utf8Error),
}

