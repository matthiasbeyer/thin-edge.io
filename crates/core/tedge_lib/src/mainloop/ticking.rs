//! Utility functionality for building a mainloop for a plugin
//!

use std::sync::Arc;

use tedge_api::error::PluginError;

pub struct Mainloop;

impl Mainloop {
    pub fn ticking_every<State>(
        duration: std::time::Duration,
        state: State,
    ) -> (MainloopStopper, MainloopTick<State>)
    where
        State: Sized,
    {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let mainloop = MainloopTick {
            state,
            logging: false,
            stopper: receiver,
            duration,
        };
        let stopper = MainloopStopper(sender);

        (stopper, mainloop)
    }
}

pub struct MainloopStopper(tokio::sync::oneshot::Sender<()>);

impl MainloopStopper {
    pub fn stop(self) -> Result<(), ()> {
        self.0.send(()).map_err(|_| ())
    }
}

pub struct MainloopTick<State: Sized> {
    state: State,
    logging: bool,
    stopper: tokio::sync::oneshot::Receiver<()>,
    duration: std::time::Duration,
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

    pub async fn run<Func, Fut>(mut self, func: Func) -> Result<(), PluginError>
    where
        Func: Fn(Arc<State>) -> Fut,
        Fut: futures::future::Future<Output = Result<(), PluginError>>,
    {
        let mut interval = tokio::time::interval(self.duration);
        let state = Arc::new(self.state);
        loop {
            tokio::select! {
                _tick = interval.tick() => {
                    if self.logging {
                        log::trace!("Tick");
                    }

                    match func(state.clone()).await {
                        Ok(_) if self.logging => log::debug!("Ok(_) from mainloop function"),
                        Err(e) if self.logging => {
                            log::error!("Error from mainloop function: {:?}", e);
                            return Err(e)
                        },
                        _ => {},
                    }

                    if self.logging {
                        log::trace!("func returned");
                    }
                },

                _ = &mut self.stopper => {
                    if self.logging {
                        log::trace!("stopping...");
                    }

                    break;
                }
            }
        }

        Ok(())
    }
}
