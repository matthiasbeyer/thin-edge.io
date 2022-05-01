use tedge_api::error::PluginError;

pub struct MainloopDetach<State: Sized> {
    pub(super) state: State,
    pub(super) stopper: tedge_api::CancellationToken,
}

impl<State> MainloopDetach<State>
where
    State: Sized,
{
    pub async fn run<Func, Fut>(self, func: Func) -> Result<(), PluginError>
    where
        Func: Fn(State, tedge_api::CancellationToken) -> Fut,
        Fut: futures::future::Future<Output = Result<(), PluginError>>,
    {
        func(self.state, self.stopper).await
    }
}
