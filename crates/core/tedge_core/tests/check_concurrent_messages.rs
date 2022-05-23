use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use miette::IntoDiagnostic;
use tedge_api::make_receiver_bundle;
use tedge_api::plugin::Handle;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginDeclaration;
use tedge_api::Address;
use tedge_api::CoreMessages;
use tedge_api::Message;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_api::PluginExt;
use tedge_core::TedgeApplication;
use tracing::debug;

const MESSAGE_COUNT: usize = 500;

#[derive(Debug)]
struct Spam;

impl Message for Spam {}

pub struct SpammyPluginBuilder;

#[async_trait::async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for SpammyPluginBuilder {
    fn kind_name() -> &'static str {
        "spammer"
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
        Ok(SpammyPlugin {
            core_target: plugin_dir.get_address_for_core(),
            target: plugin_dir.get_address_for("spammed")?,
        }
        .finish())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        SpammyPlugin::get_handled_types()
    }
}

struct SpammyPlugin {
    target: Address<SpamReceiver>,
    core_target: Address<CoreMessages>,
}

make_receiver_bundle!(struct SpamReceiver(Spam));

impl tedge_api::plugin::PluginDeclaration for SpammyPlugin {
    type HandledMessages = ();
}

#[async_trait]
impl Plugin for SpammyPlugin {
    #[allow(unreachable_code)]
    async fn main(&self) -> Result<(), PluginError> {
        for _ in 0..MESSAGE_COUNT {
            debug!("Sending SPAM");
            self.target
                .send_and_wait(Spam)
                .await
                .expect("Could not send message");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        self.core_target
            .send_and_wait(tedge_api::message::StopCore)
            .await
            .expect("Sending to core");

        Ok(())
    }
}

struct SpammedPluginBuilder {
    sender: tokio::sync::mpsc::Sender<()>,
}

#[async_trait::async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for SpammedPluginBuilder {
    fn kind_name() -> &'static str {
        "spammed"
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
        Ok(SpammedPlugin {
            sender: self.sender.clone(),
        }
        .finish())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        SpammedPlugin::get_handled_types()
    }
}

struct SpammedPlugin {
    sender: tokio::sync::mpsc::Sender<()>,
}

impl PluginDeclaration for SpammedPlugin {
    type HandledMessages = (Spam,);
}

#[async_trait::async_trait]
impl Plugin for SpammedPlugin {
    async fn start(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl Handle<Spam> for SpammedPlugin {
    async fn handle_message(
        &self,
        _message: Spam,
        _sender: tedge_api::address::ReplySenderFor<Spam>,
    ) -> Result<(), PluginError> {
        tokio::time::sleep(Duration::from_millis(10)).await;
        self.sender.send(()).await.unwrap();
        Ok(())
    }
}

#[tokio::test]
async fn test_verify_concurrent_messages() -> miette::Result<()> {
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

    let (sender, mut recv) = tokio::sync::mpsc::channel(200);

    let (_cancel_sender, application) = TedgeApplication::builder()
        .with_plugin_builder(SpammyPluginBuilder {})?
        .with_plugin_builder(SpammedPluginBuilder { sender })?
        .with_config_from_path(config_file_path)
        .await?;

    let app_loop = tokio::spawn(application.run());
    let messages = tokio::spawn(futures::stream::poll_fn(move |ctx| recv.poll_recv(ctx)).count());

    let (app_err, messages) = tokio::time::timeout(Duration::from_millis(300), async move {
        tokio::join!(app_loop, messages)
    })
    .await
    .into_diagnostic()?;

    app_err.unwrap().unwrap();

    assert_eq!(MESSAGE_COUNT, messages.unwrap());

    Ok(())
}
