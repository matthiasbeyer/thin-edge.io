use tedge_api::address::MessageReceiver;
use tedge_api::plugin::BuiltPlugin;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::errors::Result;
use crate::task::Task;

pub struct PluginTask {
    plugin_name: String,
    plugin: BuiltPlugin,
    plugin_msg_receiver: MessageReceiver,
    task_cancel_token: CancellationToken,
}

impl std::fmt::Debug for PluginTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginTask")
            .field("plugin_name", &self.plugin_name)
            .finish()
    }
}

impl PluginTask {
    pub fn new(
        plugin_name: String,
        plugin: BuiltPlugin,
        plugin_msg_receiver: MessageReceiver,
        task_cancel_token: CancellationToken,
    ) -> Self {
        Self {
            plugin_name,
            plugin,
            plugin_msg_receiver,
            task_cancel_token,
        }
    }
}

#[async_trait::async_trait]
impl Task for PluginTask {
    #[tracing::instrument]
    async fn run(mut self) -> Result<()> {
        self.plugin.plugin_mut().setup().await?;
        let mut receiver_closed = false;

        loop {
            tokio::select! {
                next_message = self.plugin_msg_receiver.recv(), if !receiver_closed => {
                    match next_message {
                        Some(msg) => match self.plugin.handle_message(msg).await {
                            Ok(_) => debug!("Plugin handled message successfully"),
                            Err(e) => warn!("Plugin failed to handle message: {:?}", e),
                        },

                        None => {
                            receiver_closed = true;
                            debug!("Receiver closed for {} plugin", self.plugin_name);
                        },
                    }
                }

                _shutdown = self.task_cancel_token.cancelled() => {
                    // no communication happened when we got this future returned,
                    // so we're done now
                    debug!("Received shutdown request");
                    info!("Going to shut down {}", self.plugin_name);
                    break
                }
            }
        }

        info!("Shutting down {}", self.plugin_name);
        self.plugin.plugin_mut().shutdown().await?;
        info!("Shutting down {} completed", self.plugin_name);
        Ok(())
    }
}
