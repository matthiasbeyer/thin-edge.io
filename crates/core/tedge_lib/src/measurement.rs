use std::collections::HashMap;
use type_uuid::TypeUuid;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TypeUuid)]
#[uuid = "daad7462-0298-40e0-97b5-1a7b0c2da297"]
pub struct Measurement {
    name: String,
    value: MeasurementValue,
}

impl Measurement {
    pub const fn new(name: String, value: MeasurementValue) -> Self {
        Self { name, value }
    }

    /// Get a reference to the measurement's name.
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Get a reference to the measurement's value.
    pub fn value(&self) -> &MeasurementValue {
        &self.value
    }
}

impl tedge_api::Message for Measurement {}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
#[serde(untagged)]
pub enum MeasurementValue {
    Bool(bool),
    Float(f64),
    Text(String),
    List(Vec<MeasurementValue>),
    Map(HashMap<String, MeasurementValue>),
}
