//! Utility functionality for building a mainloop for a plugin
//!

mod ticking;
pub use ticking::*;

mod stopper;
pub use stopper::*;

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
