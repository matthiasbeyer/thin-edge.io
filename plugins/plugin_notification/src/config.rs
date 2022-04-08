use tedge_lib::notification::Notification;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub(crate) forward_to: String,

    pub(crate) notify: String,

    pub(crate) raise: NotificationType,
    pub(crate) raise_message: String,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub enum NotificationType {
    #[serde(rename = "info")]
    Info,

    #[serde(rename = "warning")]
    Warning,

    #[serde(rename = "error")]
    Error,
}

impl NotificationType {
    pub(crate) fn into_notification(self, message: String) -> Notification {
        match self {
            NotificationType::Info => Notification::info(message),
            NotificationType::Warning => Notification::warning(message),
            NotificationType::Error => Notification::error(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_valid() {
        let config = r#"
            forward_to = "foo"
            notify = "bar"
            raise = "info"
            raise_message = "it is getting warm here"
        "#;

        let cfg: Config = toml::from_str(config).unwrap();
        assert_eq!(cfg.forward_to, "foo");
        assert_eq!(cfg.notify, "bar");
        assert_eq!(cfg.raise, NotificationType::Info);
        assert_eq!(cfg.raise_message, String::from("it is getting warm here"));
    }
}

