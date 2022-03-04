use futures::StreamExt;

use tedge_api::Plugin;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::trace;

use crate::configuration::PluginInstanceConfiguration;
use crate::configuration::PluginKind;
use crate::errors::Result;
use crate::errors::TedgeApplicationError;
use crate::plugin_task::PluginTask;
use crate::task::Task;
use crate::TedgeApplication;

/// Helper type for running a TedgeApplication
///
/// This type is only introduced for more seperation-of-concerns in the codebase
/// `Reactor::run()` is simply `TedgeApplication::run()`.
pub struct Reactor(pub TedgeApplication);

impl std::fmt::Debug for Reactor {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

type Receiver = tokio::sync::mpsc::Receiver<tedge_api::message::Message>;
type Sender = tokio::sync::mpsc::Sender<tedge_api::message::Message>;

/// Helper type for preparing a PluginTask
struct PluginTaskPrep {
    name: String,
    plugin: Box<dyn Plugin>,
    plugin_recv: Receiver,
    task_sender: Sender,
    task_recv: Receiver,
    core_msg_sender: Sender,
    task_cancel_token: CancellationToken,
}

impl Reactor {
    pub async fn run(self) -> Result<()> {
        let buf_size = self.0.config().communication_buffer_size().get();
        let (core_msg_sender, core_msg_recv) = tokio::sync::mpsc::channel(buf_size);

        let instantiated_plugins = self
            .0
            .config()
            .plugins()
            .iter()
            .map(|(pname, pconfig)| {
                self.instantiate_plugin(pname, pconfig, core_msg_sender.clone())
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Vec<Result<_>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        debug!("Plugins instantiated");

        let running_core = {
            let plugin_senders = instantiated_plugins
                .iter()
                .map(|prep| (prep.name.clone(), prep.task_sender.clone()))
                .collect();
            crate::core_task::CoreTask::new(core_msg_recv, plugin_senders).run()
        };
        debug!("Core task instantiated");

        let running_plugins = instantiated_plugins
            .into_iter()
            .map(|prep| {
                PluginTask::new(
                    prep.name,
                    prep.plugin,
                    prep.plugin_recv,
                    prep.task_recv,
                    prep.core_msg_sender,
                    prep.task_cancel_token,
                )
            })
            .map(Task::run)
            .map(Box::pin)
            .collect::<futures::stream::FuturesUnordered<_>>() // main loop
            .collect::<Vec<Result<()>>>();
        debug!("Plugin tasks instantiated");

        debug!("Entering main loop");
        let (plugin_res, core_res) = tokio::join!(running_plugins, running_core);

        plugin_res
            .into_iter() // result type conversion
            .collect::<Result<Vec<()>>>()
            .and_then(|_| core_res)
    }

    fn get_config_for_plugin<'a>(
        &'a self,
        plugin_name: &str,
    ) -> Option<&'a tedge_api::PluginConfiguration> {
        trace!("Searching config for plugin: {}", plugin_name);
        self.0
            .config()
            .plugins()
            .get(plugin_name)
            .map(|cfg| cfg.configuration())
    }

    fn find_plugin_builder<'a>(
        &'a self,
        plugin_kind: &PluginKind,
    ) -> Option<&'a dyn tedge_api::PluginBuilder> {
        trace!("Searching builder for plugin: {}", plugin_kind.as_ref());
        self.0
            .plugin_builders()
            .get(plugin_kind.as_ref())
            .map(AsRef::as_ref)
    }

    async fn instantiate_plugin(
        &self,
        plugin_name: &str,
        plugin_config: &PluginInstanceConfiguration,
        core_msg_sender: Sender,
    ) -> Result<PluginTaskPrep> {
        let builder = self
            .find_plugin_builder(plugin_config.kind())
            .ok_or_else(|| {
                let kind_name = plugin_config.kind().as_ref().to_string();
                TedgeApplicationError::UnknownPluginKind(kind_name)
            })?;

        let config = self.get_config_for_plugin(plugin_name).ok_or_else(|| {
            let pname = plugin_name.to_string();
            TedgeApplicationError::PluginConfigMissing(pname)
        })?;

        let buf_size = self.0.config().communication_buffer_size().get();
        let (plugin_message_sender, plugin_message_receiver) = tokio::sync::mpsc::channel(buf_size);
        let (task_sender, task_receiver) = tokio::sync::mpsc::channel(buf_size);

        // Retreive task cancel token for cancling a task inside the core
        let task_cancel_token = self.0.cancellation_token.child_token();

        // ... and from that a plugin cancel token, that can be used to cancel only the plugin
        let plugin_cancel_token = task_cancel_token.child_token();

        let comms = tedge_api::plugin::CoreCommunication::new(
            plugin_name.to_string(),
            plugin_message_sender,
            plugin_cancel_token,
        );

        trace!(
            "Instantiating plugin: {} of kind {}",
            plugin_name,
            plugin_config.kind().as_ref()
        );
        builder
            .instantiate(config.clone(), comms)
            .await
            .map_err(TedgeApplicationError::from)
            .map(|plugin| PluginTaskPrep {
                name: plugin_name.to_string(),
                plugin,
                plugin_recv: plugin_message_receiver,
                task_sender,
                task_recv: task_receiver,
                core_msg_sender,
                task_cancel_token,
            })
    }
}
