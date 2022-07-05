use std::sync::Arc;

use futures::FutureExt;
use tedge_api::address::MessageSender;
use tedge_api::plugin::BuiltPlugin;
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;
use tracing::trace;
use tracing::Instrument;

use crate::errors::PluginLifecycleError;
use crate::errors::PluginMainFailed;
use crate::errors::PluginMainPanicked;
use crate::errors::PluginStartFailed;
use crate::errors::PluginStartPanicked;
use crate::errors::PluginStopFailed;
use crate::errors::PluginStopPanicked;
use crate::errors::PluginStopTimeout;
use crate::message_handler::make_message_handler;

/// Type for handling the lifecycle of one individual Plugin instance
pub struct PluginTask {
    plugin_name: String,
    plugin: Arc<RwLock<BuiltPlugin>>,
    max_concurrency: usize,
    plugin_msg_communications: MessageSender,
    panic_channel: (Sender<()>, Receiver<()>),
    _task_cancel_token: CancellationToken,
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
        plugin: Arc<RwLock<BuiltPlugin>>,
        max_concurrency: usize,
        plugin_msg_communications: MessageSender,
        task_cancel_token: CancellationToken,
        shutdown_timeout: std::time::Duration,
    ) -> Self {
        let panic_channel = channel::<()>(1);
        Self {
            plugin_name,
            plugin,
            max_concurrency,
            plugin_msg_communications,
            panic_channel,
            _task_cancel_token: task_cancel_token,
            shutdown_timeout,
        }
    }

    /// Get a reference to the plugin task's plugin name.
    pub fn plugin_name(&self) -> &str {
        self.plugin_name.as_ref()
    }
}

impl PluginTask {
    pub async fn run_start(&mut self) -> Result<(), PluginLifecycleError> {
        let plugin_name: &str = &self.plugin_name;
        let mut plug_write = self.plugin.write().await;

        // we can use AssertUnwindSafe here because we're _not_ using the plugin after a panic has
        // happened.
        match std::panic::AssertUnwindSafe(plug_write.plugin_mut().start())
            .catch_unwind()
            .instrument(tracing::trace_span!("core.plugin_task.setup.start"))
            .await
        {
            Err(panic) => {
                let message: &str = {
                    if let Some(message) = panic.downcast_ref::<&'static str>() {
                        message
                    } else if let Some(message) = panic.downcast_ref::<String>() {
                        &*message
                    } else {
                        "Unknown panic message"
                    }
                };
                error!(panic = %message, "Plugin paniced in setup");

                // don't make use of the plugin for unwind safety reasons, and the plugin
                // will be dropped
                return Err(PluginLifecycleError::PluginStartPanicked(
                    PluginStartPanicked {
                        name: plugin_name.to_string(),
                    },
                ));
            }
            Ok(res) => {
                res.map_err(|e| {
                    PluginLifecycleError::PluginStartFailed(PluginStartFailed {
                        name: plugin_name.to_string(),
                        error: e,
                    })
                })?;
            }
        };

        Ok(())
    }

    pub async fn enable_communications(&self) -> Result<(), PluginLifecycleError> {
        trace!(max_concurrency = ?self.max_concurrency, "enabling communications");
        self.plugin_msg_communications
            .init_with(make_message_handler(
                Arc::new(Semaphore::new(self.max_concurrency)),
                self.plugin.clone(),
                self.panic_channel.0.clone(),
            ))
            .await;

        Ok(())
    }

    /// Run the PluginTask
    ///
    /// This handles the complete lifecycle of one [`tedge_api::Plugin`] instance. That includes
    /// message passing as well as the crash-safety of that instance.
    pub async fn run_main(&self) -> Result<(), PluginLifecycleError> {
        let plug_read = self.plugin.read().await;

        // we can use AssertUnwindSafe here because we're _not_ using the plugin after a panic has
        // happened.
        match std::panic::AssertUnwindSafe(plug_read.plugin().main())
            .catch_unwind()
            .instrument(tracing::trace_span!("core.plugin_task.setup.main"))
            .await
        {
            Err(panic) => {
                let message: &str = {
                    if let Some(message) = panic.downcast_ref::<&'static str>() {
                        message
                    } else if let Some(message) = panic.downcast_ref::<String>() {
                        &*message
                    } else {
                        "Unknown panic message"
                    }
                };
                error!(panic = %message, "Plugin paniced in main");

                // don't make use of the plugin for unwind safety reasons, and the plugin
                // will be dropped
                return Err(PluginLifecycleError::PluginMainPanicked(
                    PluginMainPanicked {
                        name: self.plugin_name.to_string(),
                    },
                ));
            }
            Ok(res) => {
                res.map_err(|e| {
                    PluginLifecycleError::PluginMainFailed(PluginMainFailed {
                        name: self.plugin_name.to_string(),
                        error: e,
                    })
                })?;
            }
        };

        Ok(())
    }

    pub async fn disable_communications(&self) -> Result<(), PluginLifecycleError> {
        self.plugin_msg_communications.reset().await;

        Ok(())
    }

    pub async fn run_shutdown(&mut self) -> Result<(), PluginLifecycleError> {
        let plugin_name: &str = &self.plugin_name;
        let shutdown_timeout = self.shutdown_timeout;
        let plugin = self.plugin.clone();
        let shutdown_fut = tokio::spawn(
            async move {
                let mut write_plug = plugin
                    .write()
                    .instrument(tracing::trace_span!("core.plugin_task.shutdown.lock"))
                    .await;

                write_plug
                    .plugin_mut()
                    .shutdown()
                    .instrument(tracing::trace_span!("core.plugin_task.shutdown.shutdown"))
                    .await
            }
            .in_current_span(),
        );

        let timeouted_shutdown = tokio::time::timeout(shutdown_timeout, shutdown_fut)
                .instrument(
                    tracing::trace_span!("core.plugin_task.shutdown.timeout", timeout = ?shutdown_timeout),
                )
                .await;

        match timeouted_shutdown {
            Err(_timeout) => {
                error!("Shutting down {} timeouted", plugin_name);
                Err(PluginLifecycleError::PluginStopTimeout(PluginStopTimeout {
                    name: plugin_name.to_string(),
                    timeout_duration: shutdown_timeout,
                }))
            }
            Ok(Err(e)) => {
                error!("Waiting for plugin {} shutdown failed", plugin_name);
                if e.is_panic() {
                    error!("Shutdown of {} paniced", plugin_name);
                } else if e.is_cancelled() {
                    error!("Shutdown of {} cancelled", plugin_name);
                }
                Err(PluginLifecycleError::PluginStopPanicked(
                    PluginStopPanicked {
                        name: plugin_name.to_string(),
                    },
                ))
            }
            Ok(Ok(res)) => {
                info!("Shutting down {} completed", plugin_name);
                res.map_err(|e| {
                    PluginLifecycleError::PluginStopFailed(PluginStopFailed {
                        name: plugin_name.to_string(),
                        error: e,
                    })
                })
            }
        }
    }
}
