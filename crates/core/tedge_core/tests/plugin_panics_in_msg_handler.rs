use async_trait::async_trait;
use futures::future::FutureExt;
use tedge_api::address::ReplySenderFor;
use tedge_api::plugin::Handle;
use tedge_api::plugin::Message;
use tedge_api::plugin::PluginExt;
use tedge_api::Address;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_core::TedgeApplication;

pub struct HandlePanicPluginBuilder;

#[async_trait::async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for HandlePanicPluginBuilder {
    fn kind_name() -> &'static str {
        "handlepanic"
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
        let self_addr = plugin_dir.get_address_for::<ReceivePanic>("panic")?;
        Ok(HandlePanicPlugin { self_addr }.finish())
    }

    fn kind_message_types() -> tedge_api::plugin::HandleTypes
    where
        Self: Sized,
    {
        HandlePanicPlugin::get_handled_types()
    }
}

tedge_api::make_receiver_bundle!(struct ReceivePanic(DoPanic));

struct HandlePanicPlugin {
    self_addr: Address<ReceivePanic>,
}

impl tedge_api::plugin::PluginDeclaration for HandlePanicPlugin {
    type HandledMessages = (DoPanic,);
}

#[async_trait]
impl Plugin for HandlePanicPlugin {
    async fn start(&mut self) -> Result<(), PluginError> {
        tracing::info!("Setup called");
        let _ = self.self_addr.send_and_wait(DoPanic).await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        tracing::info!("Shutdown called");
        Ok(())
    }
}

#[derive(Debug)]
struct DoPanic;

impl Message for DoPanic {}

#[async_trait::async_trait]
impl Handle<DoPanic> for HandlePanicPlugin {
    async fn handle_message(
        &self,
        _message: DoPanic,
        _sender: ReplySenderFor<DoPanic>,
    ) -> Result<(), PluginError> {
        panic!("Oh noez!")
    }
}

#[test]
fn test_handler_panic() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
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
            .with_plugin_builder(HandlePanicPluginBuilder {})
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

                _ = &mut run_fut => {
                    tracing::info!("application.run() returned");
                    assert!(cancelled, "Application returned but cancel did not happen yet");
                    // cancel happened... everything fine.
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
