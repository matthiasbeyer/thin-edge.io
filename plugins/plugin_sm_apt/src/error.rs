#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub(crate) enum Error {
    #[error("Failed to parse configuration")]
    ConfigParseFailed(toml::de::Error),
}

