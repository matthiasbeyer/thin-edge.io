#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Failed to parse configuration")]
    ConfigParseFailed(toml::de::Error),

    #[error("Parsing bytes as UTF8 String failed")]
    ParseUtf8Error(#[from] std::str::Utf8Error),

    #[error("Failed to parse ThinEdgeJson")]
    ThinEdgeJsonParserError(#[from] thin_edge_json::parser::ThinEdgeJsonParserError),

    #[error("Failed to build ThinEdgeJson")]
    ThinEdgeJsonBuilderError(#[from] thin_edge_json::builder::ThinEdgeJsonBuilderError),

    #[error("Failed to send ThinEdgeJson")]
    FailedToSend,
}
