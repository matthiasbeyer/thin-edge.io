#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Failed to parse configuration")]
    ConfigParseFailed(toml::de::Error),

    #[error("Failed to parse JSON payload")]
    DeserError(serde_json::Error),

    #[error("Failed to serialize JSON payload")]
    SerError(serde_json::Error),

    #[error("Error while waiting for reply")]
    ReplyError(#[from] tedge_api::address::ReplyError),

    #[error("Failed to send message")]
    SendFailed,
}

