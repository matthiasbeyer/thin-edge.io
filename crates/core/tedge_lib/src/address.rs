//! Helper types for working with addresses

use futures::StreamExt;

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
    ) -> impl futures::stream::Stream<Item = Result<tedge_api::address::ReplyReceiverFor<M>, M>> + '_
    where
        M: tedge_api::Message + Clone,
        RB: tedge_api::address::Contains<M>,
    {
        futures::stream::iter(&self.0).then(move |addr| addr.send_and_wait(msg.clone()))
    }

    pub fn try_send<M>(
        &self,
        msg: M,
    ) -> impl futures::stream::Stream<Item = Result<tedge_api::address::ReplyReceiverFor<M>, M>> + '_
    where
        M: tedge_api::Message + Clone,
        RB: tedge_api::address::Contains<M>,
    {
        futures::stream::iter(&self.0).then(move |addr| addr.try_send(msg.clone()))
    }
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("Directory errors were encountered")]
pub struct AddressGroupBuildError {
    #[related]
    errors: Vec<tedge_api::error::DirectoryError>,
}
