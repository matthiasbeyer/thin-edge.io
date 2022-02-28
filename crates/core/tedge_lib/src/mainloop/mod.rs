//! Utility functionality for building a mainloop for a plugin
//!

mod ticking;
pub use ticking::*;

mod stopper;
pub use stopper::*;

mod detach;
pub use detach::*;

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

    pub fn detach<State>(state: State) -> (MainloopStopper, MainloopDetach<State>) {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let mainloop = MainloopDetach {
            state,
            stopper: receiver,
        };
        let stopper = MainloopStopper(sender);

        (stopper, mainloop)
    }
}
