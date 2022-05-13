use tedge_lib::notification::Notification;

#[derive(Debug, serde::Deserialize, tedge_api::Config)]
pub struct Config {
    /// The name of the plugin to forward messages to
    pub(crate) forward_to: String,

    /// The name of the plugin to send notifications to
    pub(crate) notify: String,

    /// The type of the notification to raise
    pub(crate) raise: NotificationType,

    /// The message to attach to the notification
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

impl tedge_api::AsConfig for NotificationType {
    fn as_config() -> tedge_api::ConfigDescription {
        tedge_api::ConfigDescription::new(
            "NotificationType".to_string(),
            tedge_api::ConfigKind::Enum(
                tedge_api::config::ConfigEnumKind::Untagged,
                vec![
                    (
                        "String",
                        Some("Set the notification level to 'info'"),
                        tedge_api::config::EnumVariantRepresentation::String("info"),
                    ),
                    (
                        "String",
                        Some("Set the notification level to 'warning'"),
                        tedge_api::config::EnumVariantRepresentation::String("warning"),
                    ),
                    (
                        "String",
                        Some("Set the notification level to 'error'"),
                        tedge_api::config::EnumVariantRepresentation::String("error"),
                    ),
                ],
            ),
            None,
        )
    }
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
