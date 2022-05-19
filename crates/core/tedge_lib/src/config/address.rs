/// An address of a plugin
#[derive(Debug, Eq, PartialEq, Hash, serde::Deserialize, tedge_api::Config)]
#[serde(transparent)]
pub struct Address(String);

impl AsRef<str> for Address {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Address {
    pub fn build<RB, PD>(
        &self,
        pd: &PD,
    ) -> Result<tedge_api::Address<RB>, tedge_api::error::DirectoryError>
    where
        RB: tedge_api::address::ReceiverBundle,
        PD: tedge_api::PluginDirectory,
    {
        pd.get_address_for(&self.0)
    }
}

#[cfg(test)]
mod tests {
    #[derive(serde::Deserialize)]
    struct A {
        addr: super::Address,
    }

    #[derive(serde::Deserialize)]
    struct G {
        addrs: crate::config::OneOrMany<super::Address>,
    }

    #[test]
    fn test_deser() {
        let s = r#"
            addr = "foo"
        "#;

        let a: A = toml::from_str(s).unwrap();
        assert_eq!(a.addr.0, "foo");
    }

    #[test]
    fn test_deser_group() {
        let s = r#"
            addrs = [ "foo", "bar", "baz" ]
        "#;

        let g: G = toml::from_str(s).unwrap();
        let addrs = g.addrs.into_vec();
        assert_eq!(addrs.get(0).map(AsRef::as_ref), Some("foo"));
        assert_eq!(addrs.get(1).map(AsRef::as_ref), Some("bar"));
        assert_eq!(addrs.get(2).map(AsRef::as_ref), Some("baz"));
        assert_eq!(addrs.get(3), None);
    }
}
