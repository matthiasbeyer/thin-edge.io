use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures::FutureExt;
use tedge_api::{
    address::ReplySenderFor,
    message::{AcceptsReplies, Message, MessageType},
    plugin::{BuiltPlugin, Handle, PluginDeclaration, PluginExt},
    Address, CancellationToken, Plugin, PluginBuilder, PluginConfiguration, PluginDirectory,
    PluginError,
};
use tokio::sync::RwLock;
use type_uuid::TypeUuid;

#[derive(Debug, TypeUuid)]
#[uuid = "94916be9-17ba-4bca-a3a0-408d33136fed"]
/// A message that represents a heartbeat that gets sent to plugins
struct Heartbeat;
impl Message for Heartbeat {}
impl AcceptsReplies for Heartbeat {
    type Reply = HeartbeatStatus;
}

#[derive(Debug, TypeUuid)]
#[uuid = "a6d03c65-51bf-4f89-b383-c67c9ed8533b"]
/// The reply for a heartbeat
enum HeartbeatStatus {
    Alive,
    Degraded,
}
impl Message for HeartbeatStatus {}

/// A PluginBuilder that gets used to build a HeartbeatService plugin instance
#[derive(Debug)]
struct HeartbeatServiceBuilder;

#[derive(miette::Diagnostic, thiserror::Error, Debug)]
enum HeartbeatBuildError {
    #[error(transparent)]
    TomlParse(#[from] toml::de::Error),
}

#[async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for HeartbeatServiceBuilder {
    fn kind_name() -> &'static str {
        todo!()
    }

    fn kind_message_types() -> tedge_api::plugin::HandleTypes
    where
        Self: Sized,
    {
        HeartbeatService::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        _config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        Ok(())
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError>
    where
        PD: 'async_trait,
    {
        let hb_config: HeartbeatConfig =
            toml::Value::try_into(config).map_err(HeartbeatBuildError::from)?;
        let monitored_services = hb_config
            .plugins
            .iter()
            .map(|name| {
                plugin_dir
                    .get_address_for::<HeartbeatMessages>(name)
                    .map(|addr| (name.clone(), addr))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(HeartbeatService::new(
            Duration::from_millis(hb_config.interval),
            monitored_services,
            cancellation_token,
        )
        .finish())
    }
}

/// The configuration a HeartbeatServices can receive is represented by this type
#[derive(serde::Deserialize, Debug)]
struct HeartbeatConfig {
    interval: u64,
    plugins: Vec<String>,
}

/// The HeartbeatService type represents the actual plugin
struct HeartbeatService {
    interval_duration: Duration,
    monitored_services: Vec<(String, Address<HeartbeatMessages>)>,
    cancel_token: CancellationToken,
}

impl PluginDeclaration for HeartbeatService {
    type HandledMessages = ();
}

#[async_trait]
impl Plugin for HeartbeatService {
    /// The setup function of the HeartbeatService can be used by the plugin author to setup for
    /// example a connection to an external service. In this example, it is simply used to send the
    /// heartbeat
    ///
    /// Because this example is _simple_, we do not spawn a background task that periodically sends
    /// the heartbeat. In a real world scenario, that background task would be started here.
    async fn start(&mut self) -> Result<(), PluginError> {
        println!(
            "HeartbeatService: Setting up heartbeat service with interval: {:?}!",
            self.interval_duration
        );

        for service in &self.monitored_services {
            let mut interval = tokio::time::interval(self.interval_duration);
            let service = service.clone();
            let cancel_token = self.cancel_token.child_token();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = interval.tick() => {}
                        _ = cancel_token.cancelled() => {
                            break
                        }
                    }
                    println!(
                        "HeartbeatService: Sending heartbeat to service: {:?}",
                        service
                    );
                    tokio::select! {
                        reply = service
                        .1
                        .send_and_wait(Heartbeat)
                        .then(|answer| {
                            answer.unwrap()
                            .wait_for_reply(Duration::from_millis(100))}
                        ) => {
                            match reply
                            {
                                Ok(HeartbeatStatus::Alive) => {
                                    println!("HeartbeatService: Received all is well!")
                                }
                                Ok(HeartbeatStatus::Degraded) => {
                                    println!(
                                        "HeartbeatService: Oh-oh! Plugin '{}' is not doing well",
                                        service.0
                                        )
                                }

                                Err(reply_error) => {
                                    println!(
                                        "HeartbeatService: Critical error for '{}'! {reply_error}",
                                        service.0
                                        )
                                }
                            }
                        }

                        _ = cancel_token.cancelled() => {
                            break
                        }
                    }
                }
            });
        }
        Ok(())
    }

    /// A plugin author can use this shutdown function to clean resources when thin-edge shuts down
    async fn shutdown(&mut self) -> Result<(), PluginError> {
        println!("HeartbeatService: Shutting down heartbeat service!");
        Ok(())
    }
}

impl HeartbeatService {
    fn new(
        interval_duration: Duration,
        monitored_services: Vec<(String, Address<HeartbeatMessages>)>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            interval_duration,
            monitored_services,
            cancel_token,
        }
    }
}

/// A plugin that receives heartbeats
struct CriticalServiceBuilder;

// declare a set of messages that the CriticalService can receive.
// In this example, it can only receive a Heartbeat.
tedge_api::make_receiver_bundle!(struct HeartbeatMessages(Heartbeat));

#[async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for CriticalServiceBuilder {
    fn kind_name() -> &'static str {
        todo!()
    }

    fn kind_message_types() -> tedge_api::plugin::HandleTypes
    where
        Self: Sized,
    {
        CriticalService::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        _config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        Ok(())
    }

    async fn instantiate(
        &self,
        _config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        _plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError>
    where
        PD: 'async_trait,
    {
        Ok(CriticalService {
            status: tokio::sync::Mutex::new(true),
        }
        .finish())
    }
}

/// The actual "critical" plugin implementation
struct CriticalService {
    status: tokio::sync::Mutex<bool>,
}

/// The CriticalService can receive Heartbeat objects, thus it needs a Handle<Heartbeat>
/// implementation
#[async_trait]
impl Handle<Heartbeat> for CriticalService {
    async fn handle_message(
        &self,
        _message: Heartbeat,
        sender: ReplySenderFor<Heartbeat>,
    ) -> Result<(), PluginError> {
        println!("CriticalService: Received Heartbeat!");
        let mut status = self.status.lock().await;

        let _ = sender.reply(if *status {
            println!("CriticalService: Sending back alive!");
            HeartbeatStatus::Alive
        } else {
            println!("CriticalService: Sending back degraded!");
            HeartbeatStatus::Degraded
        });

        *status = !*status;
        Ok(())
    }
}

impl PluginDeclaration for CriticalService {
    type HandledMessages = (Heartbeat,);
}

/// Because the CriticalService is of course a Plugin, it needs an implementation for that as well.
#[async_trait]
impl Plugin for CriticalService {
    async fn start(&mut self) -> Result<(), PluginError> {
        println!("CriticalService: Setting up critical service!");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        println!("CriticalService: Shutting down critical service service!");
        Ok(())
    }
}

//
// The following pieces of code would be implemented by a "core" component, that is responsible for
// setting up plugins and their communication.
//
// Plugin authors do not need to write this code, but need a basic understanding what it does and
// how it works.
// As this is an example, we implement it here to showcase how it is done.
//

/// Helper type for keeping information about plugins during runtime
struct PluginInfo {
    types: Vec<MessageType>,
    sender: Arc<RwLock<Option<Box<tedge_api::address::MessageFutureProducer>>>>,
}

impl std::fmt::Debug for PluginInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginInfo")
            .field("types", &self.types)
            .finish_non_exhaustive()
    }
}

/// The type that provides the communication infrastructure to the plugins.
#[derive(Debug)]
struct Communication {
    plugins: HashMap<String, PluginInfo>,
}

impl Communication {
    pub fn declare_plugin<PB: PluginBuilder<Self>>(&mut self, name: &str) {
        let sender = Arc::new(RwLock::new(None));
        self.plugins.insert(
            name.to_owned(),
            PluginInfo {
                types: PB::kind_message_types().into_types(),
                sender,
            },
        );
    }
}

impl PluginDirectory for Communication {
    fn get_address_for<MB: tedge_api::address::ReceiverBundle>(
        &self,
        name: &str,
    ) -> Result<Address<MB>, tedge_api::error::DirectoryError> {
        let asked_types: Vec<_> = MB::get_ids().into_iter().collect();

        let plug = self.plugins.get(name).unwrap_or_else(|| {
            // This is an example, so we panic!() here.
            // In real-world, we would do some reporting and return an error
            panic!(
                "Didn't find plugin with name {}, got: {:?}",
                name,
                self.plugins.keys().collect::<Vec<_>>()
            )
        });

        if !asked_types
            .iter()
            .all(|req_type| plug.types.iter().any(|ty| ty.satisfy(req_type)))
        {
            // This is an example, so we panic!() here
            // In real-world, we would do some reporting and return an error
            panic!(
                "Asked for {:#?} but plugin {} only has types {:#?}",
                asked_types, name, plug.types,
            );
        } else {
            Ok(Address::new(tedge_api::address::InnerMessageSender::new(
                plug.sender.clone(),
            )))
        }
    }

    fn get_address_for_core(&self) -> Address<tedge_api::CoreMessages> {
        todo!()
    }
}

/// Helper function
async fn build_critical_plugin(
    comms: &mut Communication,
    cancel_token: CancellationToken,
) -> BuiltPlugin {
    let csb = CriticalServiceBuilder;

    let config = toml::from_str("").unwrap();

    csb.instantiate(config, cancel_token, comms).await.unwrap()
}

/// Helper function
async fn build_heartbeat_plugin(
    comms: &mut Communication,
    cancel_token: CancellationToken,
) -> BuiltPlugin {
    let hsb = HeartbeatServiceBuilder;

    let config = toml::from_str(
        r#"
    interval = 5000
    plugins = ["critical-service"]
    "#,
    )
    .unwrap();

    hsb.instantiate(config, cancel_token, comms).await.unwrap()
}

#[tokio::main]
async fn main() {
    // This implementation now ties everything together
    //
    // This would be implemented in a CLI binary using the "core" implementation to boot things up.
    //
    // Here, we just tie everything together in the minimal possible way, to showcase how such a
    // setup would basically work.

    let mut comms = Communication {
        plugins: HashMap::new(),
    };

    // in a main(), the core would be told what plugins are available.
    // This would, in a real-world scenario, not happen on the "communication" type directly.
    // Still, this needs to be done by a main()-author.
    comms.declare_plugin::<CriticalServiceBuilder>("critical-service");
    comms.declare_plugin::<HeartbeatServiceBuilder>("heartbeat");

    // The following would all be handled by the core implementation, a main() author would only
    // need to call some kind of "run everything" function

    let cancel_token = CancellationToken::new();

    let mut heartbeat = Arc::new(RwLock::new(
        build_heartbeat_plugin(&mut comms, cancel_token.child_token()).await,
    ));
    let mut critical_service = Arc::new(RwLock::new(
        build_critical_plugin(&mut comms, cancel_token.child_token()).await,
    ));

    heartbeat.write().await.plugin_mut().start().await.unwrap();
    critical_service
        .write()
        .await
        .plugin_mut()
        .start()
        .await
        .unwrap();

    let recv = comms.plugins.get("heartbeat").unwrap();

    {
        let mut lock = recv.sender.write().await;
        let heartbeat = heartbeat.clone();

        *lock = Some(Box::new(move |msg, _wait_kind| {
            let heartbeat = heartbeat.clone();
            async move {
                let heartbeat = heartbeat.read().await;
                heartbeat.handle_message(msg).await;
                Ok(())
            }
            .boxed()
        }));
    }

    let recv = comms.plugins.get("critical-service").unwrap();

    {
        let mut lock = recv.sender.write().await;
        let critical_service = critical_service.clone();

        *lock = Some(Box::new(move |msg, _wait_kind| {
            let critical_service = critical_service.clone();
            async move {
                let critical_service = critical_service.read().await;
                critical_service.handle_message(msg).await;
                Ok(())
            }
            .boxed()
        }));
    }

    println!("Core: Stopping everything in 10 seconds!");
    tokio::time::sleep(Duration::from_secs(12)).await;

    println!("Core: SHUTTING DOWN");
    cancel_token.cancel();

    heartbeat
        .write()
        .await
        .plugin_mut()
        .shutdown()
        .await
        .unwrap();
    critical_service
        .write()
        .await
        .plugin_mut()
        .shutdown()
        .await
        .unwrap();

    println!("Core: Shut down");
}
