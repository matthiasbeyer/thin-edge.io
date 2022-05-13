use crate::topic::CollectdTopic;
use crate::payload::CollectdPayload;

use tedge_lib::measurement::Measurement;
use tedge_lib::measurement::MeasurementValue;

pub struct CollectdMeasurement<'a> {
    topic: CollectdTopic<'a>,
    payload: CollectdPayload,
}

impl<'a> CollectdMeasurement<'a> {
    pub fn new(topic: CollectdTopic<'a>, payload: CollectdPayload) -> Self {
        Self { topic, payload }
    }

    pub fn into_measurements(&self) -> impl Iterator<Item = tedge_lib::measurement::Measurement> + '_ {
        let name = format!("{}/{}", self.topic.metric_group_key, self.topic.metric_key);

        self.payload
            .iter_metric_values()
            .map(MeasurementValue::Float)
            .map(move |v| Measurement::new(name.clone(), v))
    }
}
