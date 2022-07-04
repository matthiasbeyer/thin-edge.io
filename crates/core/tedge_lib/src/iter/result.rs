/// A Result helper type that can be collected
///
/// ```no_run
/// # async {
/// #     #[derive(Clone, Debug, bevy_reflect::TypeUuid)]
/// #     #[uuid = "e9457c98-e956-4403-89ff-33635bf18ee3"]
/// #     struct M;
/// #     impl tedge_api::Message for M {}
/// #     tedge_api::make_receiver_bundle!(struct MessageReceiver(M));
/// #
/// #     struct PD;
/// #     impl tedge_api::PluginDirectory for PD {
/// #         fn get_address_for<RB: tedge_api::address::ReceiverBundle>(
/// #             &self,
/// #             name: &str,
/// #         ) -> Result<tedge_api::Address<RB>, tedge_api::error::DirectoryError> {
/// #             unimplemented!()
/// #         }
/// #         fn get_address_for_core(&self) -> tedge_api::Address<tedge_api::message::CoreMessages> {
/// #             unimplemented!()
/// #         }
/// #         fn get_address_for_self<RB: tedge_api::address::ReceiverBundle>(&self) -> Result<tedge_api::Address<RB>, tedge_api::error::DirectoryError> {
/// #             unimplemented!()
/// #         }
/// #     }
/// #
/// #     let plugin_dir: PD = { unimplemented!() };
/// #     let plugin_dir = &plugin_dir;
/// #
/// // The configuration of the plugin contains a number of addresses to send to
/// #[derive(serde::Deserialize)]
/// struct Config {
///     addresses: tedge_lib::config::OneOrMany<tedge_lib::config::Address>,
/// }
/// #
/// #     let config = r#"
/// #         addresses = ["foo"]
/// #     "#;
/// #
/// #    let config: Config = toml::from_str(config).unwrap();
///
/// use tedge_lib::address::AddressGroup;
/// use futures::stream::StreamExt;
///
/// // The AddressGroup type is built using the plugin directory and the configured addresses
/// let addrs: AddressGroup<MessageReceiver> = AddressGroup::build(plugin_dir, &config.addresses).unwrap();
///
/// // Messages are send using the AddressGroup::send_and_wait() interface, which sends to all
/// // addresses
/// #    let message = M;
/// let results = addrs.send_and_wait(message)
///     .collect::<tedge_lib::iter::SendAllResult<M>>()
///     .await;
///
/// // "results" now contains all successful results as well as all errors
/// // and these can be processed as required
/// # };
/// ```
pub struct SendAllResult<M: tedge_api::Message> {
    oks: Vec<tedge_api::address::ReplyReceiverFor<M>>,
    errs: Vec<M>,
}

impl<M> Default for SendAllResult<M>
where
    M: tedge_api::Message,
{
    fn default() -> Self {
        SendAllResult {
            oks: Vec::new(),
            errs: Vec::new(),
        }
    }
}

impl<M> Extend<Result<tedge_api::address::ReplyReceiverFor<M>, M>> for SendAllResult<M>
where
    M: tedge_api::Message,
{
    fn extend<T: IntoIterator<Item = Result<tedge_api::address::ReplyReceiverFor<M>, M>>>(
        &mut self,
        iter: T,
    ) {
        for elem in iter {
            match elem {
                Ok(ok) => self.oks.push(ok),
                Err(e) => self.errs.push(e),
            }
        }
    }
}

impl<M> SendAllResult<M>
where
    M: tedge_api::Message,
{
    pub fn oks(&self) -> &[tedge_api::address::ReplyReceiverFor<M>] {
        &self.oks
    }

    pub fn into_oks(self) -> Vec<tedge_api::address::ReplyReceiverFor<M>> {
        self.oks
    }

    pub fn errs(&self) -> &[M] {
        &self.errs
    }

    pub fn into_errs(self) -> Vec<M> {
        self.errs
    }

    pub fn into_result(self) -> Result<Vec<tedge_api::address::ReplyReceiverFor<M>>, Vec<M>> {
        if !self.errs().is_empty() {
            Ok(self.into_oks())
        } else {
            Err(self.into_errs())
        }
    }
}
