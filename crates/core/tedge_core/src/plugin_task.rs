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
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;
use tracing::Instrument;

use crate::errors::Result;
use crate::errors::TedgeApplicationError;
use crate::message_handler::make_message_handler;

/// Type for handling the lifecycle of one individual Plugin instance
pub struct PluginTask {
    plugin_name: String,
    plugin: Arc<RwLock<BuiltPlugin>>,
    channel_size: usize,
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
        channel_size: usize,
        plugin_msg_communications: MessageSender,
        task_cancel_token: CancellationToken,
        shutdown_timeout: std::time::Duration,
    ) -> Self {
        let panic_channel = channel::<()>(1);
        Self {
            plugin_name,
            plugin,
            channel_size,
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
    pub async fn run_start(&mut self) -> crate::errors::Result<()> {
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
                return Err(TedgeApplicationError::PluginSetupPaniced(
                    plugin_name.to_string(),
                ));
            }
            Ok(res) => {
                res.map_err(|e| {
                    TedgeApplicationError::PluginSetupFailed(plugin_name.to_string(), e)
                })?;
            }
        };

        Ok(())
    }

    pub async fn enable_communications(&self) -> crate::errors::Result<()> {
        trace!(channel_size = ?self.channel_size, "enabling communications");
        self.plugin_msg_communications
            .init_with(make_message_handler(
                Arc::new(Semaphore::new(self.channel_size)),
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
    pub async fn run_main(&self) -> Result<()> {
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
                return Err(TedgeApplicationError::PluginMainPaniced(
                    self.plugin_name.to_string(),
                ));
            }
            Ok(res) => {
                res.map_err(|e| {
                    TedgeApplicationError::PluginMainFailed(self.plugin_name.to_string(), e)
                })?;
            }
        };

        Ok(())
    }

    pub async fn disable_communications(&self) -> crate::errors::Result<()> {
        self.plugin_msg_communications.reset().await;

        Ok(())
    }

    pub async fn run_shutdown(&mut self) -> crate::errors::Result<()> {
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
}

/// Handle the setup phase of the Plugin instance
///
/// This makes sure the [`tedge_api::Plugin::start`] function is called in a crash-safe way. That
/// means if the implementation of that function panics, this will simply return an error and not
/// take down the rest of the application.
///
/// If the starting of the plugin failed, this will error as well, of course.
#[tracing::instrument(skip(plugin, plugin_name))]
async fn plugin_setup(plugin: Arc<RwLock<BuiltPlugin>>, plugin_name: &str) -> Result<()> {
    let mut plug_write = plugin.write().await;

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
            return Err(TedgeApplicationError::PluginSetupPaniced(
                plugin_name.to_string(),
            ));
        }
        Ok(res) => {
            res.map_err(|e| TedgeApplicationError::PluginSetupFailed(plugin_name.to_string(), e))?;
        }
    };

    Ok(())
}

/// Run the "main loop" for the Plugin instance
///
/// This runs the main part of the lifecycle of the Plugin instance: Waiting for messages from
/// other plugins and passing them to the instance handled here in a concurrent way.
///
/// If the application is cancelled by the user (via the CancellationToken), this function takes
/// care of stopping the "main loop" as well and returns cleanly.
#[tracing::instrument(skip_all, name = "plugin_messagehandling")]
async fn plugin_mainloop(
    mut panic_err_recv: Receiver<()>,
    task_cancel_token: CancellationToken,
) -> Result<()> {
    loop {
        tokio::select! {
            panic_err = panic_err_recv.recv() => {
                if let Some(()) = panic_err {
                    break
                }
            }

            _shutdown = task_cancel_token.cancelled() => {
                // no communication happened when we got this future returned,
                // so we're done now
                debug!("Received shutdown request");
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
#[tracing::instrument(skip(plugin, plugin_name))]
async fn plugin_shutdown(
    plugin: Arc<RwLock<BuiltPlugin>>,
    plugin_name: &str,
    shutdown_timeout: std::time::Duration,
) -> Result<()> {
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
