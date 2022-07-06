//! Utilities for pubsub-style plugin communication

mod request {
    use tedge_api::message::AcceptsReplies;
    use tedge_api::message::Message;

    use super::SubscribeReply;

    /// A SubscribeRequest represents a request for subsribing to a certain type of messages
    ///
    /// For example, a plugin might send a subscription request for Measurements to another plugin that
    /// must be able to handle such a request, which then sends messages to the subscriber.
    #[derive(Debug)]
    pub struct SubscribeRequest<M: Message> {
        pd: std::marker::PhantomData<M>,
    }

    impl<M: Message> SubscribeRequest<M> {
        pub fn new() -> Self {
            Self {
                pd: std::marker::PhantomData,
            }
        }
    }

    // Helper type so we can type out the UUID in a human readable form
    #[derive(bevy_reflect::TypeUuid)]
    #[uuid = "e33209db-c5ec-4e10-9760-6047d02edbfe"]
    struct SubscribeRequestUuid;

    impl<M: Message + bevy_reflect::TypeUuid> bevy_reflect::TypeUuid for SubscribeRequest<M> {
        const TYPE_UUID: bevy_reflect::Uuid =
            tedge_api::util::generate_composite_uuid(SubscribeRequestUuid::TYPE_UUID, M::TYPE_UUID);
    }

    impl<M: Message + bevy_reflect::TypeUuid> Message for SubscribeRequest<M> {}

    impl<M: Message + bevy_reflect::TypeUuid> AcceptsReplies for SubscribeRequest<M> {
        type Reply = SubscribeReply<M>;
    }
}

mod reply {
    use tedge_api::error::PluginError;
    use tedge_api::message::Message;

    #[derive(Debug)]
    pub struct SubscribeReply<M: Message>(Result<tokio::sync::broadcast::Receiver<M>, PluginError>);

    impl<M: Message> SubscribeReply<M> {
        pub fn new(result: Result<tokio::sync::broadcast::Receiver<M>, PluginError>) -> Self {
            Self(result)
        }

        pub fn new_from_sender(sender: &tokio::sync::broadcast::Sender<M>) -> Self {
            Self(Ok(sender.subscribe()))
        }

        pub fn into_inner(self) -> Result<tokio::sync::broadcast::Receiver<M>, PluginError> {
            self.0
        }
    }

    // Helper type so we can type out the UUID in a human readable form
    #[derive(bevy_reflect::TypeUuid)]
    #[uuid = "efc60260-588a-4d63-a5ba-31e19a9b2d99"]
    struct SubscribeReplyUuid;

    impl<M: Message + bevy_reflect::TypeUuid> bevy_reflect::TypeUuid for SubscribeReply<M> {
        const TYPE_UUID: bevy_reflect::Uuid =
            tedge_api::util::generate_composite_uuid(SubscribeReplyUuid::TYPE_UUID, M::TYPE_UUID);
    }

    impl<M: Message> From<SubscribeReply<M>>
        for Result<tokio::sync::broadcast::Receiver<M>, PluginError>
    {
        fn from(sr: SubscribeReply<M>) -> Self {
            sr.0
        }
    }

    impl<M: Message + bevy_reflect::TypeUuid> Message for SubscribeReply<M> {}
}

pub use self::reply::*;
pub use self::request::*;
