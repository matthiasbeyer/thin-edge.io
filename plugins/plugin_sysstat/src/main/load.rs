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

use crate::config::HasBaseConfig;
use crate::main::State;
use crate::main::StateFromConfig;
use crate::plugin::MeasurementReceiver;

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

pub async fn main_load(state: Arc<Mutex<LoadState>>) -> Result<(), PluginError> {
    let lock = state.lock().await;
    let state = lock.deref();
    let timeout_duration = std::time::Duration::from_millis(state.interval);
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
        .wait_for_reply(timeout_duration)
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<Result<tedge_lib::iter::SendResult<_>, _>>>()
        .await
        .into_iter()
        .map(|res| {
            res.map_err(|_| PluginError::from(anyhow::anyhow!("Failed to send measurement")))
                .map(|_| ())
        })
        .collect::<Result<Vec<_>, PluginError>>()
        .map(|_| ())
}
