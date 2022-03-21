use std::collections::HashMap;

use tracing::trace;
use tokio_util::sync::CancellationToken;

pub struct CoreTask {
    cancellation_token: CancellationToken,
}

impl std::fmt::Debug for CoreTask {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("CoreTask").finish()
    }
}

impl CoreTask {
    pub fn new(cancellation_token: CancellationToken) -> Self {
        Self {
            cancellation_token
        }
    }

    pub(crate) async fn run(self) -> crate::errors::Result<()> {
        Ok(())
    }
}
