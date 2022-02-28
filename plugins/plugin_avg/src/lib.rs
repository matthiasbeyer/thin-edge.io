use std::sync::Arc;

use async_trait::async_trait;

use tedge_api::address::EndpointKind;
use tedge_api::message::MeasurementValue;
use tedge_api::Address;
use tedge_api::CoreCommunication;
use tedge_api::Message;
use tedge_api::MessageKind;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginError;
use tedge_lib::mainloop::MainloopStopper;

use tokio::sync::RwLock;

pub struct AvgPluginBuilder;

#[derive(serde::Deserialize, Debug)]
struct AvgConfig {
    #[serde(with = "humantime_serde")]
    timeframe: std::time::Duration,

    target: String,

    /// Whether to report a 0 (zero) if there are zero measurements in the timeframe
    report_on_zero_elements: bool,

    /// If set to `true`, report [1, 2].avg() as 1.5
    /// If set to false, integers are reported (and possibly inaccurate)
    int_to_float_avg: bool,
}

#[async_trait]
impl PluginBuilder for AvgPluginBuilder {
    fn kind_name(&self) -> &'static str {
        "avg"
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .get_ref()
            .clone()
            .try_into()
            .map(|_: AvgConfig| ())
            .map_err(|_| anyhow::anyhow!("Failed to parse log configuration"))
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        comms: tedge_api::plugin::CoreCommunication,
    ) -> Result<Box<dyn Plugin>, PluginError> {
        let config = config
            .into_inner()
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to parse log configuration"))?;

        Ok(Box::new(AvgPlugin::new(comms, config)))
    }
}

struct AvgPlugin {
    comms: tedge_api::plugin::CoreCommunication,
    config: AvgConfig,
    values: Arc<RwLock<Vec<u64>>>,
    stopper: Option<MainloopStopper>,
}

impl AvgPlugin {
    fn new(comms: tedge_api::plugin::CoreCommunication, config: AvgConfig) -> Self {
        Self {
            comms,
            config,
            values: Arc::new(RwLock::new(Vec::new())),
            stopper: None,
        }
    }
}

struct State {
    target: Address,
    report_zero: bool,
    int_to_float_avg: bool,
    comms: CoreCommunication,
    values: Arc<RwLock<Vec<u64>>>,
}

#[async_trait]
impl Plugin for AvgPlugin {
    async fn setup(&mut self) -> Result<(), PluginError> {
        let state = State {
            target: Address::new(EndpointKind::Plugin {
                id: self.config.target.clone(),
            }),
            report_zero: self.config.report_on_zero_elements,
            int_to_float_avg: self.config.int_to_float_avg,
            comms: self.comms.clone(),
            values: self.values.clone(),
        };
        let (stopper, mainloop) =
            tedge_lib::mainloop::Mainloop::ticking_every(self.config.timeframe, state);
        self.stopper = Some(stopper);

        let _ = tokio::spawn(mainloop.run(main_avg));
        Ok(())
    }

    async fn handle_message(&self, message: Message) -> Result<(), PluginError> {
        let value = match message.kind() {
            MessageKind::Measurement { name, value } => match value {
                MeasurementValue::Int(i) => Some(i),
                other => {
                    log::error!(
                        "Received measurement that I cannot handle: {} = {}",
                        name,
                        measurement_to_str(other)
                    );
                    None
                }
            },
            _ => {
                log::error!("Received message kind that I cannot handle");
                None
            }
        };

        if let Some(value) = value {
            self.values.write().await.push(*value);
        }

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        if let Some(stopper) = self.stopper.take() {
            stopper
                .stop()
                .map_err(|_| anyhow::anyhow!("Failed to stop mainloop"))?
        }
        Ok(())
    }
}

async fn main_avg(state: Arc<State>) -> Result<(), PluginError> {
    let mut values = state.values.write().await;

    let count = values.len() as u64; // TODO: Here be dragons
    if count > 0 || state.report_zero {
        let sum = values.drain(0..).sum::<u64>();
        let value = MeasurementValue::Int(sum / count);
        let measurement = MessageKind::Measurement {
            name: "avg".to_string(),
            value,
        };
        let _ = state.comms.send(measurement, state.target.clone()).await?;
    }

    Ok(())
}

fn measurement_to_str(val: &MeasurementValue) -> &'static str {
    match val {
        MeasurementValue::Bool(_) => "Bool",
        MeasurementValue::Int(_) => "Int",
        MeasurementValue::Float(_) => "Float",
        MeasurementValue::Str(_) => "Str",
        MeasurementValue::Aggregate(_) => "Aggregate",
        _ => "Unknown",
    }
}
