use futures::FutureExt;

use tedge_api::Message;
use tedge_api::address::Address;
use tedge_api::address::ReceiverBundle;
use tedge_api::address::ReplyReceiverFor;
use tedge_api::message::AcceptsReplies;

pub trait IntoSendAll<'addr, M: Message, RB: ReceiverBundle>
where
    Self: Iterator<Item = (M, &'addr Address<RB>)> + Sized,
    RB: 'addr,
    RB: tedge_api::address::Contains<M>,
{
    fn send_all(self) -> SendAll<'addr, M, RB, Self>;
}

impl<'addr, M, RB, I> IntoSendAll<'addr, M, RB> for I
where
    Self: Iterator<Item = (M, &'addr Address<RB>)> + Sized,
    M: Message,
    RB: 'addr,
    RB: ReceiverBundle,
    RB: tedge_api::address::Contains<M>,
{
    fn send_all(self) -> SendAll<'addr, M, RB, I> {
        SendAll { inner: self }
    }
}

pub struct SendAll<'addr, M, RB, I>
where
    I: Iterator<Item = (M, &'addr Address<RB>)> + Sized,
    M: Message,
    RB: 'addr,
    RB: ReceiverBundle,
    RB: tedge_api::address::Contains<M>,
{
    inner: I,
}

impl<'addr, M, RB, I> Iterator for SendAll<'addr, M, RB, I>
where
    I: Iterator<Item = (M, &'addr Address<RB>)> + Sized,
    M: Message,
    RB: 'addr,
    RB: ReceiverBundle,
    RB: tedge_api::address::Contains<M>,
{
    type Item = std::pin::Pin<
        Box<dyn futures::future::Future<Output = Result<ReplyReceiverFor<M>, M>> + Send + 'addr>,
    >;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(msg, address)| async { address.send_and_wait(msg).await }.boxed())
    }
}

impl<'addr, M, RB, I> SendAll<'addr, M, RB, I>
where
    I: Iterator<Item = (M, &'addr Address<RB>)>,
    M: Message,
    RB: 'addr,
    RB: ReceiverBundle,
    RB: tedge_api::address::Contains<M>,
{
    pub fn wait_for_reply(self, to: std::time::Duration) -> SendAllWaitForReply<'addr, M, RB, I> {
        SendAllWaitForReply {
            inner: self.inner,
            timeout: to,
        }
    }
}

pub struct SendAllWaitForReply<'addr, M, RB, I>
where
    I: Iterator<Item = (M, &'addr Address<RB>)>,
    M: Message,
    RB: 'addr,
    RB: ReceiverBundle,
    RB: tedge_api::address::Contains<M>,
{
    inner: I,
    timeout: std::time::Duration,
}

impl<'addr, M, RB, I> Iterator for SendAllWaitForReply<'addr, M, RB, I>
where
    I: Iterator<Item = (M, &'addr Address<RB>)>,
    M: AcceptsReplies,
    RB: 'addr,
    RB: ReceiverBundle,
    RB: tedge_api::address::Contains<M>,
{
    type Item = std::pin::Pin<
        Box<dyn futures::future::Future<Output = Result<SendResult<M::Reply>, M>> + Send + 'addr>,
    >;

    fn next(&mut self) -> Option<Self::Item> {
        let timeout = self.timeout;

        self.inner.next().map(|(msg, address)| {
            async move {
                let reply_recv = address.send_and_wait(msg).await?.wait_for_reply(timeout);
                match reply_recv.await {
                    Err(err) => Ok(SendResult::ReplyError(err)),
                    Ok(msg) => Ok(SendResult::ReplyReceived(msg)),
                }
            }
            .boxed()
        })
    }
}

#[derive(Debug)]
pub enum SendResult<M: Message> {
    ReplyReceived(M),
    ReplyError(tedge_api::address::ReplyError),
}
