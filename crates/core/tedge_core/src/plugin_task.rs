use tedge_api::message::Message;
use tedge_api::Plugin;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::errors::Result;
use crate::errors::TedgeApplicationError;
use crate::task::Task;

type Sender = tokio::sync::mpsc::Sender<Message>;
type Receiver = tokio::sync::mpsc::Receiver<Message>;

pub struct PluginTask {
    plugin_name: String,
    plugin: Box<dyn Plugin>,
    plugin_message_receiver: Receiver,
    tasks_receiver: Receiver,
    core_msg_sender: Sender,
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
        plugin: Box<dyn Plugin>,
        plugin_message_receiver: Receiver,
        tasks_receiver: Receiver,
        core_msg_sender: Sender,
        task_cancel_token: CancellationToken,
    ) -> Self {
        Self {
            plugin_name,
            plugin,
            plugin_message_receiver,
            tasks_receiver,
            core_msg_sender,
            task_cancel_token,
        }
    }

    async fn receive_only_from_other_tasks(mut self) -> Result<()> {
        loop {
            let stop = tokio::select! {
                _shutdown = self.task_cancel_token.cancelled() => {
                    debug!("Received shutdown request");
                    // Shutting down
                    true
                },

                next_message = self.tasks_receiver.recv() => match next_message {
                    Some(msg) => !self.handle_message_to_plugin(msg).await?,
                    None => true
                },
            };

            if stop {
                break;
            }
        }

        debug!("Shutting down plugin");
        self.plugin
            .shutdown()
            .await
            .map_err(TedgeApplicationError::from)
    }

    async fn handle_message_from_plugin(&mut self, msg: Message) -> Result<()> {
        debug!("Received message from plugin {}", self.plugin_name);
        self.core_msg_sender
            .send(msg)
            .await
            .map_err(TedgeApplicationError::from)
    }

    async fn handle_message_to_plugin(&mut self, msg: Message) -> Result<bool> {
        debug!("Sending message to plugin {}", self.plugin_name);
        let mut handle_msg_fut = self.plugin.handle_message(msg);
        tokio::select! {
            _shutdown = self.task_cancel_token.cancelled() => {
                // handling shutdown. Waiting for current message to be handled and then we are
                // done here
                debug!("Received shutdown request");
                handle_msg_fut.await?;
                Ok(false)
            }
            res = &mut handle_msg_fut => res.map(|_| true).map_err(TedgeApplicationError::from),
        }
    }
}

#[async_trait::async_trait]
impl Task for PluginTask {
    #[tracing::instrument]
    async fn run(mut self) -> Result<()> {
        self.plugin.setup().await?;

        loop {
            tokio::select! {
                message_from_plugin = self.plugin_message_receiver.recv() => if let Some(msg) = message_from_plugin {
                    debug!("Received message from the plugin that should be passed to another PluginTask");
                    self.handle_message_from_plugin(msg).await?;
                } else {
                    // If the plugin_message_receiver is closed, the plugin cannot send messages to
                    // thin-edge.
                    //
                    // This means we continue to receive only messages from other tasks and send it
                    // to the plugin, until all communication with this PluginTask is finished and
                    // then return from PluginTask::run()
                    //
                    // This is implemented in a helper function that is called here
                    debug!("Communication has been closed by the plugin. Continuing to only send messages to the plugin");
                    return self.receive_only_from_other_tasks().await
                },

                message_to_plugin = self.tasks_receiver.recv() => if let Some(msg) = message_to_plugin {
                    debug!("Received message that should be passed to the plugin");
                    let continue_processing = self.handle_message_to_plugin(msg).await?;
                    if !continue_processing {
                        break;
                    }
                } else {
                    // If the communication _to_ this PluginTask is closed, there _cannot_ be any
                    // more communication _to_ the plugin.
                    // This means we shut down.
                    debug!("Communication has been closed by the other PluginTask instances");
                    break
                },

                _shutdown = self.task_cancel_token.cancelled() => {
                    // no communication happened when we got this future returned,
                    // so we're done now
                    debug!("Received shutdown request");
                    break
                }
            }
        }

        debug!("Shutting down plugin");
        self.plugin
            .shutdown()
            .await
            .map_err(TedgeApplicationError::from)
    }
}
