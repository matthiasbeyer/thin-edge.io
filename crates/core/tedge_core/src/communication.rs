use std::collections::HashMap;
use std::sync::Arc;

use tedge_api::address::MessageSender;
use tedge_api::error::DirectoryError;
use tedge_api::message::MessageType;
use tedge_api::plugin::PluginDirectory as ApiPluginDirectory;
use tedge_api::Address;
use tokio::sync::RwLock;

use crate::errors::TedgeApplicationError;

/// Type for taking care of addresses
///
/// In a way, this is the address book type of thinedge. This associates a name of a plugin
/// instance with an [`tedge_api::Address`] object that can be used to send messages to this plugin
/// instance.
pub struct CorePluginDirectory {
    plugins: HashMap<String, PluginInfo>,
    core_communicator: MessageSender,
}

impl CorePluginDirectory {
    pub(crate) fn collect_from<I>(iter: I) -> Result<Self, TedgeApplicationError>
    where
        I: std::iter::IntoIterator<Item = Result<(String, PluginInfo), TedgeApplicationError>>,
    {
        Ok(CorePluginDirectory {
            plugins: iter.into_iter().collect::<Result<HashMap<_, _>, _>>()?,
            core_communicator: MessageSender::new(Default::default()),
        })
    }

    pub(crate) fn get_mut<S: AsRef<str>>(&mut self, name: S) -> Option<&mut PluginInfo> {
        self.plugins.get_mut(name.as_ref())
    }

    /// Get the address of a plugin named `name`, which must be able to receive messages of the
    /// types specified in `MB`.
    fn get_address_for<MB: tedge_api::address::ReceiverBundle>(
        &self,
        name: &str,
    ) -> Result<Address<MB>, DirectoryError> {
        // list of types that is requested to be supported by the address
        let types = MB::get_ids().into_iter().collect::<Vec<_>>();

        // list of types the plugin supports
        let plug = self
            .plugins
            .get(name)
            .ok_or_else(|| DirectoryError::PluginNameNotFound(name.to_string()))?;

        let all_types_supported = {
            // all requested types
            types.iter().all(|mb_type: &MessageType| {
                // must be satisfied in the supported types
                plug.types
                    .iter()
                    .any(|plugin_type: &MessageType| plugin_type.satisfy(mb_type))
            })
        };

        if !all_types_supported {
            let unsupported_types = types
                .iter()
                .filter(|mty| !plug.types.iter().any(|pty| pty.satisfy(mty)))
                .map(|mty| mty.name())
                .collect();

            Err(DirectoryError::PluginDoesNotSupport(
                name.to_string(),
                unsupported_types,
            ))
        } else {
            Ok(Address::new(plug.communicator.clone()))
        }
    }

    /// Get the [`tedge_api::Address`] object that can be used to send messages to the core itself
    fn get_address_for_core(&self) -> Address<tedge_api::CoreMessages> {
        Address::new(self.core_communicator.clone())
    }

    pub fn get_core_communicator(&self) -> MessageSender {
        self.core_communicator.clone()
    }

    /// Construct a PluginDirectory object for a plugin named `plugin_name`
    ///
    /// This function is used to construct a `PluginDirectory` object that can be passed to the
    /// plugin named `plugin_name`.
    pub fn for_plugin_named(self: Arc<Self>, plugin_name: &str) -> PluginDirectory {
        PluginDirectory {
            core: self.clone(),
            plugin_name: plugin_name.to_string(),
        }
    }
}

/// Helper type for information about a plugin instance
pub(crate) struct PluginInfo {
    /// The types of messages the plugin claims to handle
    pub(crate) types: Vec<MessageType>,

    /// A sender to send messages to the plugin
    pub(crate) communicator: MessageSender,
}

impl std::fmt::Debug for PluginInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginInfo")
            .field("types", &self.types)
            .field("sender", &"...")
            .finish()
    }
}

impl PluginInfo {
    pub(crate) fn new(types: Vec<MessageType>, channel_size: usize) -> Self {
        Self {
            types,
            communicator: MessageSender::new(Default::default()),
        }
    }
}

/// `PluginDirectory` for one specific plugin named `plugin_name`
///
/// This `PluginDirectory` is a wrapper for `CorePluginDirectory`. It implements the
/// [`tedge_api::plugin::PluginDirectory`] interface that a `PluginBuilder` implementation can
/// then use to fetch addresses from.
///
/// The [`tedge_api::plugin::PluginDirectory::get_address_for`]
/// and [`tedge_api::plugin::PluginDirectory::get_address_for_core`]
/// functions are simply forwarded to the corresponding `CorePluginDirectory` functions.
///
/// The [`tedge_api::plugin::PluginDirectory::get_address_for_self`] function is a wrapper for
/// [`tedge_api::plugin::PluginDirectory::get_address_for`], but the plugin calling this function
/// does not need to know its own name.
pub struct PluginDirectory {
    core: Arc<CorePluginDirectory>,
    plugin_name: String,
}

impl ApiPluginDirectory for PluginDirectory {
    /// Forwarded to the corresponding `CorePluginDirectory` function
    fn get_address_for<RB: tedge_api::address::ReceiverBundle>(
        &self,
        name: &str,
    ) -> Result<Address<RB>, DirectoryError> {
        self.core.get_address_for::<RB>(name)
    }

    /// Forwarded to the corresponding `CorePluginDirectory` function
    fn get_address_for_core(&self) -> Address<tedge_api::CoreMessages> {
        self.core.get_address_for_core()
    }

    /// Call [`tedge_api::plugin::PluginDirectory::get_address_for`] with the name of the plugin
    /// itself.
    fn get_address_for_self<RB: tedge_api::address::ReceiverBundle>(
        &self,
    ) -> Result<Address<RB>, DirectoryError> {
        self.core.get_address_for::<RB>(&self.plugin_name)
    }
}

#[cfg(test)]
mod tests {
    mod testplugin {
        use async_trait::async_trait;
        use tedge_api::plugin::PluginExt;
        use tedge_api::Plugin;
        use tedge_api::PluginBuilder;
        use tedge_api::PluginConfiguration;
        use tedge_api::PluginDirectory;
        use tedge_api::PluginError;

        pub const TEST_PLUGIN_NAME: &'static str = "testplugin";

        pub struct Builder;

        #[async_trait::async_trait]
        impl<PD: PluginDirectory> PluginBuilder<PD> for Builder {
            fn kind_name() -> &'static str {
                "notsupported"
            }

            async fn verify_configuration(
                &self,
                _config: &PluginConfiguration,
            ) -> Result<(), tedge_api::error::PluginError> {
                Ok(())
            }

            async fn instantiate(
                &self,
                _config: PluginConfiguration,
                _cancellation_token: tedge_api::CancellationToken,
                _plugin_dir: &PD,
            ) -> Result<tedge_api::plugin::BuiltPlugin, PluginError> {
                Ok(Plug {}.finish())
            }

            fn kind_message_types() -> tedge_api::plugin::HandleTypes
            where
                Self: Sized,
            {
                Plug::get_handled_types()
            }
        }

        struct Plug;

        #[derive(Debug)]
        pub struct SupportedMessage;

        impl tedge_api::Message for SupportedMessage {}

        impl tedge_api::plugin::PluginDeclaration for Plug {
            type HandledMessages = (SupportedMessage,);
        }

        #[async_trait::async_trait]
        impl tedge_api::plugin::Handle<SupportedMessage> for Plug {
            async fn handle_message(
                &self,
                _message: SupportedMessage,
                _sender: tedge_api::address::ReplySenderFor<SupportedMessage>,
            ) -> Result<(), PluginError> {
                Ok(())
            }
        }

        #[async_trait]
        impl Plugin for Plug {
            async fn start(&mut self) -> Result<(), PluginError> {
                tracing::info!("Setup called");
                Ok(())
            }

            async fn shutdown(&mut self) -> Result<(), PluginError> {
                tracing::info!("Shutdown called");
                Ok(())
            }
        }
    }

    #[derive(Debug)]
    struct UnsupportedMessage;

    impl tedge_api::Message for UnsupportedMessage {}

    #[derive(Debug)]
    struct OtherUnsupportedMessage;

    impl tedge_api::Message for OtherUnsupportedMessage {}

    tedge_api::make_receiver_bundle!(pub struct UnsupportedMessageReceiver(UnsupportedMessage));
    tedge_api::make_receiver_bundle!(pub struct OtherUnsupportedMessageReceiver(OtherUnsupportedMessage));
    tedge_api::make_receiver_bundle!(pub struct AllUnsupportedMessageReceiver(UnsupportedMessage, OtherUnsupportedMessage));
    tedge_api::make_receiver_bundle!(pub struct SomeSupportedMessageReceiver(UnsupportedMessage, OtherUnsupportedMessage, testplugin::SupportedMessage));

    use tedge_api::message::MessageType;

    use super::*;
    use crate::configuration::TedgeConfiguration;

    fn make_directory() -> CorePluginDirectory {
        let _ = tracing_subscriber::fmt::try_init();

        let conf = format!(
            r#"
            communication_buffer_size = 10

            plugin_shutdown_timeout_ms = 2000

            [plugins]
            [plugins.{plugin_name}]
            kind = "notsupported"
            [plugins.{plugin_name}.configuration]
        "#,
            plugin_name = testplugin::TEST_PLUGIN_NAME
        );

        let channel_size = 1;
        let tedge_builder = crate::TedgeApplication::builder()
            .with_plugin_builder(testplugin::Builder {})
            .unwrap();

        let config: TedgeConfiguration = toml::de::from_str(&conf).unwrap();
        let directory_iter = config.plugins().iter().map(|(pname, pconfig)| {
            let handle_types = tedge_builder
                .plugin_builders()
                .get(pconfig.kind().as_ref())
                .map(|(handle_types, _)| {
                    handle_types
                        .get_types()
                        .into_iter()
                        .cloned()
                        .collect::<Vec<MessageType>>()
                })
                .ok_or_else(|| {
                    TedgeApplicationError::UnknownPluginKind(pconfig.kind().as_ref().to_string())
                })?;

            Ok((
                pname.to_string(),
                PluginInfo::new(handle_types, channel_size),
            ))
        });
        let (core_sender, _core_receiver) = tokio::sync::mpsc::channel(channel_size);

        CorePluginDirectory::collect_from(directory_iter, core_sender).unwrap()
    }

    #[test]
    fn test_not_supported_error_msg_mentions_unsupported_type() {
        let directory = make_directory();

        let addr =
            directory.get_address_for::<UnsupportedMessageReceiver>(testplugin::TEST_PLUGIN_NAME);
        match addr {
            Err(DirectoryError::PluginDoesNotSupport(plug_name, types)) => {
                assert_eq!(plug_name, testplugin::TEST_PLUGIN_NAME);
                assert_eq!(types.len(), 1);
                assert_eq!(types[0], std::any::type_name::<UnsupportedMessage>());
            }

            Err(other) => panic!("Expected PluginDoesNotSupport error, got {:?}", other),
            Ok(_) => panic!("Expected Err(PluginDoesNotSupport), got Ok(_)"),
        }
    }

    #[test]
    fn test_not_supported_error_msg_mentions_other_unsupported_type() {
        let directory = make_directory();

        let addr = directory
            .get_address_for::<OtherUnsupportedMessageReceiver>(testplugin::TEST_PLUGIN_NAME);
        match addr {
            Err(DirectoryError::PluginDoesNotSupport(plug_name, types)) => {
                assert_eq!(plug_name, testplugin::TEST_PLUGIN_NAME);
                assert_eq!(types.len(), 1);
                assert_eq!(types[0], std::any::type_name::<OtherUnsupportedMessage>());
            }

            Err(other) => panic!("Expected PluginDoesNotSupport error, got {:?}", other),
            Ok(_) => panic!("Expected Err(PluginDoesNotSupport), got Ok(_)"),
        }
    }

    #[test]
    fn test_not_supported_error_msg_mentions_all_unsupported_types() {
        let directory = make_directory();

        let addr = directory
            .get_address_for::<AllUnsupportedMessageReceiver>(testplugin::TEST_PLUGIN_NAME);
        match addr {
            Err(DirectoryError::PluginDoesNotSupport(plug_name, types)) => {
                assert_eq!(plug_name, testplugin::TEST_PLUGIN_NAME);
                assert_eq!(types.len(), 2);
                assert!(types
                    .iter()
                    .any(|e| *e == std::any::type_name::<UnsupportedMessage>()));
                assert!(types
                    .iter()
                    .any(|e| *e == std::any::type_name::<OtherUnsupportedMessage>()));
            }

            Err(other) => panic!("Expected PluginDoesNotSupport error, got {:?}", other),
            Ok(_) => panic!("Expected Err(PluginDoesNotSupport), got Ok(_)"),
        }
    }

    #[test]
    fn test_not_supported_error_msg_does_not_mention_supported_message() {
        let directory = make_directory();

        let addr =
            directory.get_address_for::<SomeSupportedMessageReceiver>(testplugin::TEST_PLUGIN_NAME);
        match addr {
            Err(DirectoryError::PluginDoesNotSupport(plug_name, types)) => {
                assert_eq!(plug_name, testplugin::TEST_PLUGIN_NAME);
                assert_eq!(types.len(), 2);
                assert!(types
                    .iter()
                    .any(|e| *e == std::any::type_name::<UnsupportedMessage>()));
                assert!(types
                    .iter()
                    .any(|e| *e == std::any::type_name::<OtherUnsupportedMessage>()));
                assert!(types
                    .iter()
                    .all(|e| *e != std::any::type_name::<testplugin::SupportedMessage>()));
            }

            Err(other) => panic!("Expected PluginDoesNotSupport error, got {:?}", other),
            Ok(_) => panic!("Expected Err(PluginDoesNotSupport), got Ok(_)"),
        }
    }
}
