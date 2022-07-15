/// A helper type for specifying a port in the configuration
///
/// This type wraps an `u16` for specifying a port.
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Deserialize, serde::Serialize,
)]
#[serde(transparent)]
pub struct Port(u16);

impl From<Port> for u16 {
    fn from(p: Port) -> Self {
        p.0
    }
}

impl tedge_api::AsConfig for Port {
    fn as_config() -> tedge_api::ConfigDescription {
        tedge_api::ConfigDescription::new(
            "A Port number".to_string(),
            tedge_api::ConfigKind::Integer,
            Some("A number, representing a Port"),
        )
    }
}
