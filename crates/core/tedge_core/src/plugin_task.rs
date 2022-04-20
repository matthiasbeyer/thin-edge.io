use std::sync::Arc;

use futures::FutureExt;
use tedge_api::address::MessageReceiver;
use tedge_api::plugin::BuiltPlugin;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;
use tracing::warn;

use crate::errors::Result;
use crate::errors::TedgeApplicationError;
use crate::task::Task;

pub struct PluginTask {
    plugin_name: String,
    plugin: BuiltPlugin,
    plugin_msg_receiver: MessageReceiver,
    task_cancel_token: CancellationToken,
    shutdown_timeout: std::time::Duration,
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
        shutdown_timeout: std::time::Duration,
    ) -> Self {
        Self {
            plugin_name,
            plugin,
            plugin_msg_receiver,
            task_cancel_token,
            shutdown_timeout,
        }
    }
}

#[async_trait::async_trait]
impl Task for PluginTask {
    #[tracing::instrument]
    async fn run(mut self) -> Result<()> {
        // In this implementation, we have the problem that all messages sent to a plugin should be
        // handled _concurrently_.
        // If we simply loop over `self.plugin_msg_receiver.recv()`ed messages and pass them to
        // `self.plugin.handle_message()`, we do not get real concurrency, but process messages
        // sequentially.
        //
        // For this to resolve, we build the following pattern:
        //
        // A Plugin instance is guarded with a RwLock. That RwLock is aquired mutably during the
        // plugin setup, non-mutably via the message handing and mutable again for the shutdown.
        // With this we get "waiting for all handling to be finished" for free.
        //
        let plugin = Arc::new(RwLock::new(self.plugin));

        trace!("Setup for plugin '{}'", self.plugin_name);
        plugin_setup(plugin.clone(), &self.plugin_name).await?;
        trace!("Setup for plugin '{}' finished", self.plugin_name);

        trace!("Mainloop for plugin '{}'", self.plugin_name);
        {
            let plugin_msg_receiver = self.plugin_msg_receiver;
            let task_cancel_token = self.task_cancel_token;
            plugin_mainloop(
                plugin.clone(),
                &self.plugin_name,
                plugin_msg_receiver,
                task_cancel_token,
            )
            .await?;
        }
        trace!("Mainloop for plugin '{}' finished", self.plugin_name);

        info!("Shutting down {}", self.plugin_name);
        plugin_shutdown(plugin, &self.plugin_name, self.shutdown_timeout).await
    }
}

async fn plugin_setup(plugin: Arc<RwLock<BuiltPlugin>>, plugin_name: &str) -> Result<()> {
    let mut plug = plugin.write().await;
    // we can use AssertUnwindSafe here because we're _not_ using the plugin after a panic has
    // happened.
    match std::panic::AssertUnwindSafe(plug.plugin_mut().start())
        .catch_unwind()
        .await
    {
        Err(_) => {
            // don't make use of the plugin for unwind safety reasons, and the plugin
            // will be dropped

            error!("Plugin {} paniced in setup", plugin_name);
            return Err(TedgeApplicationError::PluginSetupPaniced(
                plugin_name.to_string(),
            ));
        }
        Ok(res) => {
            res.map_err(|e| TedgeApplicationError::PluginSetupFailed(plugin_name.to_string(), e))
        }
    }
}

async fn plugin_mainloop(
    plugin: Arc<RwLock<BuiltPlugin>>,
    plugin_name: &str,
    mut plugin_msg_receiver: MessageReceiver,
    task_cancel_token: CancellationToken,
) -> Result<()> {
    let mut receiver_closed = false;
    loop {
        tokio::select! {
            next_message = plugin_msg_receiver.recv(), if !receiver_closed => {
                match next_message {
                    Some(msg) => {
                        let pname = plugin_name.to_string();
                        let plug = plugin.clone();

                        // send the future that calls Plugin::handle_message() to the task that
                        // takes care of awaiting these futures.
                        tokio::spawn(async move {
                            let read_plug = plug.read().await;
                            match std::panic::AssertUnwindSafe(read_plug.handle_message(msg)).catch_unwind().await {
                                Err(_) => {
                                    // panic happened in handle_message() implementation

                                    error!("Plugin {} paniced in message handler", pname);
                                    return Err(TedgeApplicationError::PluginMessageHandlerPaniced(pname.to_string()))
                                },
                                Ok(Ok(_)) => debug!("Plugin handled message successfully"),
                                Ok(Err(e)) => warn!("Plugin failed to handle message: {:?}", e),
                            }
                            Ok(())
                        }).await
                        .map_err(|_| TedgeApplicationError::PluginMessageHandlingFailed(plugin_name.to_string()))??;
                    },

                    None => {
                        receiver_closed = true;
                        debug!("Receiver closed for {} plugin", plugin_name);
                    },
                }
            },

            _shutdown = task_cancel_token.cancelled() => {
                // no communication happened when we got this future returned,
                // so we're done now
                debug!("Received shutdown request");
                info!("Going to shut down {}", plugin_name);
                break
            }
        }
    }
    Ok(())
}

async fn plugin_shutdown(
    plugin: Arc<RwLock<BuiltPlugin>>,
    plugin_name: &str,
    shutdown_timeout: std::time::Duration,
) -> Result<()> {
    let shutdown_fut = tokio::spawn(async move {
        let mut write_plug = plugin.write().await;
        write_plug.plugin_mut().shutdown().await
    });

    match tokio::time::timeout(shutdown_timeout, shutdown_fut).await {
        Err(_timeout) => {
            error!("Shutting down {} timeouted", plugin_name);
            Err(TedgeApplicationError::PluginShutdownTimeout(
                plugin_name.to_string(),
            ))
        }
        Ok(Err(e)) => {
            error!("Waiting for plugin {} shutdown failed", plugin_name);
            if e.is_panic() {
                error!("Shutdown of {} paniced", plugin_name);
            } else if e.is_cancelled() {
                error!("Shutdown of {} cancelled", plugin_name);
            }
            Err(TedgeApplicationError::PluginShutdownError(
                plugin_name.to_string(),
            ))
        }
        Ok(Ok(res)) => {
            info!("Shutting down {} completed", plugin_name);
            res.map_err(|_| TedgeApplicationError::PluginShutdownError(plugin_name.to_string()))
        }
    }
}
