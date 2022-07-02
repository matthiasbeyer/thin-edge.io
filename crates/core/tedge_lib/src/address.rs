//! Helper types for working with addresses

use futures::StreamExt;

use crate::config::OneOrMany;

/// A collection of addresses
///
/// This type implements helper functionality for a list of addresses.
/// If the user can specify one or many addresses (via the
/// [OneOrMany](crate::config::OneOrMany) config helper type)
/// they can be collected into an [AddressGroup](crate::address::AddressGroup)
/// and be used together with some convenience for sending to all addresses at once.
#[derive(Debug)]
pub struct AddressGroup<RB: tedge_api::address::ReceiverBundle>(Vec<tedge_api::Address<RB>>);

impl<RB> AddressGroup<RB>
where
    RB: tedge_api::address::ReceiverBundle,
{
    /// Build an [AddressGroup] from [OneOrMany](crate::config::OneOrMany)
    /// [Address](crate::config::Address)es.
    ///
    /// This function fails with [AddressGroupBuildError] containing all errors that were collected
    /// when asking the [PluginDirectory](tedge_api::PluginDirectory::get_address_for) (via
    /// [Address::build](crate::config::Address::build)) for the [Address](tedge_api::Address)
    /// object.
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

    /// Send a [Message](tedge_api::Message) to all addresses in the group
    ///
    /// This function internally uses [Address::send_and_wait](tedge_api::Address::send_and_wait).
    ///
    /// # Note
    ///
    /// This function returns a stream that must be consumed to actually send the messages out.
    ///
    /// The caller of this function can, using the returned stream, decide whether the messages
    /// should be sent out concurrently or in sequence.
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

    /// Try to send a [Message](tedge_api::Message) to all addresses in the group
    ///
    /// This function internally uses [Address::try_send](tedge_api::Address::try_send).
    ///
    /// # Note
    ///
    /// This function returns a stream that must be consumed to actually send the messages out.
    ///
    /// The caller of this function can, using the returned stream, decide whether the messages
    /// should be sent out concurrently or in sequence.
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
