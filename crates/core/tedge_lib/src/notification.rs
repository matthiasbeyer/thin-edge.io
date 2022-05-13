#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Notification {
    Info { message: String },

    Warning { message: String },

    Error { message: String },
}

impl Notification {
    pub const fn info(message: String) -> Self {
        Self::Info { message }
    }
    pub const fn warning(message: String) -> Self {
        Self::Warning { message }
    }
    pub const fn error(message: String) -> Self {
        Self::Error { message }
    }
}

impl tedge_api::plugin::Message for Notification {}
