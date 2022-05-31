//! Messages outgoing from this plugin

use tedge_api::PluginError;

// A message that was received over MQTT by this plugin and is to be send to another plugin
#[derive(Debug, type_uuid::TypeUuid)]
#[uuid = "61d5df16-7f0c-4d3d-8a90-e8a7cd6f5545"]
pub struct IncomingMessage {
    pub(crate) payload: Vec<u8>,
    pub(crate) qos: i32,
    pub(crate) retain: bool,
    pub(crate) topic: String,
}

impl IncomingMessage {
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }

    pub fn qos(&self) -> Result<crate::config::QoS, PluginError> {
        crate::config::QoS::try_from(self.qos)
    }

    pub fn retain(&self) -> bool {
        self.retain
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }
}

impl tedge_api::Message for IncomingMessage {}

tedge_api::make_receiver_bundle!(pub struct MqttMessageReceiver(IncomingMessage));

#[derive(Debug, type_uuid::TypeUuid)]
#[uuid = "b71dd332-1e67-4be1-8824-a4b6cf547d5f"]
pub struct OutgoingMessage {
    pub(crate) payload: Vec<u8>,
    pub(crate) topic: String,
    pub(crate) qos: crate::config::QoS,
}

impl OutgoingMessage {
    pub fn new(payload: Vec<u8>, topic: String) -> Self {
        OutgoingMessage {
            payload,
            topic,
            qos: crate::config::QoS::AtLeastOnce,
        }
    }

    pub fn with_qos(mut self, qos: crate::config::QoS) -> Self {
        self.qos = qos;
        self
    }
}

impl tedge_api::Message for OutgoingMessage {}
