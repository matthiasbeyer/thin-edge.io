use std::path::Path;
use std::sync::Arc;

use futures::StreamExt;

use itertools::Itertools;
use tedge_api::message::MessageType;
use tedge_api::plugin::BuiltPlugin;
use tedge_api::PluginExt;
use tokio::sync::mpsc::channel;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::info_span;
use tracing::trace;
use tracing::trace_span;
use tracing::warn;
use tracing::Instrument;

use crate::communication::CorePluginDirectory;
use crate::communication::PluginDirectory;
use crate::communication::PluginInfo;
use crate::configuration::InstanceConfiguration;
use crate::configuration::PluginInstanceConfiguration;
use crate::configuration::PluginKind;
use crate::core_task::CoreInternalMessage;
use crate::core_task::CorePlugin;
use crate::errors::PluginBuilderInstantiationError;

use crate::errors::PluginConfigurationNotFoundError;
use crate::errors::PluginInstantiationError;
use crate::errors::PluginKindUnknownError;

use crate::errors::TedgeApplicationError;
use crate::plugin_task::PluginTask;
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
    channel_size: usize,
    plugin_msg_comms: tedge_api::address::MessageSender,
    cancellation_token: CancellationToken,
}

impl Reactor {
    /// Run the application
    ///
    /// This function implements running the application. That includes the complete lifecycle of
    /// the application, the plugins that need to be started and stopped accordingly as well as
    /// crash safety concerns.
    pub async fn run(self) -> Result<(), TedgeApplicationError> {
        let channel_size = self.0.config().communication_buffer_size().get();

        // find all PluginBuilder objects that are registered and specified in the configuration to
        // be used to build Plugin instances with.
        //
        // This is then collected into a CorePluginDirectory, our "addressbook type" that can be
        // used to retrieve addresses for message passing.
        let mut directory = tracing::debug_span!("core.build_plugin_directory").in_scope(|| {
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
                        PluginInstantiationError::KindNotFound(PluginKindUnknownError {
                            name: pconfig.kind().as_ref().to_string(),
                            alternatives: None,
                        })
                    })?;

                Ok((
                    pname.to_string(),
                    PluginInfo::new(handle_types, channel_size),
                ))
            });

            CorePluginDirectory::collect_from(directory_iter)
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
                            .map(|pinfo| pinfo.communicator.clone())
                        {
                            Some(receiver) => receiver,
                            None => unreachable!(
                            "Could not find existing plugin. This is a FATAL bug, please report it"
                        ),
                        };

                        (pname, pconfig, receiver)
                    })
                    .collect::<Vec<_>>()
            });

        let directory = Arc::new(directory);

        // ... and then instantiate the plugins requested by the user
        let (mut instantiated_plugins, failed_instantiations): (Vec<PluginTaskPrep>, Vec<_>) =
            plugin_instantiation_prep
                .into_iter()
                .map(|(pname, pconfig, communicator)| {
                    {
                        self.instantiate_plugin(
                            pname,
                            self.0.config_path(),
                            pconfig,
                            directory.clone(),
                            communicator,
                            self.0.cancellation_token().child_token(),
                        )
                    }
                    .instrument(info_span!("plugin.instantiate", name = %pname))
                })
                .collect::<futures::stream::FuturesUnordered<_>>()
                .collect::<Vec<Result<_, _>>>()
                .instrument(tracing::debug_span!("core.plugin_instantiation"))
                .await
                .into_iter()
                .partition_result();
        trace!("Plugins instantiated");

        if !failed_instantiations.is_empty() {
            return Err(TedgeApplicationError::PluginInstantiationsError {
                errors: failed_instantiations,
            });
        }

        // Now we need to make sure we start the "CoreTask", which is responsible for handling the
        // communication within the core itself.
        let (internal_sender, mut internal_receiver) = channel(10);
        let core_plugin = CorePlugin::new(internal_sender);
        instantiated_plugins.push(PluginTaskPrep {
            name: "core".to_string(),
            plugin: core_plugin.finish(),
            channel_size: 10,
            plugin_msg_comms: directory.get_core_communicator(),
            cancellation_token: self.0.cancellation_token.clone(),
        });

        debug!("Core task instantiated");

        let mut all_plugins: Vec<PluginTask> = instantiated_plugins
            .into_iter()
            .map(|prep| {
                let timeout = self.0.config().plugin_shutdown_timeout();
                let plugin = Arc::new(RwLock::new(prep.plugin));
                PluginTask::new(
                    prep.name,
                    plugin,
                    prep.channel_size,
                    prep.plugin_msg_comms,
                    prep.cancellation_token,
                    timeout,
                )
            })
            .collect();

        // TODO: Handle these errors

        debug!("Running 'start' for plugins");
        let _start_results = all_plugins
            .iter_mut()
            .map(|plugin_task| {
                let span =
                    tracing::debug_span!("plugin.start", plugin.name = %plugin_task.plugin_name());
                plugin_task.run_start().instrument(span)
            })
            .collect::<futures::stream::FuturesOrdered<_>>()
            .collect::<Vec<Result<(), _>>>()
            .instrument(tracing::info_span!("core.mainloop.plugins.start"))
            .await;

        debug!("Enabling communications for plugins");
        all_plugins
            .iter()
            .map(|plugin_task| {
                let span =
                    tracing::debug_span!("plugin.enable_communication", plugin.name = %plugin_task.plugin_name());
                plugin_task.enable_communications().instrument(span)
            })
            .collect::<futures::stream::FuturesOrdered<_>>()
            .collect::<Vec<Result<(), _>>>()
            .instrument(tracing::info_span!(
                "core.mainloop.plugins.enable-communications"
            ))
            .await;

        debug!("Running 'main' for plugins");
        let _main_results = all_plugins
            .iter_mut()
            .map(|plugin_task| {
                let span =
                    tracing::debug_span!("plugin.main", plugin.name = %plugin_task.plugin_name());
                plugin_task.run_main().instrument(span)
            })
            .collect::<futures::stream::FuturesOrdered<_>>()
            .collect::<Vec<Result<(), _>>>()
            .instrument(tracing::info_span!("core.mainloop.plugins.main"))
            .await;

        // And now we wait until all communication is finished.
        //
        // There are two ways how this could return: Either one plugin requests the core to shut
        // down, which it then will, or the user requests a shutdown via Sigint (Ctrl-C), which
        // notifies the cancellation tokens in the application and plugins.
        loop {
            tokio::select! {
                _cancel = self.0.cancellation_token.cancelled() => {
                    debug!("Cancelled main loop");
                    break;
                },

                internal_message = internal_receiver.recv() => {
                    trace!("Received message");
                    match internal_message {
                        msg @ None | msg @ Some(CoreInternalMessage::Stop) => {
                            if msg.is_none() {
                                warn!("Internal core communication stopped");
                            }
                            debug!("Cancelling cancellation token to stop plugins");
                            self.0.cancellation_token.cancel();
                            debug!("Stopping core");
                            break;
                        }
                    }
                },
            }
        }

        debug!("Disabling communications for plugins");
        all_plugins
            .iter()
            .map(|plugin_task| {
                let span =
                    tracing::debug_span!("plugin.disable_communication", plugin.name = %plugin_task.plugin_name());
                plugin_task.disable_communications().instrument(span)
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Vec<Result<(), _>>>()
            .instrument(tracing::info_span!(
                "core.mainloop.plugins.disable-communications"
            ))
            .await;

        debug!("Running 'shutdown' for plugins");
        let _shutdown_results = all_plugins
            .iter_mut()
            .map(|plugin_task| {
                let span =
                    tracing::debug_span!("plugin.shutdown", plugin.name = %plugin_task.plugin_name());
                plugin_task.run_shutdown().instrument(span)
            })
            .collect::<futures::stream::FuturesOrdered<_>>()
            .collect::<Vec<Result<(), _>>>()
            .instrument(tracing::info_span!("core.mainloop.plugins.shutdown"))
            .await;

        Ok(())
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
        plugin_msg_comms: tedge_api::address::MessageSender,
        cancellation_token: CancellationToken,
    ) -> Result<PluginTaskPrep, PluginInstantiationError> {
        let builder = self
            .find_plugin_builder(plugin_config.kind())
            .ok_or_else(|| {
                let kind_name = plugin_config.kind().as_ref().to_string();
                PluginInstantiationError::KindNotFound(PluginKindUnknownError {
                    name: kind_name,
                    alternatives: None,
                })
            })?;

        let config = self.get_config_for_plugin(plugin_name).ok_or_else(
            || -> PluginInstantiationError {
                let pname = plugin_name.to_string();
                PluginInstantiationError::ConfigurationNotFound(PluginConfigurationNotFoundError {
                    name: pname,
                })
            },
        )?;

        let config = match config
            .verify_with_builder(plugin_name, builder, root_config_path)
            .instrument(trace_span!("core.config_verification"))
            .await
        {
            Err(e) => {
                error!(
                    "Verification of configuration failed for plugin '{}'",
                    plugin_name
                );
                return Err(PluginInstantiationError::ConfigurationVerificationFailed(e))?;
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
            .map(|plugin| {
                trace!(plugin.name = ?plugin_name, "Instantiation of plugin successfull");

                let channel_size = self.0.config().communication_buffer_size().get();

                PluginTaskPrep {
                    name: plugin_name.to_string(),
                    plugin,
                    channel_size,
                    plugin_msg_comms,
                    cancellation_token,
                }
            })
            .map_err(|e| {
                PluginInstantiationError::BuilderInstantiation(PluginBuilderInstantiationError {
                    name: plugin_name.to_string(),
                    error: e,
                })
            })
    }
}
