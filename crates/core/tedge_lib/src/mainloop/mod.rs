//! Utility functionality for building a mainloop for a plugin
//!

mod ticking;
pub use ticking::MainloopTick;

mod stopper;
pub use stopper::MainloopStopper;

mod detach;
pub use detach::MainloopDetach;

pub struct Mainloop;

impl Mainloop {
    pub fn ticking_every<State>(
        duration: std::time::Duration,
        state: State,
    ) -> (MainloopStopper, MainloopTick<State>)
    where
        State: Sized,
    {
        let token = tedge_api::CancellationToken::new();
        let mainloop = MainloopTick {
            state,
            logging: false,
            stopper: token.clone(),
            duration,
        };
        let stopper = MainloopStopper(token);

        (stopper, mainloop)
    }

    pub fn detach<State>(state: State) -> (MainloopStopper, MainloopDetach<State>) {
        let token = tedge_api::CancellationToken::new();
        let mainloop = MainloopDetach {
            state,
            stopper: token.clone(),
        };
        let stopper = MainloopStopper(token);

        (stopper, mainloop)
    }
}
