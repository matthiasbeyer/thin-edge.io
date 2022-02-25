use std::sync::Arc;

use futures::FutureExt;

use tedge_api::address::Address;
use tedge_api::CoreCommunication;
use tedge_api::MessageKind;
use tedge_api::PluginError;

use crate::reply::IntoReplyable;
use crate::reply::ReplyReceiver;
use crate::reply::ReplyableCoreCommunication;

pub trait IntoSendAll
where
    Self: Iterator<Item = (MessageKind, Address)> + Sized,
{
    fn send_all(self, comms: CoreCommunication) -> SendAll<Self>;
}

impl<I> IntoSendAll for I
where
    I: Iterator<Item = (MessageKind, Address)> + Sized,
{
    fn send_all(self, comms: CoreCommunication) -> SendAll<I> {
        SendAll { inner: self, comms }
    }
}

pub struct SendAll<I>
where
    I: Iterator<Item = (MessageKind, Address)> + Sized,
{
    inner: I,
    comms: CoreCommunication,
}

impl<I> Iterator for SendAll<I>
where
    I: Iterator<Item = (MessageKind, Address)> + Sized,
{
    type Item = std::pin::Pin<
        Box<dyn futures::future::Future<Output = Result<uuid::Uuid, PluginError>> + Send>,
    >;

    fn next(&mut self) -> Option<Self::Item> {
        let comms = self.comms.clone();
        self.inner.next().map(move |(message_kind, address)| {
            async move { comms.send(message_kind, address).await }.boxed()
        })
    }
}

impl<I> SendAll<I>
where
    I: Iterator<Item = (MessageKind, Address)> + Sized,
{
    pub fn wait_for_reply(self) -> SendAllWaitForReply<I> {
        SendAllWaitForReply {
            inner: self.inner,
            comms: Arc::new(self.comms.with_replies()),
        }
    }
}

pub struct SendAllWaitForReply<I> {
    inner: I,
    comms: Arc<ReplyableCoreCommunication>,
}

impl<I> Iterator for SendAllWaitForReply<I>
where
    I: Iterator<Item = (MessageKind, Address)> + Sized,
{
    type Item = std::pin::Pin<
        Box<dyn futures::future::Future<Output = Result<ReplyReceiver, PluginError>> + Send>,
    >;

    fn next(&mut self) -> Option<Self::Item> {
        let comms = self.comms.clone();
        self.inner.next().map(move |(message_kind, address)| {
            async move { comms.send_and_wait_for_reply(message_kind, address).await }.boxed()
        })
    }
}

impl<I> SendAllWaitForReply<I>
where
    I: Iterator<Item = (MessageKind, Address)> + Sized,
{
    pub fn with_timeout(self, timeout: std::time::Duration) -> SendAllWaitForReplyWithTimeout<I> {
        SendAllWaitForReplyWithTimeout {
            inner: self.inner,
            comms: self.comms,
            timeout,
        }
    }
}

pub struct SendAllWaitForReplyWithTimeout<I> {
    inner: I,
    comms: Arc<ReplyableCoreCommunication>,
    timeout: std::time::Duration,
}

impl<I> Iterator for SendAllWaitForReplyWithTimeout<I>
where
    I: Iterator<Item = (MessageKind, Address)> + Sized,
{
    type Item = std::pin::Pin<
        Box<dyn futures::future::Future<Output = Result<SendResult, PluginError>> + Send>,
    >;

    fn next(&mut self) -> Option<Self::Item> {
        let comms = self.comms.clone();
        let timeout = self.timeout.clone();

        self.inner.next().map(move |(message_kind, address)| {
            async move {
                let reply_channel = comms.send_and_wait_for_reply(message_kind, address).await?;
                match tokio::time::timeout(timeout, reply_channel).await {
                    Err(_) => Ok(SendResult::Timeout),
                    Ok(Ok(reply)) => Ok(SendResult::ReplyReceived(reply)),
                    Ok(Err(err)) => Ok(SendResult::ReceiveError(err)),
                }
            }
            .boxed()
        })
    }
}

#[derive(Debug)]
pub enum SendResult {
    ReplyReceived(tedge_api::Message),
    ReceiveError(tokio::sync::oneshot::error::RecvError),
    Timeout,
}
