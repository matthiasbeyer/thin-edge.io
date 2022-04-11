use std::sync::Arc;

use async_trait::async_trait;
use tedge_api::plugin::PluginExt;
use tokio_util::sync::CancellationToken;

use tedge_lib::measurement::Measurement;
use tedge_lib::measurement::MeasurementValue;
use tedge_api::Address;
use tedge_api::plugin::Handle;
use tedge_api::PluginDirectory;
use tedge_api::address::ReplySender;
use tedge_api::message::NoReply;
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
impl<PD: PluginDirectory> PluginBuilder<PD> for AvgPluginBuilder {
    fn kind_name() -> &'static str {
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
        _cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<tedge_api::plugin::BuiltPlugin, PluginError> {
        let config = config
            .into_inner()
            .try_into::<AvgConfig>()
            .map_err(|_| anyhow::anyhow!("Failed to parse log configuration"))?;

        let address = plugin_dir.get_address_for::<MeasurementReceiver>(&config.target)?;
        Ok(AvgPlugin::new(address, config).into_untyped::<(Measurement,)>())
    }

    fn kind_message_types() -> tedge_api::plugin::HandleTypes
        where Self:Sized
    {
        tedge_api::plugin::HandleTypes::declare_handlers_for::<(Measurement,), AvgPlugin>()
    }

}

tedge_api::make_receiver_bundle!(struct MeasurementReceiver(Measurement));

struct AvgPlugin {
    addr: Address<MeasurementReceiver>,
    config: AvgConfig,
    values: Arc<RwLock<Vec<f64>>>,
    stopper: Option<MainloopStopper>,
}

impl AvgPlugin {
    fn new(addr: Address<MeasurementReceiver>, config: AvgConfig) -> Self {
        Self {
            addr,
            config,
            values: Arc::new(RwLock::new(Vec::new())),
            stopper: None,
        }
    }
}

struct State {
    target: Address<MeasurementReceiver>,
    report_zero: bool,
    int_to_float_avg: bool,
    values: Arc<RwLock<Vec<f64>>>,
}

#[async_trait]
impl Plugin for AvgPlugin {
    async fn setup(&mut self) -> Result<(), PluginError> {
        let state = State {
            target: self.addr.clone(),
            report_zero: self.config.report_on_zero_elements,
            int_to_float_avg: self.config.int_to_float_avg,
            values: self.values.clone(),
        };
        let (stopper, mainloop) =
            tedge_lib::mainloop::Mainloop::ticking_every(self.config.timeframe, state);
        self.stopper = Some(stopper);

        let _ = tokio::spawn(mainloop.run(main_avg));
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

#[async_trait]
impl Handle<Measurement> for AvgPlugin {
    async fn handle_message(&self, message: Measurement, _reply: ReplySender<NoReply>) -> Result<(), PluginError> {
        let value = match message.value() {
            MeasurementValue::Float(f) => Some(f),
            other => {
                log::error!(
                    "Received measurement that I cannot handle: {} = {}",
                    message.name(),
                    measurement_to_str(other)
                );
                None
            }
        };

        if let Some(value) = value {
            self.values.write().await.push(*value);
        }

        Ok(())
    }
}

async fn main_avg(state: Arc<State>) -> Result<(), PluginError> {
    let mut values = state.values.write().await;

    let count = values.len() as u64; // TODO: Here be dragons
    if count > 0 || state.report_zero {
        let sum = values.drain(0..).sum::<f64>();
        let value = MeasurementValue::Float(sum / (count as f64));
        let measurement = Measurement::new("avg".to_string(), value);
        let _ = state.target.send(measurement).await;
    }

    Ok(())
}

fn measurement_to_str(val: &MeasurementValue) -> &'static str {
    match val {
        MeasurementValue::Bool(_) => "Bool",
        MeasurementValue::Float(_) => "Float",
        MeasurementValue::Text(_) => "Str",
        MeasurementValue::List(_) => "List",
        MeasurementValue::Map(_) => "Map",
        _ => "Unknown",
    }
}
