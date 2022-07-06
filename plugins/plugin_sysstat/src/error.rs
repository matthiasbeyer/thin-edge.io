#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Failed to parse configuration")]
    ConfigParseFailed(toml::de::Error),

    #[error("Failed to send measurement")]
    FailedToSendMeasurement,

    #[error("Cannot read disk name")]
    CannotReadDiskName,

    #[error("Not valid UTF-8")]
    Utf8Error(#[from] std::str::Utf8Error),
}
