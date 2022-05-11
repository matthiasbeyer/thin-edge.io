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
use tracing::Instrument;

use crate::errors::Result;
use crate::errors::TedgeApplicationError;
use crate::task::Task;

/// Type for handling the lifecycle of one individual Plugin instance
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
    /// Run the PluginTask
    ///
    /// This handles the complete lifecycle of one [`tedge_api::Plugin`] instance. That includes
    /// message passing as well as the crash-safety of that instance.
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

        plugin_setup(plugin.clone(), &self.plugin_name)
            .in_current_span()
            .instrument(tracing::trace_span!(
                "Setup for plugin '{name}'",
                name = %self.plugin_name
            ))
            .await?;
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
            .instrument(tracing::trace_span!("Plugin mainloop", name = %self.plugin_name))
            .await?;
        }
        trace!("Mainloop for plugin '{}' finished", self.plugin_name);

        info!("Shutting down {}", self.plugin_name);
        plugin_shutdown(plugin, &self.plugin_name, self.shutdown_timeout)
            .instrument(tracing::trace_span!("Plugin shutdown", name = %self.plugin_name))
            .await
    }
}

/// Handle the setup phase of the Plugin instance
///
/// This makes sure the [`tedge_api::Plugin::start`] function is called in a crash-safe way. That
/// means if the implementation of that function panics, this will simply return an error and not
/// take down the rest of the application.
///
/// If the starting of the plugin failed, this will error as well, of course.
#[tracing::instrument(skip(plugin))]
async fn plugin_setup(plugin: Arc<RwLock<BuiltPlugin>>, plugin_name: &str) -> Result<()> {
    let mut plug = plugin
        .write()
        .instrument(tracing::trace_span!("Aquiring write lock for plugin"))
        .await;

    // we can use AssertUnwindSafe here because we're _not_ using the plugin after a panic has
    // happened.
    match std::panic::AssertUnwindSafe(plug.plugin_mut().start())
        .catch_unwind()
        .instrument(tracing::trace_span!("Calling Plugin::start", name = %plugin_name))
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

/// Run the "main loop" for the Plugin instance
///
/// This runs the main part of the lifecycle of the Plugin instance: Waiting for messages from
/// other plugins and passing them to the instance handled here in a concurrent way.
///
/// If the application is cancelled by the user (via the CancellationToken), this function takes
/// care of stopping the "main loop" as well and returns cleanly.
#[tracing::instrument(skip(plugin, plugin_msg_receiver))]
async fn plugin_mainloop(
    plugin: Arc<RwLock<BuiltPlugin>>,
    plugin_name: &str,
    mut plugin_msg_receiver: MessageReceiver,
    task_cancel_token: CancellationToken,
) -> Result<()> {
    let mut receiver_closed = false;

    // allocate a mpsc channel with one element size
    // one element is enough because we stop the plugin anyways if there was a panic
    let (panic_err_sender, mut panic_err_recv) = tokio::sync::mpsc::channel(1);

    loop {
        tokio::select! {
            next_message = plugin_msg_receiver.recv(), if !receiver_closed => {
                match next_message {
                    Some(msg) => {
                        let pname = plugin_name.to_string();
                        let plug = plugin.clone();
                        let panic_err_sender = panic_err_sender.clone();

                        tokio::spawn(async move {
                            let read_plug = plug.read().await;
                            let handle_message_span = tracing::trace_span!("Calling Plugin::handle_message()", name = %pname, msg = ?msg);
                            let handled_message = std::panic::AssertUnwindSafe(read_plug.handle_message(msg))
                                .catch_unwind()
                                .instrument(handle_message_span)
                                .await;

                            match handled_message {
                                Err(_) => {
                                    // panic happened in handle_message() implementation

                                    error!("Plugin {} paniced in message handler", pname);
                                    let _ = panic_err_sender
                                        .send(TedgeApplicationError::PluginMessageHandlerPaniced(pname.to_string()));
                                },
                                Ok(Ok(_)) => debug!("Plugin handled message successfully"),
                                Ok(Err(e)) => warn!("Plugin failed to handle message: {:?}", e),
                            }
                        }.in_current_span());
                    },

                    None => {
                        receiver_closed = true;
                        debug!("Receiver closed for {} plugin", plugin_name);
                    },
                }
            },

            panic_err = panic_err_recv.recv() => {
                if let Some(panic_err) = panic_err {
                    return Err(panic_err)
                }
            }

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

/// Handle the shutdown procedure of the Plugin instance
///
/// This function takes care of calling [`tedge_api::Plugin::shutdown`] and makes sure that if the
/// implementation of that function panics, it does not take down the rest of the application.
///
/// A shutdown timeout (as configured by the user) is applied as well.
#[tracing::instrument(skip(plugin))]
async fn plugin_shutdown(
    plugin: Arc<RwLock<BuiltPlugin>>,
    plugin_name: &str,
    shutdown_timeout: std::time::Duration,
) -> Result<()> {
    let shutdown_fut = tokio::spawn(
        async move {
            let mut write_plug = plugin
                .write()
                .instrument(tracing::trace_span!("Aquiring write lock for plugin"))
                .await;

            write_plug
                .plugin_mut()
                .shutdown()
                .instrument(tracing::trace_span!(""))
                .await
        }
        .in_current_span(),
    );

    let timeouted_shutdown = tokio::time::timeout(shutdown_timeout, shutdown_fut)
        .instrument(tracing::trace_span!("Timeouted Plugin shutdown", name = %plugin_name, timeout = ?shutdown_timeout))
        .await;

    match timeouted_shutdown {
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
