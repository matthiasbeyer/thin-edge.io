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

use crate::errors::TedgeApplicationError;
use crate::errors::Result;
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
        // Two tasks are started. The first one receives messages in a loop. For each received
        // message, a tokio task is spawned that calls `Plugin::handle_message()` (plus some panic
        // catching). This task (its JoinHandle) is then pushed to a channel.
        //
        // The second task receives from the channel and puts these JoinHandle objects into a
        // FuturesUnordered, which gets awaited in a streaming-like fashion (`StreamExt::next()`).
        // This also happens to be in a loop, of course.
        //
        let (handler_sender, handler_receiver) = tokio::sync::mpsc::channel(10); // TODO decide size, this decides the "concurrencyness" of the Plugin::handle_message()` calls for this plugin
        let plugin_lifecycle_join_handle = {
            let plugin = self.plugin;
            let plugin_msg_receiver = self.plugin_msg_receiver;
            let plugin_name: String = self.plugin_name.clone();
            let task_cancel_token = self.task_cancel_token;
            let shutdown_timeout = self.shutdown_timeout;

            // Spawn a task on the runtime that implements the lifecycle of the plugin
            tokio::spawn(process_plugin_lifecycle(
                plugin,
                plugin_msg_receiver,
                plugin_name,
                handler_sender,
                task_cancel_token,
                shutdown_timeout,
            ))
        };

        // Spawn a task on the runtime that takes care of waiting for the futures that call
        // `Plugin::handle_message()`.
        let waiter_task_join_handle = tokio::spawn(process_message_handling_futures(
            handler_receiver,
            self.plugin_name.clone(),
        ));

        match tokio::try_join!(plugin_lifecycle_join_handle, waiter_task_join_handle) {
            Ok((Ok(_), Ok(_))) => Ok(()),
            Ok((Ok(_), Err(e))) => Err(e),
            Ok((Err(e), _)) => Err(e),
            Err(e) => Err(TedgeApplicationError::MessageHandlingJobFailed(
                self.plugin_name.clone(), e
            )),
        }
    }
}

/// Implements the lifecycle of a plugin
///
/// This function implements the lifecycle of a plugin by calling its setup() method, looping over
/// incoming messages and passing them to the plugin and finally calling shutdown() on the plugin.
async fn process_plugin_lifecycle(
    plugin: BuiltPlugin,
    plugin_msg_receiver: MessageReceiver,
    plugin_name: String,
    handler_sender: tokio::sync::mpsc::Sender<tokio::task::JoinHandle<Result<()>>>,
    task_cancel_token: CancellationToken,
    shutdown_timeout: std::time::Duration,
) -> Result<()> {
    let plugin = Arc::new(RwLock::new(plugin));

    trace!("Setup for plugin '{}'", plugin_name);
    plugin_setup(plugin.clone(), &plugin_name).await?;
    trace!("Setup for plugin '{}' finished", plugin_name);

    trace!("Mainloop for plugin '{}'", plugin_name);
    plugin_mainloop(
        plugin.clone(),
        &plugin_name,
        plugin_msg_receiver,
        handler_sender,
        task_cancel_token,
    )
    .await?;
    trace!("Mainloop for plugin '{}' finished", plugin_name);

    info!("Shutting down {}", plugin_name);
    plugin_shutdown(plugin, &plugin_name, shutdown_timeout).await
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
            ))
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
    handler_sender: tokio::sync::mpsc::Sender<tokio::task::JoinHandle<Result<()>>>,
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
                        handler_sender.send(tokio::spawn(async move {
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
                        })).await
                        .map_err(|_| TedgeApplicationError::PluginMessageHandlingFailed(plugin_name.to_string()))?;
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

/// Receive and wait for the futures that call `Plugin::handle_message()`
async fn process_message_handling_futures(
    mut receiver: tokio::sync::mpsc::Receiver<tokio::task::JoinHandle<Result<()>>>,
    plugin_name: String,
) -> Result<()> {
    use futures::stream::StreamExt;

    let mut message_handle_tasks = futures::stream::FuturesUnordered::new();
    loop {
        tokio::select! {
            next_handler = receiver.recv() => {
                match next_handler {
                    Some(h) => message_handle_tasks.push(h),
                    None => break,
                }
            },

            next_handler_result = message_handle_tasks.next(), if !message_handle_tasks.is_empty() => {
                match next_handler_result {
                    Some(Ok(Ok(()))) => {
                        trace!("Message handler task returned Ok(())");
                    },

                    Some(Ok(Err(e))) => {
                        trace!("Message handler task errored");
                        return Err(e)
                    },

                    Some(Err(e)) => {
                        trace!("Joining message handle task failed");
                        return Err(TedgeApplicationError::MessageHandlingJobFailed(plugin_name, e))
                    },

                    None => {
                        trace!("Stream exhausted");
                    }
                }
            }
        }
    }

    Ok(())
}
