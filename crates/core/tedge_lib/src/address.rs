//! Helper types for working with addresses

use futures::FutureExt;

use crate::config::OneOrMany;

#[derive(Debug)]
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
