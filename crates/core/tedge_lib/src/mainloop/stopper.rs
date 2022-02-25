pub struct MainloopStopper(pub(super) tokio::sync::oneshot::Sender<()>);

impl MainloopStopper {
    pub fn stop(self) -> Result<(), ()> {
        self.0.send(()).map_err(|_| ())
    }
}
