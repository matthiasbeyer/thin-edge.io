use std::collections::HashMap;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

impl tedge_api::plugin::Message for Measurement {
}

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
