use std::sync::Arc;

use async_trait::async_trait;
use tedge_api::plugin::PluginExt;
use tokio_util::sync::CancellationToken;

use tedge_api::address::ReplySenderFor;
use tedge_api::plugin::Handle;
use tedge_api::Address;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_lib::mainloop::MainloopStopper;
use tedge_lib::measurement::Measurement;
use tedge_lib::measurement::MeasurementValue;

use tokio::sync::RwLock;
use tracing::Instrument;

pub struct AvgPluginBuilder;

#[derive(serde::Deserialize, Debug, tedge_api::Config)]
struct AvgConfig {
    /// The duration of the time window to calculate the average for
    timeframe: tedge_lib::config::Humantime,

    /// The name of the plugin to send the result to
    target: tedge_lib::config::Address,

    /// Whether to report a 0 (zero) if there are zero measurements in the timeframe
    report_on_zero_elements: bool,
}

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
enum Error {
    #[error("Failed to parse configuration")]
    ConfigParseFailed(#[from] toml::de::Error),

    #[error("Failed to stop main loop")]
    FailedToStopMainloop,
}

#[async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for AvgPluginBuilder {
    fn kind_name() -> &'static str {
        "avg"
    }

    fn kind_configuration() -> Option<tedge_api::ConfigDescription> {
        Some(<AvgConfig as tedge_api::AsConfig>::as_config())
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into()
            .map(|_: AvgConfig| ())
            .map_err(Error::from)
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<tedge_api::plugin::BuiltPlugin, PluginError> {
        let config = config.try_into::<AvgConfig>().map_err(Error::from)?;

        let address = config.target.build(plugin_dir)?;
        Ok(AvgPlugin::new(address, config).finish())
    }

    fn kind_message_types() -> tedge_api::plugin::HandleTypes
    where
        Self: Sized,
    {
        AvgPlugin::get_handled_types()
    }
}

tedge_api::make_receiver_bundle!(struct MeasurementReceiver(Measurement));

#[derive(Debug)]
struct AvgPlugin {
    addr: Address<MeasurementReceiver>,
    config: AvgConfig,
    values: Arc<RwLock<Vec<f64>>>,
    stopper: Option<MainloopStopper>,
}

impl tedge_api::plugin::PluginDeclaration for AvgPlugin {
    type HandledMessages = (Measurement,);
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

#[derive(Debug)]
struct State {
    target: Address<MeasurementReceiver>,
    report_zero: bool,
    values: Arc<RwLock<Vec<f64>>>,
}

#[async_trait]
impl Plugin for AvgPlugin {
    #[tracing::instrument(name = "plugin.avg.start")]
    async fn start(&mut self) -> Result<(), PluginError> {
        let state = State {
            target: self.addr.clone(),
            report_zero: self.config.report_on_zero_elements,
            values: self.values.clone(),
        };
        let (stopper, mainloop) = tedge_lib::mainloop::Mainloop::ticking_every(
            self.config.timeframe.into_duration(),
            state,
        );
        self.stopper = Some(stopper);

        let _ = tokio::spawn(
            mainloop
                .run(main_avg)
                .instrument(tracing::debug_span!("plugin.avg.mainloop")),
        );
        Ok(())
    }

    #[tracing::instrument(name = "plugin.avg.shutdown")]
    async fn shutdown(&mut self) -> Result<(), PluginError> {
        if let Some(stopper) = self.stopper.take() {
            stopper.stop().map_err(|()| Error::FailedToStopMainloop)?;
        }
        Ok(())
    }
}

#[async_trait]
impl Handle<Measurement> for AvgPlugin {
    #[tracing::instrument(name = "plugin.avg.handle_message")]
    async fn handle_message(
        &self,
        message: Measurement,
        _reply: ReplySenderFor<Measurement>,
    ) -> Result<(), PluginError> {
        let value = match message.value() {
            MeasurementValue::Float(f) => Some(f),
            other => {
                tracing::error!(
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

#[tracing::instrument(name = "plugin.avg.main", skip(state))]
async fn main_avg(state: Arc<State>) -> Result<(), PluginError> {
    let mut values = state
        .values
        .write()
        .instrument(tracing::trace_span!("plugin.avg.main.lock"))
        .await;

    let count = values.len() as u64; // TODO: Here be dragons
    if count > 0 || state.report_zero {
        let sum = values.drain(0..).sum::<f64>();
        let avg = sum / (count as f64);
        tracing::trace!(avg, "Calculated average");

        let value = MeasurementValue::Float(avg);
        let measurement = Measurement::new("avg".to_string(), value);

        let _ = state
            .target
            .send_and_wait(measurement)
            .instrument(tracing::trace_span!("plugin.avg.main.send_and_wait"))
            .await;
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
