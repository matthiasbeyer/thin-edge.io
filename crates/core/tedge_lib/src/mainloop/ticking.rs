//! Utility functionality for building a mainloop for a plugin
//!

use std::sync::Arc;

use tedge_api::error::PluginError;
use tracing::debug;
use tracing::error;
use tracing::trace;

pub struct MainloopTick<State: Sized> {
    pub(crate) state: State,
    pub(crate) logging: bool,
    pub(crate) stopper: tokio::sync::oneshot::Receiver<()>,
    pub(crate) duration: std::time::Duration,
}

impl<State> MainloopTick<State>
where
    State: Sized,
{
    #[must_use]
    pub fn with_logging(mut self, logging: bool) -> Self {
        self.logging = logging;
        self
    }

    #[tracing::instrument(skip_all)]
    pub async fn run<Func, Fut>(mut self, func: Func) -> Result<(), PluginError>
    where
        Func: Fn(Arc<State>) -> Fut,
        Fut: futures::future::Future<Output = Result<(), PluginError>>,
    {
        debug!(
            "Building ticking mainloop with interval = {:?}",
            self.duration
        );
        let mut interval = tokio::time::interval(self.duration);
        let state = Arc::new(self.state);
        loop {
            tokio::select! {
                _tick = interval.tick() => {
                    if self.logging {
                        trace!("Tick");
                    }

                    match func(state.clone()).await {
                        Ok(_) if self.logging => log::debug!("Ok(_) from mainloop function"),
                        Err(e) if self.logging => {
                            error!("Error from mainloop function: {:?}", e);
                            return Err(e)
                        },
                        _ => {},
                    }

                    if self.logging {
                        trace!("func returned");
                    }
                },

                _ = &mut self.stopper => {
                    if self.logging {
                        trace!("stopping...");
                    }

                    break;
                }
            }
        }

        Ok(())
    }
}
