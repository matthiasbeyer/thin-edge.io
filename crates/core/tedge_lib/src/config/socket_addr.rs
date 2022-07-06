/// A helper type for specifying an IP address in the configuartion
///
/// This type wraps a [std::net::SocketAddr](std::net::SocketAddr) for specifying an IP address in
/// the configuration. It implements [tedge_api::AsConfig](tedge_api::AsConfig) for convenient use.
#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialOrd, PartialEq, serde::Deserialize, serde::Serialize,
)]
pub struct SocketAddr(std::net::SocketAddr);

impl From<SocketAddr> for std::net::SocketAddr {
    fn from(sa: SocketAddr) -> Self {
        sa.0
    }
}

impl tedge_api::AsConfig for SocketAddr {
    fn as_config() -> tedge_api::ConfigDescription {
        tedge_api::ConfigDescription::new(
            "A socket address".to_string(),
            tedge_api::ConfigKind::String,
            Some(indoc::indoc! {r#"
                A String that represents a socket address

                ## Examples

                A socket address can either be an IPv4 address:

                ```toml
                "127.0.0.1"
                ```

                Or an IPv6 address:

                ```toml
                "::1"
                ```
            "#}),
        )
    }
}
