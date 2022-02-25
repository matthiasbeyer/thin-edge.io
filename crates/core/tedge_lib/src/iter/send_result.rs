use tedge_api::PluginError;

use crate::iter::SendResult;

pub trait MapSendResult
where
    Self: Iterator<Item = Result<SendResult, PluginError>> + Sized,
{
    fn map_send_result<F>(self, f: F) -> MapSendResultIter<Self, F>
    where
        F: Fn(SendResult) -> Result<(), PluginError>;
}

impl<I> MapSendResult for I
where
    I: Iterator<Item = Result<SendResult, PluginError>> + Sized,
{
    fn map_send_result<F>(self, f: F) -> MapSendResultIter<Self, F>
    where
        F: Fn(SendResult) -> Result<(), PluginError>,
    {
        MapSendResultIter { inner: self, f }
    }
}

pub struct MapSendResultIter<I, F>
where
    I: Iterator<Item = Result<SendResult, PluginError>> + Sized,
    F: Fn(SendResult) -> Result<(), PluginError>,
{
    inner: I,
    f: F,
}

impl<I, F> Iterator for MapSendResultIter<I, F>
where
    I: Iterator<Item = Result<SendResult, PluginError>> + Sized,
    F: Fn(SendResult) -> Result<(), PluginError>,
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

pub fn log_and_ignore_timeout(sr: SendResult) -> Result<(), PluginError> {
    match sr {
        SendResult::Timeout => {
            log::warn!("Timeout while waiting for reply");
            Ok(())
        }
        SendResult::ReceiveError(e) => {
            Err(anyhow::anyhow!("Error while receiving reply: {}", e).into())
        }
        SendResult::ReplyReceived(_) => Ok(()),
    }
}
