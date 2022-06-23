use std::{marker::PhantomData, sync::Arc, time::Duration};

use futures::future::BoxFuture;
use tokio::sync::RwLock;
use tracing::{instrument, trace};

use crate::message::{AcceptsReplies, Message, MessageType};

#[doc(hidden)]
pub type AnyMessageBox = Box<dyn Message>;

#[doc(hidden)]
pub struct InternalMessage {
    pub(crate) data: AnyMessageBox,
    pub(crate) reply_sender: tokio::sync::oneshot::Sender<AnyMessageBox>,
}

impl std::fmt::Debug for InternalMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InternalMessage")
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub enum ShouldWait {
    Wait,
    DontWait,
    Timeout(std::time::Duration),
}

#[doc(hidden)]
pub type MessageFutureProducer = dyn Fn(InternalMessage, ShouldWait) -> BoxFuture<'static, Result<(), InternalMessage>>
    + Sync
    + Send;

#[doc(hidden)]
#[derive(Clone)]
pub struct InnerMessageSender {
    #[doc(hidden)]
    pub send_provider: Arc<RwLock<Option<Box<MessageFutureProducer>>>>,
}

impl std::fmt::Debug for InnerMessageSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InnerMessageSender").finish_non_exhaustive()
    }
}

impl InnerMessageSender {
    pub fn new(send_provider: Arc<RwLock<Option<Box<MessageFutureProducer>>>>) -> Self {
        Self { send_provider }
    }

    pub async fn init_with(&self, producer: Box<MessageFutureProducer>) {
        let mut lock = self.send_provider.write().await;
        *lock = Some(producer);
    }

    pub async fn reset(&self) {
        let mut lock = self.send_provider.write().await;
        *lock = None;
    }

    #[instrument(skip_all, level = "trace")]
    async fn send(&self, message: InternalMessage) -> Result<(), InternalMessage> {
        let lock = self.send_provider.read().await;
        trace!(sender_exists = ?lock.is_some(), "Checking for internal sender");
        if let Some(sender) = &*lock {
            let sender = (*sender)(message, ShouldWait::Wait);

            sender.await
        } else {
            Err(message)
        }
    }

    async fn try_send(&self, message: InternalMessage) -> Result<(), InternalMessage> {
        let lock = self.send_provider.read().await;
        if let Some(sender) = &*lock {
            let sender = (*sender)(message, ShouldWait::DontWait);

            sender.await
        } else {
            Err(message)
        }
    }

    async fn send_timeout(
        &self,
        message: InternalMessage,
        timeout: Duration,
    ) -> Result<(), InternalMessage> {
        let lock = self.send_provider.read().await;
        if let Some(sender) = &*lock {
            let sender = (*sender)(message, ShouldWait::Timeout(timeout));

            sender.await
        } else {
            Err(message)
        }
    }
}

/// THIS IS NOT PART OF THE PUBLIC API, AND MAY CHANGE AT ANY TIME
#[doc(hidden)]
pub type MessageSender = InnerMessageSender;

/// THIS IS NOT PART OF THE PUBLIC API, AND MAY CHANGE AT ANY TIME
#[doc(hidden)]
pub type MessageReceiver = tokio::sync::mpsc::Receiver<InternalMessage>;

/// An address of a plugin that can receive messages a certain type of messages
///
/// An instance of this type represents an address that can be used to send messages of a
/// well-defined type to a specific plugin.
/// The `Address` instance can be used to send messages of several types, but each type has to be
/// in `RB: ReceiverBundle`.
pub struct Address<RB: ReceiverBundle> {
    _pd: PhantomData<fn(RB)>,
    sender: MessageSender,
}

impl<RB: ReceiverBundle> std::fmt::Debug for Address<RB> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("Address<{}>", std::any::type_name::<RB>()))
            .finish_non_exhaustive()
    }
}

impl<RB: ReceiverBundle> Clone for Address<RB> {
    fn clone(&self) -> Self {
        Self {
            _pd: PhantomData,
            sender: self.sender.clone(),
        }
    }
}

impl<RB: ReceiverBundle> Address<RB> {
    /// THIS IS NOT PART OF THE PUBLIC API, AND MAY CHANGE AT ANY TIME
    #[doc(hidden)]
    pub fn new(sender: MessageSender) -> Self {
        Self {
            _pd: PhantomData,
            sender,
        }
    }

    /// Send a message `M` to the address represented by the instance of this struct and wait for
    /// them to accept it
    ///
    /// This function can be used to send a message of type `M` to the plugin that is addressed by
    /// the instance of this type.
    ///
    /// # Return
    ///
    /// The function either returns `Ok(())` if sending the message succeeded,
    /// or the message in the error variant of the `Result`: `Err(M)`.
    ///
    /// The error is returned if the receiving side (the plugin that is addressed) does not receive
    /// messages anymore.
    ///
    /// # Details
    ///
    /// This function may block indefinitely if the receiving end does not start correctly. If this
    /// could become an issue use something akin to timeout (like
    /// [`timeout`](tokio::time::timeout)).
    /// For details on sending and receiving, see `tokio::sync::mpsc::Sender`.
    pub async fn send_and_wait<M: Message>(&self, msg: M) -> Result<ReplyReceiverFor<M>, M>
    where
        RB: Contains<M>,
    {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        self.sender
            .send(InternalMessage {
                data: Box::new(msg),
                reply_sender: sender,
            })
            .await
            .map_err(|msg| *msg.data.downcast::<M>().unwrap())?;

        Ok(ReplyReceiverFor {
            _pd: PhantomData,
            reply_recv: receiver,
        })
    }

    /// Try sending a message `M` to the plugin behind this address without potentially waiting
    ///
    /// This function should be used when waiting for the plugin to receive the message is not
    /// required.
    ///
    /// # Return
    ///
    /// The function either returns `Ok(())` if sending the message succeeded,
    /// or the message in the error variant of the `Result`: `Err(M)`.
    ///
    /// The error is returned if the receiving side (the plugin that is addressed) cannot currently
    /// receive messages (either because it is closed or the queue is full).
    pub async fn try_send<M: Message>(&self, msg: M) -> Result<ReplyReceiverFor<M>, M> {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        self.sender
            .try_send(InternalMessage {
                data: Box::new(msg),
                reply_sender: sender,
            })
            .await
            .map_err(|msg| *msg.data.downcast::<M>().unwrap())?;

        Ok(ReplyReceiverFor {
            _pd: PhantomData,
            reply_recv: receiver,
        })
    }

    /// Send a message `M` to the address represented by the instance of this struct and wait for
    /// them to accept it or timeout
    ///
    /// This method is identical to [`Address::send_and_wait`] except a timeout can be specified after which
    /// trying to send is aborted.
    ///
    /// If you do not wish to wait for a timeout see [`Address::try_send`]
    pub async fn send_with_timeout<M: Message>(
        &self,
        msg: M,
        timeout: Duration,
    ) -> Result<ReplyReceiverFor<M>, M> {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        self.sender
            .send_timeout(
                InternalMessage {
                    data: Box::new(msg),
                    reply_sender: sender,
                },
                timeout,
            )
            .await
            .map_err(|msg| *msg.data.downcast::<M>().unwrap())?;

        Ok(ReplyReceiverFor {
            _pd: PhantomData,
            reply_recv: receiver,
        })
    }

    /// Whether this Address could potentially receive this message.
    ///
    /// This does a check whether the [`ReceiverBundle`] contains the type of the message.
    pub fn could_receive(&self, msg: &dyn Message) -> bool {
        let types = RB::get_ids();
        let msg_type = MessageType::from_message(msg);

        types.iter().any(|ty| ty.satisfy(&msg_type))
    }
}

#[derive(Debug)]
/// Listener that allows one to wait for a reply as sent through [`Address::send_and_wait`]
pub struct ReplyReceiverFor<M> {
    _pd: PhantomData<fn(M)>,
    reply_recv: tokio::sync::oneshot::Receiver<AnyMessageBox>,
}

impl<M: Message> ReplyReceiverFor<M> {
    /// Wait for a reply until for the duration given in `timeout`
    ///
    /// ## Note
    ///
    /// Plugins could not reply for any number of reasons, hence waiting indefinitely on a reply
    /// can cause problems in long-running applications. As such, one needs to specify how long a
    /// reply should take before another action be taken.
    ///
    /// It is also important, that just because a given `M: Message` has a `M::Reply` type set,
    /// that the plugin that a message was sent to does _not_ have to reply with it. It can choose
    /// to not do so.
    pub async fn wait_for_reply<R>(self, timeout: Duration) -> Result<R, ReplyError>
    where
        R: Message,
        M: AcceptsReplies<Reply = R>,
    {
        let data = tokio::time::timeout(timeout, self.reply_recv)
            .await
            .map_err(|_| ReplyError::Timeout)?
            .map_err(|_| ReplyError::SendSideClosed)?;

        Ok(*data.downcast().expect("Invalid type received"))
    }
}

#[derive(Debug)]
/// Allows the [`Handle`](crate::plugin::Handle) implementation to reply with a given message as
/// specified by the currently handled message.
pub struct ReplySenderFor<M> {
    _pd: PhantomData<fn(M)>,
    reply_sender: tokio::sync::oneshot::Sender<AnyMessageBox>,
}

impl<M: Message> ReplySenderFor<M> {
    pub(crate) fn new(reply_sender: tokio::sync::oneshot::Sender<AnyMessageBox>) -> Self {
        Self {
            _pd: PhantomData,
            reply_sender,
        }
    }

    /// Reply to the originating plugin with the given message
    pub fn reply<R>(self, msg: R) -> Result<(), M>
    where
        R: Message,
        M: AcceptsReplies<Reply = R>,
    {
        self.reply_sender
            .send(Box::new(msg))
            .map_err(|msg| *msg.downcast::<M>().unwrap())
    }

    /// Check whether the ReplySender is closed
    ///
    /// This function returns when the internal communication channel is closed.
    /// This can be used (with e.g. [tokio::select]) to check whether the message sender stopped
    /// waiting for a reply.
    pub async fn closed(&mut self) {
        self.reply_sender.closed().await
    }
}

#[derive(Debug, thiserror::Error)]
/// An error occured while replying
pub enum ReplyError {
    /// The timeout elapsed before the other plugin responded
    #[error("There was no response before timeout")]
    Timeout,
    /// The other plugin dropped its sending side
    ///
    /// This means that there will never be an answer
    #[error("Could not send reply")]
    SendSideClosed,
}

#[doc(hidden)]
pub trait ReceiverBundle: Send + 'static {
    fn get_ids() -> Vec<MessageType>;
}

#[doc(hidden)]
pub trait Contains<M: Message> {}

/// Declare a set of messages to be a [`ReceiverBundle`] which is then used with an [`Address`] to
/// specify which kind of messages a given recipient plugin has to support.
///
/// The list of messages MUST be a subset of the messages the plugin behind `Address` supports.
///
/// ## Example
///
/// ```rust
/// # use bevy_reflect::TypeUuid;
/// # use tedge_api::{Message, make_receiver_bundle};
///
/// #[derive(Debug, TypeUuid)]
/// #[uuid = "b4e62630-0404-4d39-b435-95d777029887"]
/// struct IntMessage(u8);
///
/// impl Message for IntMessage { }
///
/// #[derive(Debug, TypeUuid)]
/// #[uuid = "92734ceb-7b65-499a-95cd-17164f1b3729"]
/// struct StatusMessage(String);
///
/// impl Message for StatusMessage { }
///
/// make_receiver_bundle!(struct MessageReceiver(IntMessage, StatusMessage));
///
/// // or if you want to export it
///
/// make_receiver_bundle!(pub struct AnotherMessageReceiver(IntMessage, StatusMessage));
/// ```
#[macro_export]
macro_rules! make_receiver_bundle {
    ($pu:vis struct $name:ident($($msg:ty),+)) => {
        #[allow(missing_docs)]
        #[derive(Debug)]
        $pu struct $name;

        impl $crate::address::ReceiverBundle for $name {
            #[allow(unused_parens)]
            fn get_ids() -> Vec<$crate::message::MessageType> {
                vec![
                    $($crate::message::MessageType::for_message::<$msg>()),+
                ]
            }
        }

        $(impl $crate::address::Contains<$msg> for $name {})+
    };
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bevy_reflect::TypeUuid;
    use static_assertions::{assert_impl_all, assert_not_impl_any};
    use tokio::sync::RwLock;

    use crate::{
        address::{InnerMessageSender, ReplyReceiverFor, ReplySenderFor},
        make_receiver_bundle,
        message::{AcceptsReplies, Message},
        Address,
    };

    #[derive(Debug, TypeUuid)]
    #[uuid = "df2b8bb3-8c15-49bb-8d11-cc14d7f3b000"] 
    struct Foo;

    impl Message for Foo {}
    impl AcceptsReplies for Foo {
        type Reply = Bar;
    }

    #[derive(Debug, TypeUuid)]
    #[uuid = "953a243d-333a-4870-8297-272fff6262b5"] 
    struct Bar;

    impl Message for Bar {}

    #[derive(Debug, TypeUuid)]
    #[uuid = "fe98650c-b067-47f4-8fd8-2f3ed04fdc21"] 
    struct Blub;

    impl Message for Blub {}

    make_receiver_bundle!(struct FooBar(Foo, Bar));

    #[allow(unreachable_code, dead_code, unused)]
    fn check_compile() {
        let addr: Address<FooBar> = todo!();
        addr.send_and_wait(Foo);
        addr.send_and_wait(Bar);
    }

    /////// Assert that types have the correct traits

    #[allow(dead_code)]
    struct NotSync {
        _pd: std::marker::PhantomData<*const ()>,
    }

    assert_impl_all!(Address<FooBar>: Clone, Send, Sync);

    assert_not_impl_any!(NotSync: Send, Sync);
    assert_impl_all!(ReplySenderFor<NotSync>: Send, Sync);
    assert_impl_all!(ReplyReceiverFor<NotSync>: Send, Sync);

    #[test]
    fn check_could_receive() {
        let sender = InnerMessageSender::new(Arc::new(RwLock::new(None)));
        let addr: Address<FooBar> = Address {
            _pd: std::marker::PhantomData,
            sender,
        };
        assert!(addr.could_receive(&Foo));
        assert!(addr.could_receive(&Bar));
        assert!(!addr.could_receive(&Blub));
    }
}
