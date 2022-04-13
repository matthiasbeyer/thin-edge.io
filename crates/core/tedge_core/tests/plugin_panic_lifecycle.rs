use async_trait::async_trait;
use futures::future::FutureExt;
use miette::IntoDiagnostic;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;
use tedge_core::TedgeApplication;
use tedge_core::configuration::TedgeConfiguration;

pub struct PanicPluginBuilder;

#[derive(Debug, serde::Deserialize)]
struct PanicPluginConf {
    panic_location: PanicLocation,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum PanicLocation {
    Setup,
    Shutdown,
}

#[async_trait::async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for PanicPluginBuilder {
    fn kind_name() -> &'static str {
        "panicplugin"
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into()
            .map(|_: PanicPluginConf| ())
            .into_diagnostic()?;
        Ok(())
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: tedge_api::CancellationToken,
        _plugin_dir: &PD,
    ) -> Result<tedge_api::plugin::BuiltPlugin, PluginError> {
        let config: PanicPluginConf = config
            .try_into()
            .into_diagnostic()?;

        tracing::info!("Config = {:?}", config);

        Ok(PanicPlugin { panic_loc: config.panic_location }.finish())
    }

    fn kind_message_types() -> HandleTypes
        where Self:Sized
    {
        PanicPlugin::get_handled_types()
    }
}

struct PanicPlugin {
    panic_loc: PanicLocation,
}

impl tedge_api::plugin::PluginDeclaration for PanicPlugin {
    type HandledMessages = ();
}

#[async_trait]
impl Plugin for PanicPlugin {
    #[allow(unreachable_code)]
    async fn start(&mut self) -> Result<(), PluginError> {
        tracing::info!("Setup called");
        if let PanicLocation::Setup = self.panic_loc {
            panic!("Oh noez...");
        }
        Ok(())
    }


    async fn shutdown(&mut self) -> Result<(), PluginError> {
        tracing::info!("Shutdown called");
        if let PanicLocation::Shutdown = self.panic_loc {
            panic!("Oh noez...");
        }
        Ok(())
    }
}

#[test]
fn test_setup_panic_plugin() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    let _ = tracing_subscriber::fmt::try_init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let res = rt.block_on(async {
        const CONF: &'static str = r#"
            communication_buffer_size = 10

            plugin_shutdown_timeout_ms = 2000

            [plugins]

            [plugins.panic_at_the_disco_setup]
            kind = "panicplugin"
            [plugins.panic_at_the_disco_setup.configuration]
            panic_location = "setup"

            [plugins.panic_at_the_disco_shutdown]
            kind = "panicplugin"
            [plugins.panic_at_the_disco_shutdown.configuration]
            panic_location = "shutdown"
        "#;

        let config: TedgeConfiguration = toml::de::from_str(CONF).into_diagnostic()?;
        let (cancel_sender, application) = TedgeApplication::builder()
            .with_plugin_builder(PanicPluginBuilder {})
            .into_diagnostic()?
            .with_config(config)
            .into_diagnostic()?;

        let mut run_fut = tokio::spawn(application.run());

        // send a cancel request to the app after 1 sec
        let mut cancel_fut = Box::pin({
            tokio::time::sleep(std::time::Duration::from_secs(1))
                .then(|_| async {
                    tracing::info!("Cancelling app now");
                    cancel_sender.cancel_app()
                })
        });

        // Abort the test after 5 secs, because it seems not to stop the application
        let mut test_abort = Box::pin({
            tokio::time::sleep(std::time::Duration::from_secs(5))
                .then(|_| async {
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
                        Ok(_) => assert!(cancelled, "Application returned but cancel did not happen yet"),
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


