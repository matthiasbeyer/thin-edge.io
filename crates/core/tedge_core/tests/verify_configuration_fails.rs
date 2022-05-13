use async_trait::async_trait;
use tedge_api::plugin::HandleTypes;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_api::PluginExt;
use tedge_core::errors::TedgeApplicationError;
use tedge_core::TedgeApplication;

pub struct VerifyConfigFailsPluginBuilder;

#[async_trait::async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for VerifyConfigFailsPluginBuilder {
    fn kind_name() -> &'static str {
        "verify_config_fails"
    }

    async fn verify_configuration(
        &self,
        _config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        Err(miette::miette!("Verification of config failed"))
    }

    async fn instantiate(
        &self,
        _config: PluginConfiguration,
        _cancellation_token: tedge_api::CancellationToken,
        _plugin_dir: &PD,
    ) -> Result<tedge_api::plugin::BuiltPlugin, PluginError> {
        unreachable!()
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        VerifyConfigFailsPlugin::get_handled_types()
    }
}

struct VerifyConfigFailsPlugin;

impl tedge_api::plugin::PluginDeclaration for VerifyConfigFailsPlugin {
    type HandledMessages = ();
}

#[async_trait]
impl Plugin for VerifyConfigFailsPlugin {
    #[allow(unreachable_code)]
    async fn start(&mut self) -> Result<(), PluginError> {
        unreachable!()
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        unreachable!()
    }
}

#[tokio::test]
async fn test_verify_fails_plugin() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
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

    let (_cancel_sender, application) = TedgeApplication::builder()
        .with_plugin_builder(VerifyConfigFailsPluginBuilder {})?
        .with_config_from_path(config_file_path)
        .await?;

    match application.run().await {
        Err(TedgeApplicationError::PluginConfigVerificationFailed(e)) => {
            tracing::info!("Application errored successfully: {:?}", e);
            Ok(())
        }
        Err(_) => {
            panic!("Application should have errored with PluginConfigVerificationFailed because plugin failed to verify configuration")
        }

        _ok => {
            panic!("Application should have errored because plugin failed to verify configuration")
        }
    }
}
