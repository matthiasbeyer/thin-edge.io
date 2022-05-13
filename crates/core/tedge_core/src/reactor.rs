use std::path::Path;
use std::sync::Arc;

use futures::StreamExt;

use tedge_api::message::MessageType;
use tedge_api::plugin::BuiltPlugin;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::info_span;
use tracing::trace;
use tracing::trace_span;
use tracing::Instrument;

use crate::communication::CorePluginDirectory;
use crate::communication::PluginDirectory;
use crate::communication::PluginInfo;
use crate::configuration::InstanceConfiguration;
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
pub struct Reactor(pub(crate) TedgeApplication);

impl std::fmt::Debug for Reactor {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Helper type for preparing a PluginTask
struct PluginTaskPrep {
    name: String,
    plugin: BuiltPlugin,
    plugin_msg_receiver: tedge_api::address::MessageReceiver,
    cancellation_token: CancellationToken,
}

impl Reactor {
    /// Run the application
    ///
    /// This function implements running the application. That includes the complete lifecycle of
    /// the application, the plugins that need to be started and stopped accordingly as well as
    /// crash safety concerns.
    pub async fn run(self) -> Result<()> {
        let channel_size = self.0.config().communication_buffer_size().get();

        // find all PluginBuilder objects that are registered and specified in the configuration to
        // be used to build Plugin instances with.
        //
        // This is then collected into a CorePluginDirectory, our "addressbook type" that can be
        // used to retrieve addresses for message passing.
        let (core_sender, core_receiver) = tokio::sync::mpsc::channel(channel_size);
        let mut directory = tracing::debug_span!("core.build_plugin_directory")
            .in_scope(|| {
                let directory_iter = self.0.config().plugins().iter().map(|(pname, pconfig)| {
                    // fetch the types the plugin claims to handle from the plugin builder identified
                    // by the "kind" in the configuration of the instance
                    let handle_types = self
                        .0
                        .plugin_builders()
                        .get(pconfig.kind().as_ref())
                        .map(|(handle_types, _)| {
                            handle_types
                                .get_types()
                                .into_iter()
                                .cloned()
                                .collect::<Vec<MessageType>>()
                        })
                        .ok_or_else(|| {
                            TedgeApplicationError::UnknownPluginKind(
                                pconfig.kind().as_ref().to_string(),
                            )
                        })?;

                    Ok((
                        pname.to_string(),
                        PluginInfo::new(handle_types, channel_size),
                    ))
                });

                CorePluginDirectory::collect_from(directory_iter, core_sender)
            })?;

        // Start preparing the plugin instantiation...
        let plugin_instantiation_prep = tracing::debug_span!("core.plugin_instantiation_prep")
            .in_scope(|| {
                self.0
                    .config()
                    .plugins()
                    .iter()
                    .map(|(pname, pconfig)| {
                        let receiver = match directory
                            .get_mut(pname)
                            .and_then(|pinfo| pinfo.receiver.take())
                        {
                            Some(receiver) => receiver,
                            None => unreachable!(
                            "Tried to take receiver twice. This is a FATAL bug, please report it"
                        ),
                        };

                        (pname, pconfig, receiver)
                    })
                    .collect::<Vec<_>>()
            });

        let directory = Arc::new(directory);

        // ... and then instantiate the plugins requested by the user
        let instantiated_plugins = plugin_instantiation_prep
            .into_iter()
            .map(|(pname, pconfig, receiver)| {
                {
                    self.instantiate_plugin(
                        pname,
                        self.0.config_path(),
                        pconfig,
                        directory.clone(),
                        receiver,
                        self.0.cancellation_token().child_token(),
                    )
                }
                .instrument(info_span!("plugin.instantiate", name = %pname))
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Vec<Result<_>>>()
            .instrument(tracing::debug_span!("core.plugin_instantiation"))
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        trace!("Plugins instantiated");

        // Now we need to make sure we start the "CoreTask", which is responsible for handling the
        // communication within the core itself.
        let running_core = {
            // we clone the cancellation_token here, because the core must be able to use the
            // "root" token to stop all plugins
            let core_cancel_token = self.0.cancellation_token().clone();
            crate::core_task::CoreTask::new(core_cancel_token, core_receiver)
                .run()
                .instrument(tracing::info_span!("core.mainloop.coretask"))
        };
        debug!("Core task instantiated");

        // ...and of course we need to start all the plugins.
        let running_plugins = instantiated_plugins
            .into_iter()
            .map(|prep| {
                PluginTask::new(
                    prep.name,
                    prep.plugin,
                    prep.plugin_msg_receiver,
                    prep.cancellation_token,
                    self.0.config().plugin_shutdown_timeout(),
                )
            })
            .map(|plug_task| {
                let plugin_name = plug_task.plugin_name().to_string();
                plug_task
                    .run()
                    .instrument(info_span!("plugin.mainloop", plugin.name = %plugin_name))
            })
            .map(Box::pin)
            .collect::<futures::stream::FuturesUnordered<_>>() // main loop
            .collect::<Vec<Result<()>>>()
            .instrument(tracing::info_span!("core.mainloop.plugins"));
        trace!("Plugin tasks instantiated");

        // and then we wait until all communication is finished.
        //
        // There are two ways how this could return: Either one plugin requests the core to shut
        // down, which it then will, or the user requests a shutdown via Sigint (Ctrl-C), which
        // notifies the cancellation tokens in the application and plugins.
        let (plugin_res, core_res) = tokio::join!(running_plugins, running_core);

        // After we finished the run, we collect all results and return them to the caller
        plugin_res
            .into_iter() // result type conversion
            .collect::<Result<Vec<()>>>()
            .and_then(|_| core_res)
    }

    fn get_config_for_plugin<'a>(&'a self, plugin_name: &str) -> Option<&'a InstanceConfiguration> {
        trace!(
            plugin.name = plugin_name,
            "Searching config for plugin instance"
        );
        self.0
            .config()
            .plugins()
            .get(plugin_name)
            .map(|cfg| cfg.configuration())
    }

    fn find_plugin_builder<'a>(
        &'a self,
        plugin_kind: &PluginKind,
    ) -> Option<&'a Box<dyn tedge_api::PluginBuilder<PluginDirectory>>> {
        trace!(
            plugin.kind = plugin_kind.as_ref(),
            "Searching builder for plugin kind"
        );
        self.0
            .plugin_builders()
            .get(plugin_kind.as_ref())
            .map(|(_, pb)| pb)
    }

    async fn instantiate_plugin(
        &self,
        plugin_name: &str,
        root_config_path: &Path,
        plugin_config: &PluginInstanceConfiguration,
        directory: Arc<CorePluginDirectory>,
        plugin_msg_receiver: tedge_api::address::MessageReceiver,
        cancellation_token: CancellationToken,
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

        let config = match config
            .verify_with_builder(builder, root_config_path)
            .instrument(trace_span!("core.config_verification"))
            .await
        {
            Err(e) => {
                error!(
                    "Verification of configuration failed for plugin '{}'",
                    plugin_name
                );
                return Err(e);
            }
            Ok(cfg) => cfg,
        };

        let cancel_token = self.0.cancellation_token.child_token();

        builder
            .instantiate(
                config.clone(),
                cancel_token,
                &directory.for_plugin_named(plugin_name),
            )
            .instrument(trace_span!(
                "core.instantiate_plugins",
                name = plugin_name,
                kind = plugin_config.kind().as_ref(),
            ))
            .await
            .map_err(TedgeApplicationError::PluginInstantiationFailed)
            .map(|plugin| {
                trace!("Instantiation of plugin '{}' successfull", plugin_name);

                PluginTaskPrep {
                    name: plugin_name.to_string(),
                    plugin,
                    plugin_msg_receiver,
                    cancellation_token,
                }
            })
    }
}
