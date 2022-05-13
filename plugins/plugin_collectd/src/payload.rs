use crate::error::Error;

#[derive(Debug)]
pub struct CollectdPayload {
    #[allow(unused)]
    timestamp: f64,
    metric_values: Vec<f64>,
}

impl CollectdPayload {
    pub fn parse(payload: &str) -> Result<Self, Error> {
        let msg: Vec<&str> = payload.split(':').collect();
        let vec_len = msg.len();

        if vec_len <= 1 {
            return Err(Error::InvalidMeasurementPayloadFormat(
                payload.to_string(),
            ));
        }

        // First element is always the timestamp
        let timestamp = msg[0].parse::<f64>().map_err(|_err| {
            Error::InvalidMeasurementTimestamp(msg[0].to_string())
        })?;

        let mut metric_values: Vec<f64> = Vec::with_capacity(vec_len - 1);

        // Process the values
        for i in 1..vec_len {
            let value = msg[i].parse::<f64>().map_err(|_err| {
                Error::InvalidMeasurementValue(msg[i].to_string())
            })?;

            metric_values.push(value);
        }

        Ok(CollectdPayload {
            timestamp,
            metric_values,
        })
    }

    pub fn iter_metric_values(&self) -> impl Iterator<Item = f64> + '_ {
        self.metric_values.iter().map(|v| *v)
    }

    #[allow(unused)]
    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }
}

