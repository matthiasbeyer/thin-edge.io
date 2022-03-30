//! Messages outgoing from this plugin

use tedge_api::PluginError;

// A message that was received over MQTT by this plugin and is to be send to another plugin
#[derive(Debug)]
pub struct IncomingMessage {
    pub(crate) dup: bool,
    pub(crate) payload: serde_json::Value,
    pub(crate) pkid: u16,
    pub(crate) qos: rumqttc::QoS,
    pub(crate) retain: bool,
    pub(crate) topic: String,
}

impl IncomingMessage {
    pub fn dup(&self) -> bool {
        self.dup
    }

    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }

    pub fn into_payload(self) -> serde_json::Value {
        self.payload
    }

    pub fn pkid(&self) -> u16 {
        self.pkid
    }

    pub fn qos(&self) -> rumqttc::QoS {
        self.qos
    }

    pub fn retain(&self) -> bool {
        self.retain
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }
}

impl tedge_api::plugin::Message for IncomingMessage {
    type Reply = tedge_api::message::NoReply; // for now
}

tedge_api::make_receiver_bundle!(pub struct MqttMessageReceiver(IncomingMessage));


#[derive(Debug)]
pub struct OutgoingMessage {
    pub(crate) payload: serde_json::Value,
    pub(crate) topic: String,
    pub(crate) qos: rumqttc::QoS,
    pub(crate) retain: bool,
}

impl OutgoingMessage {
    pub fn for_payload<T>(t: &T, topic: String) -> Result<Self, PluginError>
        where T: serde::Serialize + std::fmt::Debug
    {
        let payload = serde_json::to_value(t)
            .map_err(|e| anyhow::anyhow!("Failed to serialize '{:?}': {}", t, e))?;

        Ok({
            OutgoingMessage {
                payload,
                topic,
                qos: rumqttc::QoS::AtMostOnce,
                retain: false,
            }
        })
    }

    pub fn with_qos(mut self, qos: rumqttc::QoS) -> Self {
        self.qos = qos;
        self
    }

    pub fn with_retain(mut self, retain: bool) -> Self {
        self.retain = retain;
        self
    }
}

impl tedge_api::plugin::Message for OutgoingMessage {
    type Reply = tedge_api::message::NoReply; // for now
}


