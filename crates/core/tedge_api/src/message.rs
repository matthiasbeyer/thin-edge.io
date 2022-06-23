use downcast_rs::{impl_downcast, DowncastSync};
use bevy_reflect::{TypeUuid, TypeUuidDynamic};
use serde::Serialize;

use crate::address::AnyMessageBox;

/// An object that can be sent between [`Plugin`]s
///
/// This trait is a marker trait for all types that can be used as messages which can be sent
/// between plugins in thin-edge.
pub trait Message:
    Send + std::fmt::Debug + DynMessage + DowncastSync + TypeUuidDynamic + 'static
{
}

impl_downcast!(sync Message);

/// A bag of messages making it easier to work with dynamic messages
pub trait DynMessage {
    /// Get the type name of this message
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl<M: 'static> DynMessage for M {}

/// Register that the [`Message`] can receive replies of kind `R`: [`Message`]
pub trait AcceptsReplies: Message {
    /// The reply type that can be sent to implementing messages as replies
    type Reply: Message;
}

/// A message that can contain any other message
///
/// This is solely used in conjunction with [`AnyMessages`](crate::plugin::AnyMessages) and should not generally be used
/// otherwise.
///
/// To construct it, you will need to have a message and call [`AnyMessage::from_message`]
#[derive(Debug, TypeUuid)]
#[uuid = "e7e5c87b-2022-4687-8650-DEADBEEEEEEF"]
pub struct AnyMessage(pub(crate) AnyMessageBox);

impl std::ops::Deref for AnyMessage {
    type Target = dyn Message;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl AnyMessage {
    /// Construct a new [`AnyMessage`] from a message
    pub fn from_message<M: Message>(m: M) -> Self {
        AnyMessage(Box::new(m))
    }

    /// Try to downcast this message to a specific message
    pub fn downcast<M: Message>(self) -> Result<M, Self> {
        Ok(*self.0.downcast().map_err(AnyMessage)?)
    }

    /// Take out the raw boxed message
    ///
    /// Note
    ///
    /// This is an advanced API and should only be used if needed.
    /// Prefer using `AnyMessage::downcast` if possible
    pub fn into_raw(self) -> AnyMessageBox {
        self.0
    }
}

impl Message for AnyMessage {}

/// The type of a message as used by `tedge_api` to represent a type
#[derive(Debug, Clone, Serialize)]
pub struct MessageType {
    name: &'static str,
    kind: MessageKind,
}

#[derive(Clone, PartialEq, Debug)]
struct Uuid(bevy_reflect::Uuid);

#[derive(Debug, Clone, Serialize)]
enum MessageKind {
    Wildcard,
    #[serde(skip)]
    Typed(Uuid),
}

impl MessageType {
    /// Does this [`MessageType`] satisfy another [`MessageType`]
    ///
    /// ## Note
    /// A message type from [`AnyMessage`] acts as a 'wildcard', being satisfied by any other type
    /// (even itself).
    /// The reverse is not true, a specific type cannot be satisfied by a 'wildcard' (i.e.
    /// [`AnyMessage`]).
    ///
    /// [`MessageType::satisfy`] is thus reflexive but not symmetric nor transitive, meaning that it cannot be
    /// used for `PartialEq`.
    #[must_use]
    pub fn satisfy(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (MessageKind::Wildcard, _) => true,
            (_, MessageKind::Wildcard) => false,
            (MessageKind::Typed(ty_l), MessageKind::Typed(ty_r)) => ty_l.eq(ty_r),
        }
    }

    /// Get the [`MessageType`] for a `M`:[`Message`]
    #[must_use]
    pub fn for_message<M: Message + TypeUuid>() -> Self {
        let id = M::TYPE_UUID;
        MessageType {
            name: std::any::type_name::<M>(),
            kind: if id == AnyMessage::TYPE_UUID {
                MessageKind::Wildcard
            } else {
                MessageKind::Typed(Uuid(id))
            },
        }
    }

    pub(crate) fn from_message(msg: &dyn Message) -> Self {
        let id = msg.type_uuid();
        MessageType {
            name: TypeUuidDynamic::type_name(msg),
            kind: if id == AnyMessage::TYPE_UUID {
                MessageKind::Wildcard
            } else {
                MessageKind::Typed(Uuid(id))
            },
        }
    }

    /// Get the type's name
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }
}

/// A message to tell the core to stop thin-edge
#[derive(Debug, TypeUuid)]
#[uuid = "812b7235-671f-4722-b01a-333578b2a4ea"]
pub struct StopCore;

impl Message for StopCore {}

crate::make_receiver_bundle!(pub struct CoreMessages(StopCore));

#[cfg(test)]
mod tests {
    use bevy_reflect::TypeUuid;

    use crate::Message;

    use super::{AnyMessage, MessageType};

    #[derive(Debug, TypeUuid)]
    #[uuid = "0c2ed228-5ff0-4ba5-a0d3-3648f7eb6558"]
    struct Bar;

    impl Message for Bar {}

    #[derive(Debug, TypeUuid)]
    #[uuid = "8c3beacb-327d-422d-a914-47006d521ba5"]
    struct Foo;

    impl Message for Foo {}

    #[test]
    fn assert_satisfy_laws_for_types() {
        let bar_type = MessageType::for_message::<Bar>();
        let any_message_type = MessageType::for_message::<AnyMessage>();
        let foo_type = MessageType::for_message::<Foo>();

        assert!(any_message_type.satisfy(&bar_type));
        assert!(any_message_type.satisfy(&foo_type));

        assert!(!bar_type.satisfy(&any_message_type));
        assert!(!bar_type.satisfy(&foo_type));
    }
}
