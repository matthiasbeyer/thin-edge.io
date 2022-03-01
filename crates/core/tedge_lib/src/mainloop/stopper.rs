pub struct MainloopStopper(pub(super) tokio::sync::oneshot::Sender<()>);

impl MainloopStopper {
    pub fn stop(self) -> Result<(), ()> {
        self.0.send(()).map_err(|_| ())
    }
}

impl std::fmt::Debug for MainloopStopper {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("MainloopStopper").finish()
    }
}
