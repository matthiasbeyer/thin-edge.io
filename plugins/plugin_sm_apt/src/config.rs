#[derive(Debug, serde::Deserialize, tedge_api::Config)]
pub struct Config {
    /// Path to the "apt" binary.
    ///
    /// If not set "apt" from PATH will be used
    pub(crate) apt_binary: Option<String>,
}

