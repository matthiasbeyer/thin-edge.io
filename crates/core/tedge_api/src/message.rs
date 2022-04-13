use crate::plugin::Message;

#[derive(Debug)]
/// A message which cannot be constructed and thus can be used when no reply is expected.
pub enum NoReply {}

impl Message for NoReply {
    type Reply = NoReply;
}

/// A message to tell the core to stop thin-edge
#[derive(Debug)]
pub struct StopCore;

impl Message for StopCore {
    type Reply = NoReply;
}

crate::make_receiver_bundle!(pub struct CoreMessages(StopCore));
