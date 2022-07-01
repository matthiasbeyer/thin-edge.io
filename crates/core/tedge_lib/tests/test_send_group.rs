use futures::FutureExt;
use tedge_core::TedgeApplication;

mod msg {
    #[derive(Debug, Clone, bevy_reflect::TypeUuid)]
    #[uuid = "4e131f31-7056-4bb9-b0a6-aa2ca398facd"]
    pub struct Usize(pub usize);
    impl tedge_api::Message for Usize {}
}

mod send {
    extern crate tedge_lib;

    use async_trait::async_trait;
    use futures::stream::StreamExt;
    use tedge_api::message::StopCore;
    use tedge_api::plugin::PluginExt;
    use tedge_api::Plugin;
    use tedge_api::PluginBuilder;
    use tedge_api::PluginConfiguration;
    use tedge_api::PluginDirectory;
    use tedge_api::PluginError;

    pub struct SendPluginBuilder;

    #[derive(Debug, serde::Deserialize)]
    pub struct SendConfig {
        targets: tedge_lib::config::OneOrMany<tedge_lib::config::Address>,
    }

    #[derive(Debug, miette::Diagnostic, thiserror::Error)]
    pub enum Error {
        #[error("Failed to parse configuration")]
        ConfigParseFailed(#[from] toml::de::Error),

        #[error("Sending failed")]
        SendFailed,
    }

    tedge_api::make_receiver_bundle!(struct UsizeReceiver(crate::msg::Usize));

    #[async_trait::async_trait]
    impl<PD: PluginDirectory> PluginBuilder<PD> for SendPluginBuilder {
        fn kind_name() -> &'static str {
            "send"
        }

        async fn verify_configuration(
            &self,
            config: &PluginConfiguration,
        ) -> Result<(), tedge_api::error::PluginError> {
            config
                .clone()
                .try_into()
                .map(|_: SendConfig| ())
                .map_err(Error::from)
                .map_err(PluginError::from)
        }

        async fn instantiate(
            &self,
            config: PluginConfiguration,
            _cancellation_token: tedge_api::CancellationToken,
            plugin_dir: &PD,
        ) -> Result<tedge_api::plugin::BuiltPlugin, PluginError> {
            let config: SendConfig = config.try_into().map_err(Error::from)?;

            let addrs = tedge_lib::address::AddressGroup::build(plugin_dir, &config.targets)?;
            let core_addr = plugin_dir.get_address_for_core();
            Ok(SendPlugin { addrs, core_addr }.finish())
        }

        fn kind_message_types() -> tedge_api::plugin::HandleTypes
        where
            Self: Sized,
        {
            SendPlugin::get_handled_types()
        }
    }

    struct SendPlugin {
        addrs: tedge_lib::address::AddressGroup<UsizeReceiver>,
        core_addr: tedge_api::Address<tedge_api::message::CoreMessages>,
    }

    impl tedge_api::plugin::PluginDeclaration for SendPlugin {
        type HandledMessages = ();
    }

    #[async_trait]
    impl Plugin for SendPlugin {
        async fn start(&mut self) -> Result<(), PluginError> {
            self.addrs
                .send_and_wait(crate::msg::Usize(1))
                .collect::<futures::stream::FuturesUnordered<_>>()
                .collect::<Vec<Result<_, _>>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .map(|_| ())
                .map_err(|_| Error::SendFailed)?;

            self.core_addr
                .send_and_wait(StopCore)
                .await
                .map_err(|_| Error::SendFailed)?;

            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), PluginError> {
            Ok(())
        }
    }
}

mod recv {
    use async_trait::async_trait;
    use tedge_api::plugin::PluginExt;
    use tedge_api::Plugin;
    use tedge_api::PluginBuilder;
    use tedge_api::PluginConfiguration;
    use tedge_api::PluginDirectory;
    use tedge_api::PluginError;

    pub struct RecvPluginBuilder;

    #[async_trait::async_trait]
    impl<PD: PluginDirectory> PluginBuilder<PD> for RecvPluginBuilder {
        fn kind_name() -> &'static str {
            "recv"
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
            _plugin_dir: &PD,
        ) -> Result<tedge_api::plugin::BuiltPlugin, PluginError> {
            Ok(RecvPlugin(std::sync::atomic::AtomicBool::new(false)).finish())
        }

        fn kind_message_types() -> tedge_api::plugin::HandleTypes
        where
            Self: Sized,
        {
            RecvPlugin::get_handled_types()
        }
    }

    struct RecvPlugin(std::sync::atomic::AtomicBool);

    impl tedge_api::plugin::PluginDeclaration for RecvPlugin {
        type HandledMessages = (crate::msg::Usize,);
    }

    #[derive(Debug, miette::Diagnostic, thiserror::Error)]
    enum Error {
        #[error("Failed")]
        Failed,
    }

    #[async_trait]
    impl Plugin for RecvPlugin {
        async fn start(&mut self) -> Result<(), PluginError> {
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), PluginError> {
            if !self.0.load(std::sync::atomic::Ordering::Relaxed) {
                Err(PluginError::from(Error::Failed))
            } else {
                Ok(())
            }
        }
    }

    #[async_trait]
    impl tedge_api::plugin::Handle<crate::msg::Usize> for RecvPlugin {
        async fn handle_message(
            &self,
            message: crate::msg::Usize,
            _sender: tedge_api::address::ReplySenderFor<crate::msg::Usize>,
        ) -> Result<(), PluginError> {
            assert_eq!(message.0, 1);
            self.0.store(true, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
    }
}

#[test]
fn test_send_group() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
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
            .with_plugin_builder(crate::send::SendPluginBuilder {})?
            .with_plugin_builder(crate::recv::RecvPluginBuilder {})?
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

        loop {
            tracing::info!("Looping");
            tokio::select! {
                _test_abort = &mut test_abort => {
                    tracing::error!("Test aborted");
                    run_fut.abort();
                    miette::bail!("Timeout reached, shutdown did not happen")
                },

                res = &mut run_fut => {
                    tracing::info!("application.run() returned");
                    if res.is_err() {
                        panic!("result = {:?}", res);
                    }
                    break;
                },

                _ = &mut cancel_fut => {
                    panic!("Cancellation happened...");
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
