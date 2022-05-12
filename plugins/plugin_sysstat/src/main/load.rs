use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use futures::StreamExt;
use sysinfo::SystemExt;
use tokio::sync::Mutex;

use tedge_api::Address;
use tedge_api::PluginError;
use tedge_lib::iter::IntoSendAll;
use tedge_lib::measurement::Measurement;
use tedge_lib::measurement::MeasurementValue;
use tracing::Instrument;

use crate::config::HasBaseConfig;
use crate::main::State;
use crate::main::StateFromConfig;
use crate::plugin::MeasurementReceiver;

#[derive(Debug)]
pub struct LoadState {
    interval: u64,
    send_to: Arc<Vec<Address<MeasurementReceiver>>>,
    sys: sysinfo::System,
}

impl State for LoadState {
    fn interval(&self) -> u64 {
        self.interval
    }
}

impl StateFromConfig for LoadState {
    fn new_from_config(
        config: &crate::config::SysStatConfig,
        addrs: Arc<Vec<Address<MeasurementReceiver>>>,
    ) -> Option<Self> {
        config.load.as_ref().map(|config| LoadState {
            interval: config.interval_ms().get(),
            send_to: addrs,
            sys: sysinfo::System::new(),
        })
    }
}

#[tracing::instrument(name = "plugin.sysstat.main-load", skip(state))]
pub async fn main_load(state: Arc<Mutex<LoadState>>) -> Result<(), PluginError> {
    let mut lock = state.lock().await;
    let mut state = lock.deref_mut();
    state.sys.refresh_cpu(); // assuming that this is the required refresh call
    let state = lock.deref();
    let load = state.sys.load_average();

    let mut hm = HashMap::new();
    hm.insert(String::from("one"), MeasurementValue::Float(load.one));
    hm.insert(String::from("five"), MeasurementValue::Float(load.five));
    hm.insert(String::from("fifteen"), MeasurementValue::Float(load.fifteen));
    let value = MeasurementValue::Map(hm);
    let message = Measurement::new("load".to_string(), value.clone());

    std::iter::repeat(message)
        .zip(state.send_to.iter())
        .send_all()
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<Result<_, _>>>()
        .instrument(tracing::debug_span!("plugin.sysstat.main-load.sending_measurements"))
        .await
        .into_iter()
        .map(|res| {
            res.map_err(|_| PluginError::from(crate::error::Error::FailedToSendMeasurement))
                .map(|_| ())
        })
        .collect::<Result<Vec<_>, PluginError>>()
        .map(|_| ())
}
