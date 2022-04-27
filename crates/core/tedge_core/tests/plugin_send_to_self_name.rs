use async_trait::async_trait;
use futures::future::FutureExt;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;
use tedge_api::Address;
use tedge_api::CoreMessages;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_core::TedgeApplication;

pub struct SelfSendPluginBuilder;

#[derive(Debug)]
struct Msg;

impl tedge_api::Message for Msg {}

tedge_api::make_receiver_bundle!(struct MsgRecv(Msg));

#[async_trait::async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for SelfSendPluginBuilder {
    fn kind_name() -> &'static str {
        "selfsend"
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
        _cancellation_token: tedge_api::CancellationToken,
        plugin_dir: &PD,
    ) -> Result<tedge_api::plugin::BuiltPlugin, PluginError> {
        let _addr: Address<MsgRecv> = plugin_dir.get_address_for("selfsender")?;
        tracing::info!("Fetching own address worked");
        let core_addr = plugin_dir.get_address_for_core();
        Ok(SelfSendPlugin { core_addr }.finish())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        SelfSendPlugin::get_handled_types()
    }
}

struct SelfSendPlugin {
    core_addr: Address<CoreMessages>,
}

impl tedge_api::plugin::PluginDeclaration for SelfSendPlugin {
    type HandledMessages = (Msg,);
}

#[async_trait]
impl Plugin for SelfSendPlugin {
    #[allow(unreachable_code)]
    async fn main(&self) -> Result<(), PluginError> {
        tracing::info!("Sending StopCore now");
        self.core_addr
            .send_and_wait(tedge_api::message::StopCore)
            .await
            .expect("Sending StopCore failed");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

#[async_trait]
impl tedge_api::plugin::Handle<Msg> for SelfSendPlugin {
    async fn handle_message(
        &self,
        _: Msg,
        _: tedge_api::address::ReplySenderFor<Msg>,
    ) -> Result<(), PluginError> {
        unimplemented!() // will never be called in this test
    }
}

#[test]
fn test_send_to_self_via_name_does_work() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    let _ = tracing_subscriber::fmt::try_init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let res = rt.block_on(async {
        let config_file_path = {
            let dir = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .join("../../../");
            let mut name = std::path::PathBuf::from(std::file!());
            name.set_extension("toml");
            let filepath = dir.join(name);
            assert!(
                filepath.exists(),
                "Config file does not exist: {}",
                filepath.display()
            );
            filepath
        };

        let (cancel_sender, application) = TedgeApplication::builder()
            .with_plugin_builder(SelfSendPluginBuilder {})
            .with_config_from_path(config_file_path)
            .await?;

        let mut run_fut = tokio::spawn(application.run());

        // send a cancel request to the app after 1 sec
        let mut cancel_fut = Box::pin({
            tokio::time::sleep(std::time::Duration::from_secs(1)).then(|_| async {
                tracing::info!("Cancelling app now");
                cancel_sender.cancel_app()
            })
        });

        // Abort the test after 5 secs, because it seems not to stop the application
        let mut test_abort = Box::pin({
            tokio::time::sleep(std::time::Duration::from_secs(5)).then(|_| async {
                tracing::info!("Aborting test");
            })
        });

        let mut cancelled = false;
        loop {
            tracing::info!("Looping");
            tokio::select! {
                _test_abort = &mut test_abort => {
                    tracing::error!("Test aborted");
                    run_fut.abort();
                    miette::bail!("Timeout reached, shutdown did not happen")
                },

                app_res = &mut run_fut => {
                    tracing::info!("application.run() returned");
                    match app_res {
                        Err(e) => {
                            miette::bail!("Application errored: {:?}", e);
                        }
                        Ok(_) => assert!(!cancelled, "Application had to be cancelled"),
                    }
                    // cancel happened... everything fine.
                    tracing::info!("Application shutdown clean");
                    break;
                },

                _ = &mut cancel_fut, if !cancelled => {
                    tracing::info!("Cancellation happened...");
                    cancelled = true;
                }
            }
        }

        Ok(())
    });

    rt.shutdown_background();
    if let Err(e) = res {
        panic!("{e:?}");
    }
    Ok(())
}
