use futures::FutureExt;

use crate::config::OneOrMany;

/// An address of a plugin
#[derive(Debug, Eq, PartialEq, Hash, serde::Deserialize, tedge_api::Config)]
pub struct Address(String);

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
    ) -> Result<Self, tedge_api::error::DirectoryError>
    where
        PD: tedge_api::PluginDirectory,
    {
        match addrs {
            OneOrMany::One(addr) => Ok(AddressGroup(vec![addr.build(pd)?])),
            OneOrMany::Many(addrs) => addrs
                .iter()
                .map(|addr| addr.build(pd))
                .collect::<Result<Vec<_>, _>>()
                .map(AddressGroup),
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
