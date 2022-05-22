use async_trait::async_trait;

use plugin_mqtt::IncomingMessage;
use plugin_mqtt::OutgoingMessage;
use tedge_api::plugin::Handle;
use tedge_api::Address;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tedge_lib::sm::response::InstallResponse;
use tedge_lib::sm::response::ListResponse;
use tedge_lib::sm::response::UninstallResponse;
use tedge_lib::sm::response::UpdateResponse;

use crate::builder::SMReceiver;
use crate::message::SmRequest;
use crate::message::SmResponse;

tedge_api::make_receiver_bundle!(pub struct OutgoingMessageReceiver(OutgoingMessage));

#[derive(Debug)]
pub struct MqttSMPlugin {
    target: Address<SMReceiver>,
    mqtt_addr: Address<OutgoingMessageReceiver>,
    response_topic: String,
}

impl tedge_api::plugin::PluginDeclaration for MqttSMPlugin {
    type HandledMessages = (
        IncomingMessage,
        InstallResponse,
        ListResponse,
        UninstallResponse,
        UpdateResponse,
    );
}

impl MqttSMPlugin {
    pub fn new(
        target: Address<SMReceiver>,
        mqtt_addr: Address<OutgoingMessageReceiver>,
        response_topic: String,
    ) -> Self {
        Self {
            target,
            mqtt_addr,
            response_topic,
        }
    }
}

#[async_trait]
impl Plugin for MqttSMPlugin {
    async fn start(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

#[async_trait]
impl Handle<IncomingMessage> for MqttSMPlugin {
    #[tracing::instrument(name = "plugin.mqtt_sm.handle_message", skip(self, _sender))]
    async fn handle_message(
        &self,
        message: IncomingMessage,
        _sender: tedge_api::address::ReplySenderFor<IncomingMessage>,
    ) -> Result<(), PluginError> {
        let request = serde_json::from_slice::<SmRequest>(message.payload())
            .map_err(crate::error::Error::DeserError)?;

        match request {
            SmRequest::List => {
                self.target
                    .send_and_wait(tedge_lib::sm::request::List)
                    .await
                    .map_err(|_| crate::error::Error::SendFailed)?;

                Ok(())
            }
            SmRequest::Install { package_name } => {
                let reply = self
                    .target
                    .send_and_wait(tedge_lib::sm::request::Install::new(package_name))
                    .await
                    .map_err(|_| crate::error::Error::SendFailed)?
                    .wait_for_reply(std::time::Duration::from_secs(10)) // wait 10 secs for a reply
                    .await
                    .map_err(crate::error::Error::from)?;

                let buf = serde_json::to_string(&reply).map_err(crate::error::Error::SerError)?;
                let buf = buf.as_bytes().to_vec();
                let msg = plugin_mqtt::OutgoingMessage::new(buf, self.response_topic.clone());

                self.mqtt_addr
                    .send_and_wait(msg)
                    .await
                    .map_err(|_| crate::error::Error::SendFailed)?;

                Ok(())
            }
            SmRequest::Update { package_name } => {
                self.target
                    .send_and_wait(tedge_lib::sm::request::Update::new(package_name))
                    .await
                    .map_err(|_| crate::error::Error::SendFailed)?;

                Ok(())
            }
            SmRequest::Uninstall { package_name } => {
                self.target
                    .send_and_wait(tedge_lib::sm::request::Uninstall::new(package_name))
                    .await
                    .map_err(|_| crate::error::Error::SendFailed)?;

                Ok(())
            }
        }
    }
}

#[async_trait]
impl Handle<InstallResponse> for MqttSMPlugin {
    async fn handle_message(
        &self,
        message: InstallResponse,
        _sender: tedge_api::address::ReplySenderFor<InstallResponse>,
    ) -> Result<(), PluginError> {
        let response_payload = match message {
            InstallResponse::InstallProgress {
                package_name,
                progress,
            } => SmResponse::InstallingState {
                package_name,
                progress,
            },

            InstallResponse::InstallLogLine {
                package_name,
                log_line,
            } => SmResponse::InstallingLogLine {
                package_name,
                log_line,
            },

            InstallResponse::InstallSucceeded { package_name } => {
                SmResponse::InstallSucceeded { package_name }
            }

            InstallResponse::InstallFailed {
                package_name,
                failure_message,
            } => SmResponse::InstallFailed {
                package_name,
                failure_message,
            },
        };

        let message = {
            let response_payload = serde_json::to_string(&response_payload)
                .map_err(crate::error::Error::SerError)?
                .as_bytes()
                .to_vec();

            OutgoingMessage::new(response_payload, self.response_topic.clone())
        };

        self.mqtt_addr
            .send_and_wait(message)
            .await
            .map_err(|_| crate::error::Error::SendFailed)?;

        Ok(())
    }
}

#[async_trait]
impl Handle<ListResponse> for MqttSMPlugin {
    async fn handle_message(
        &self,
        message: ListResponse,
        _sender: tedge_api::address::ReplySenderFor<ListResponse>,
    ) -> Result<(), PluginError> {
        let response_payload = match message {
            ListResponse::List { list } => SmResponse::List { list },
            ListResponse::ListFailed { message } => SmResponse::ListFailed { message },
        };

        let message = {
            let response_payload = serde_json::to_string(&response_payload)
                .map_err(crate::error::Error::SerError)?
                .as_bytes()
                .to_vec();

            OutgoingMessage::new(response_payload, self.response_topic.clone())
        };

        self.mqtt_addr
            .send_and_wait(message)
            .await
            .map_err(|_| crate::error::Error::SendFailed)?;

        Ok(())
    }
}

#[async_trait]
impl Handle<UninstallResponse> for MqttSMPlugin {
    async fn handle_message(
        &self,
        message: UninstallResponse,
        _sender: tedge_api::address::ReplySenderFor<UninstallResponse>,
    ) -> Result<(), PluginError> {
        let response_payload = match message {
            UninstallResponse::UninstallProgress {
                package_name,
                progress,
            } => SmResponse::UninstallState { package_name, progress },

            UninstallResponse::UninstallLogLine {
                package_name,
                log_line,
            } => SmResponse::UninstallLogLine { package_name, log_line },

            UninstallResponse::UninstallSucceeded {
                package_name,
            } => SmResponse::UninstallSucceeded { package_name },

            UninstallResponse::UninstallFailed {
                package_name,
                failure_message,
            } => SmResponse::UninstallFailed { package_name, failure_message },
        };
        let message = {
            let response_payload = serde_json::to_string(&response_payload)
                .map_err(crate::error::Error::SerError)?
                .as_bytes()
                .to_vec();

            OutgoingMessage::new(response_payload, self.response_topic.clone())
        };

        self.mqtt_addr
            .send_and_wait(message)
            .await
            .map_err(|_| crate::error::Error::SendFailed)?;

        Ok(())
    }
}

#[async_trait]
impl Handle<UpdateResponse> for MqttSMPlugin {
    async fn handle_message(
        &self,
        message: UpdateResponse,
        _sender: tedge_api::address::ReplySenderFor<UpdateResponse>,
    ) -> Result<(), PluginError> {
        let response_payload = match message {
            UpdateResponse::UpdateProgress {
                package_name,
                progress,
            } => SmResponse::UpdatingState { package_name, progress },

            UpdateResponse::UpdateLogLine {
                package_name,
                log_line,
            } => SmResponse::UpdatingLogLine { package_name, log_line },

            UpdateResponse::UpdateSucceeded {
                package_name,
            } => SmResponse::UpdateSucceeded { package_name },

            UpdateResponse::UpdateFailed {
                package_name,
                failure_message,
            } => SmResponse::UpdateFailed { package_name, failure_message },
        };
        let message = {
            let response_payload = serde_json::to_string(&response_payload)
                .map_err(crate::error::Error::SerError)?
                .as_bytes()
                .to_vec();

            OutgoingMessage::new(response_payload, self.response_topic.clone())
        };

        self.mqtt_addr
            .send_and_wait(message)
            .await
            .map_err(|_| crate::error::Error::SendFailed)?;

        Ok(())
    }
}

