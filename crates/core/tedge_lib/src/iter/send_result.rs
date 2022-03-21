use tedge_api::PluginError;
use tedge_api::plugin::Message;

use crate::iter::SendResult;

pub trait MapSendResult<M>
where
    M: Message,
    Self: Iterator<Item = Result<SendResult<M>, PluginError>> + Sized,
{
    fn map_send_result<F>(self, f: F) -> MapSendResultIter<M, Self, F>
    where
        F: Fn(SendResult<M>) -> Result<(), PluginError>;
}

impl<M, I> MapSendResult<M> for I
where
    M: Message,
    I: Iterator<Item = Result<SendResult<M>, PluginError>> + Sized,
{
    fn map_send_result<F>(self, f: F) -> MapSendResultIter<M, Self, F>
    where
        F: Fn(SendResult<M>) -> Result<(), PluginError>,
    {
        MapSendResultIter { inner: self, f }
    }
}

pub struct MapSendResultIter<M, I, F>
where
    M: Message,
    I: Iterator<Item = Result<SendResult<M>, PluginError>> + Sized,
    F: Fn(SendResult<M>) -> Result<(), PluginError>,
{
    inner: I,
    f: F,
}

impl<M, I, F> Iterator for MapSendResultIter<M, I, F>
where
    M: Message,
    I: Iterator<Item = Result<SendResult<M>, PluginError>> + Sized,
    F: Fn(SendResult<M>) -> Result<(), PluginError>,
{
    type Item = Result<(), PluginError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(Ok(send_res)) => Some((self.f)(send_res)),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

pub fn log_and_ignore_timeout<M: Message>(sr: SendResult<M>) -> Result<(), PluginError> {
    match sr {
        SendResult::ReplyError(e) => {
            Err(anyhow::anyhow!("Error while receiving reply: {}", e).into())
        }
        SendResult::ReplyReceived(_) => Ok(()),
    }
}
