use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use futures::StreamExt;
use sysinfo::SystemExt;
use tedge_lib::iter::SendAllResult;
use tokio::sync::Mutex;

use tedge_api::Address;
use tedge_api::PluginError;
use tedge_lib::address::AddressGroup;
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
    send_to: Arc<AddressGroup<MeasurementReceiver>>,
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
        addrs: Arc<AddressGroup<MeasurementReceiver>>,
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
    lock.deref_mut().sys.refresh_cpu(); // assuming that this is the required refresh call
                                        //
    let state = lock.deref();
    let message = {
        let load = state.sys.load_average();

        let mut hm = HashMap::new();
        hm.insert(String::from("one"), MeasurementValue::Float(load.one));
        hm.insert(String::from("five"), MeasurementValue::Float(load.five));
        hm.insert(
            String::from("fifteen"),
            MeasurementValue::Float(load.fifteen),
        );
        let value = MeasurementValue::Map(hm);
        Measurement::new("load".to_string(), value)
    };

    state
        .send_to
        .send_and_wait(message)
        .collect::<SendAllResult<Measurement>>()
        .instrument(tracing::debug_span!(
            "plugin.sysstat.main-load.sending_measurements"
        ))
        .await
        .into_result()
        .map_err(|_| crate::error::Error::FailedToSendMeasurement)?;
    Ok(())
}
