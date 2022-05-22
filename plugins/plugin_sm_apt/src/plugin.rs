use async_trait::async_trait;

use tedge_api::plugin::Handle;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tedge_lib::sm::request::Install;
use tedge_lib::sm::request::List;
use tedge_lib::sm::request::Uninstall;
use tedge_lib::sm::request::Update;
use tedge_lib::sm::response::InstallResponse;
use tracing::trace;

#[derive(Debug)]
pub struct SmAptPlugin {
    #[allow(unused)] // TODO
    apt_binary_path: Option<std::path::PathBuf>,
}

impl tedge_api::plugin::PluginDeclaration for SmAptPlugin {
    type HandledMessages = (Install, List, Uninstall, Update);
}

impl SmAptPlugin {
    pub fn new(apt_binary_path: Option<&std::path::Path>) -> Self {
        Self {
            apt_binary_path: apt_binary_path.map(ToOwned::to_owned),
        }
    }
}

#[async_trait]
impl Plugin for SmAptPlugin {
    async fn start(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        trace!("Shutdown");
        Ok(())
    }
}

#[async_trait]
impl Handle<Install> for SmAptPlugin {
    #[tracing::instrument(name = "plugin.sm_apt.handle_message.install", skip(self, sender))]
    async fn handle_message(
        &self,
        message: Install,
        sender: tedge_api::address::ReplySenderFor<Install>,
    ) -> Result<(), PluginError> {
        tracing::info!(package_name = %message.package_name(), "MOCK: Going to install software");
        sender
            .reply(InstallResponse::InstallSucceeded {
                package_name: message.package_name().clone(),
            })
            .map_err(|_| crate::error::Error::ReplySendingFailed)?;
        Ok(())
    }
}

#[async_trait]
impl Handle<List> for SmAptPlugin {
    #[tracing::instrument(name = "plugin.sm_apt.handle_message.list", skip(self, _sender))]
    async fn handle_message(
        &self,
        _message: List,
        _sender: tedge_api::address::ReplySenderFor<List>,
    ) -> Result<(), PluginError> {
        tracing::info!("MOCK: Going to list software");
        Ok(())
    }
}

#[async_trait]
impl Handle<Uninstall> for SmAptPlugin {
    #[tracing::instrument(name = "plugin.sm_apt.handle_message.uninstall", skip(self, _sender))]
    async fn handle_message(
        &self,
        message: Uninstall,
        _sender: tedge_api::address::ReplySenderFor<Uninstall>,
    ) -> Result<(), PluginError> {
        tracing::info!(package_name = %message.package_name(), "MOCK: Going to uninstall software");
        Ok(())
    }
}

#[async_trait]
impl Handle<Update> for SmAptPlugin {
    #[tracing::instrument(name = "plugin.sm_apt.handle_message.update", skip(self, _sender))]
    async fn handle_message(
        &self,
        message: Update,
        _sender: tedge_api::address::ReplySenderFor<Update>,
    ) -> Result<(), PluginError> {
        tracing::info!(package_name = %message.package_name(), "MOCK: Going to update software");
        Ok(())
    }
}
