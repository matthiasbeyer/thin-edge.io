use futures::future::FutureExt;
use tedge_core::errors::TedgeApplicationError;
use tedge_core::TedgeApplication;

mod not_supported {
    use async_trait::async_trait;
    use tedge_api::plugin::PluginExt;
    use tedge_api::Plugin;
    use tedge_api::PluginBuilder;
    use tedge_api::PluginConfiguration;
    use tedge_api::PluginDirectory;
    use tedge_api::PluginError;

    pub struct NotSupportedPluginBuilder;

    #[async_trait::async_trait]
    impl<PD: PluginDirectory> PluginBuilder<PD> for NotSupportedPluginBuilder {
        fn kind_name() -> &'static str {
            "notsupported"
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
            Ok(NotSupportedPlugin {}.finish())
        }

        fn kind_message_types() -> tedge_api::plugin::HandleTypes
        where
            Self: Sized,
        {
            NotSupportedPlugin::get_handled_types()
        }
    }

    struct NotSupportedPlugin;

    impl tedge_api::plugin::PluginDeclaration for NotSupportedPlugin {
        type HandledMessages = ();
    }

    #[async_trait]
    impl Plugin for NotSupportedPlugin {
        async fn start(&mut self) -> Result<(), PluginError> {
            tracing::info!("Setup called");
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), PluginError> {
            tracing::info!("Shutdown called");
            Ok(())
        }
    }
}

mod sending {
    use async_trait::async_trait;
    use tedge_api::plugin::PluginExt;
    use tedge_api::Plugin;
    use tedge_api::PluginBuilder;
    use tedge_api::PluginConfiguration;
    use tedge_api::PluginDirectory;
    use tedge_api::PluginError;

    pub struct SendingPluginBuilder;

    #[async_trait::async_trait]
    impl<PD: PluginDirectory> PluginBuilder<PD> for SendingPluginBuilder {
        fn kind_name() -> &'static str {
            "sending"
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
            tracing::warn!("Going to fetch addresses that do not support the messages I expect");
            // this should not work
            let _target_addr = plugin_dir.get_address_for::<SendingMessages>("not_supported")?;
            Ok(SendingPlugin {}.finish())
        }

        fn kind_message_types() -> tedge_api::plugin::HandleTypes
        where
            Self: Sized,
        {
            SendingPlugin::get_handled_types()
        }
    }

    struct SendingPlugin;

    impl tedge_api::plugin::PluginDeclaration for SendingPlugin {
        type HandledMessages = ();
    }

    #[async_trait]
    impl Plugin for SendingPlugin {
        async fn start(&mut self) -> Result<(), PluginError> {
            tracing::info!("Setup called");
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), PluginError> {
            tracing::info!("Shutdown called");
            Ok(())
        }
    }

    #[derive(Debug, bevy_reflect::TypeUuid)]
    #[uuid = "2cafb4ce-f1b0-4562-9071-0091cafb95b8"]
    pub struct SendingMessage;
    impl tedge_api::message::Message for SendingMessage {}

    tedge_api::make_receiver_bundle!(pub struct SendingMessages(SendingMessage));
}

#[test_log::test(tokio::test)]
async fn test_not_supported_message() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    let _ = tracing_subscriber::fmt::try_init();

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
        .with_plugin_builder(crate::not_supported::NotSupportedPluginBuilder {})
        .with_plugin_builder(crate::sending::SendingPluginBuilder {})
        .with_config_from_path(config_file_path)
        .await?;

    let run_fut = application.run();

    // send a cancel request to the app after 1 sec
    let cancel_fut = Box::pin({
        tokio::time::sleep(std::time::Duration::from_secs(1)).then(|_| async {
            tracing::info!("Cancelling app now");
            cancel_sender.cancel_app()
        })
    });

    tokio::select! {
        app_res = run_fut => {
            tracing::info!("application.run() returned");
            match app_res {
                Ok(_) => panic!("Application exited successfully. It should return an error though"),
                Err(TedgeApplicationError::PluginInstantiationsError { .. }) => {
                    // TODO Check whether correct error kind is returned
                    Ok(())
                }

                Err(other) => {
                    panic!("Expected PluginDoesNotSupport error, found: {:?}", other);
                }

            }
        },

        _ = cancel_fut => {
            tracing::info!("Cancellation happened...");
            panic!("App should have exited on its own, but cancellation was necessary");
        }
    }
}
