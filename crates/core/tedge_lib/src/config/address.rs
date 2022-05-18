use futures::FutureExt;

use crate::config::OneOrMany;

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

pub struct AddressGroup<RB: tedge_api::address::ReceiverBundle>(Vec<tedge_api::Address<RB>>);

impl<RB> AddressGroup<RB>
where
    RB: tedge_api::address::ReceiverBundle,
{
    pub fn build<PD>(
        pd: &PD,
        addrs: &OneOrMany<crate::config::Address>,
    ) -> Result<Self, AddressGroupBuildError>
    where
        PD: tedge_api::PluginDirectory,
    {
        match addrs {
            OneOrMany::One(addr) => addr
                .build(pd)
                .map(|a| vec![a])
                .map(AddressGroup)
                .map_err(|e| AddressGroupBuildError { errors: vec![e] }),
            OneOrMany::Many(addrs) => {
                use itertools::Itertools;

                let (oks, errs): (
                    Vec<tedge_api::Address<RB>>,
                    Vec<tedge_api::error::DirectoryError>,
                ) = addrs.iter().map(|addr| addr.build(pd)).partition_result();

                if !errs.is_empty() {
                    Err(AddressGroupBuildError { errors: errs })
                } else {
                    Ok(AddressGroup(oks))
                }
            }
        }
    }

    pub fn send_and_wait<M>(
        &self,
        msg: M,
    ) -> impl Iterator<
        Item = futures::future::BoxFuture<Result<tedge_api::address::ReplyReceiverFor<M>, M>>,
    >
    where
        M: tedge_api::Message + Clone,
        RB: tedge_api::address::Contains<M>,
    {
        self.0
            .iter()
            .map(move |addr| addr.send_and_wait(msg.clone()).boxed())
    }

    pub fn try_send<M>(
        &self,
        msg: M,
    ) -> impl Iterator<Item = Result<tedge_api::address::ReplyReceiverFor<M>, M>> + '_
    where
        M: tedge_api::Message + Clone,
        RB: tedge_api::address::Contains<M>,
    {
        self.0.iter().map(move |addr| addr.try_send(msg.clone()))
    }
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("Directory errors were encountered")]
pub struct AddressGroupBuildError {
    #[related]
    errors: Vec<tedge_api::error::DirectoryError>,
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
