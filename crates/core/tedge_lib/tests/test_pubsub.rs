use futures::FutureExt;

use tedge_core::TedgeApplication;

#[derive(Clone, Debug, bevy_reflect::TypeUuid)]
#[uuid = "38c02e81-0a0b-4eca-891b-9bb2e844ef40"]
pub struct Msg;

impl tedge_api::Message for Msg {}

mod publisher {
    extern crate tedge_lib;

    use super::Msg;

    use async_trait::async_trait;
    use tedge_api::plugin::PluginExt;
    use tedge_api::Plugin;
    use tedge_api::PluginBuilder;
    use tedge_api::PluginConfiguration;
    use tedge_api::PluginDirectory;
    use tedge_api::PluginError;
    use tracing::trace;

    pub struct PubBuilder;

    #[async_trait::async_trait]
    impl<PD: PluginDirectory> PluginBuilder<PD> for PubBuilder {
        fn kind_name() -> &'static str {
            "pub"
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
            Ok(Pub {
                sender: tokio::sync::broadcast::channel(10).0,
            }
            .finish())
        }

        fn kind_message_types() -> tedge_api::plugin::HandleTypes
        where
            Self: Sized,
        {
            Pub::get_handled_types()
        }
    }

    #[derive(Debug)]
    pub struct Pub {
        sender: tokio::sync::broadcast::Sender<Msg>,
    }

    tedge_api::make_receiver_bundle!(struct SubReq(tedge_lib::pubsub::SubscribeRequest<Msg>));

    impl tedge_api::plugin::PluginDeclaration for Pub {
        type HandledMessages = (tedge_lib::pubsub::SubscribeRequest<Msg>,);
    }

    #[async_trait]
    impl Plugin for Pub {
        async fn main(&self) -> Result<(), PluginError> {
            loop {
                match self.sender.send(Msg) {
                    Ok(_) => {
                        trace!("Send message");
                        break;
                    }
                    Err(e) => {
                        trace!("Error while trying to send message: {:?}", e);
                    }
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            Ok(())
        }
    }

    #[async_trait]
    impl tedge_api::plugin::Handle<tedge_lib::pubsub::SubscribeRequest<Msg>> for Pub {
        async fn handle_message(
            &self,
            _message: tedge_lib::pubsub::SubscribeRequest<Msg>,
            sender: tedge_api::address::ReplySenderFor<tedge_lib::pubsub::SubscribeRequest<Msg>>,
        ) -> Result<(), PluginError> {
            let _ = sender
                .reply(tedge_lib::pubsub::SubscribeReply::new_from_sender(
                    &self.sender,
                ))
                .expect("Failed to reply");
            Ok(())
        }
    }
}

mod subscriber {
    use super::Msg;

    use async_trait::async_trait;
    use tedge_api::plugin::PluginExt;
    use tedge_api::Plugin;
    use tedge_api::PluginBuilder;
    use tedge_api::PluginConfiguration;
    use tedge_api::PluginDirectory;
    use tedge_api::PluginError;
    use tracing::trace;

    pub struct SubPluginBuilder;

    #[derive(Debug, serde::Deserialize)]
    pub struct SubConfig {
        target: tedge_lib::config::Address,
    }

    #[derive(Debug, miette::Diagnostic, thiserror::Error)]
    pub enum Error {
        #[error("Failed to parse configuration")]
        ConfigParseFailed(#[from] toml::de::Error),
    }

    tedge_api::make_receiver_bundle!(pub struct SubscribeRequestBundle(tedge_lib::pubsub::SubscribeRequest<Msg>));

    #[async_trait::async_trait]
    impl<PD: PluginDirectory> PluginBuilder<PD> for SubPluginBuilder {
        fn kind_name() -> &'static str {
            "sub"
        }

        async fn verify_configuration(
            &self,
            config: &PluginConfiguration,
        ) -> Result<(), tedge_api::error::PluginError> {
            config
                .clone()
                .try_into()
                .map(|_: SubConfig| ())
                .map_err(Error::from)
                .map_err(PluginError::from)
        }

        async fn instantiate(
            &self,
            config: PluginConfiguration,
            _cancellation_token: tedge_api::CancellationToken,
            plugin_dir: &PD,
        ) -> Result<tedge_api::plugin::BuiltPlugin, PluginError> {
            let config: SubConfig = config
                .clone()
                .try_into()
                .map_err(Error::from)
                .map_err(PluginError::from)?;
            let publisher_addr =
                plugin_dir.get_address_for::<SubscribeRequestBundle>(config.target.as_ref())?;
            Ok(Sub { publisher_addr }.finish())
        }

        fn kind_message_types() -> tedge_api::plugin::HandleTypes
        where
            Self: Sized,
        {
            Sub::get_handled_types()
        }
    }

    struct Sub {
        publisher_addr: tedge_api::Address<SubscribeRequestBundle>,
    }

    impl tedge_api::plugin::PluginDeclaration for Sub {
        type HandledMessages = ();
    }

    #[async_trait]
    impl Plugin for Sub {
        async fn main(&self) -> Result<(), PluginError> {
            let mut receiver = self
                .publisher_addr
                .send_and_wait(tedge_lib::pubsub::SubscribeRequest::new())
                .await
                .expect("Sending subscribe request failed")
                .wait_for_reply(std::time::Duration::from_millis(100))
                .await
                .expect("No reply for my subscriberequest received")
                .into_inner()
                .expect("Subscribing failed");

            loop {
                match receiver.recv().await {
                    Ok(Msg) => {
                        trace!("Received message");
                    }

                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        trace!("Channel closed");
                        break;
                    }

                    Err(tokio::sync::broadcast::error::RecvError::Lagged(u)) => {
                        trace!("Lagged {}", u);
                    }
                }
            }
            Ok(())
        }
    }
}

#[test]
fn test_pubsub() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
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
            .with_plugin_builder(crate::publisher::PubBuilder {})
            .with_plugin_builder(crate::subscriber::SubPluginBuilder {})
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
