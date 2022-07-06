pub struct MainloopStopper(pub(super) tedge_api::CancellationToken);

impl MainloopStopper {
    pub fn stop(self) {
        self.0.cancel();
    }
}

impl std::fmt::Debug for MainloopStopper {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("MainloopStopper").finish()
    }
}
