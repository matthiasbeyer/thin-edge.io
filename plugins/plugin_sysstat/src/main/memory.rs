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
pub struct MemoryState {
    interval: u64,
    send_to: Arc<AddressGroup<MeasurementReceiver>>,
    sys: sysinfo::System,

    total_memory: bool,
    total_memory_name: String,

    free_memory: bool,
    free_memory_name: String,

    available_memory: bool,
    available_memory_name: String,

    used_memory: bool,
    used_memory_name: String,

    free_swap: bool,
    free_swap_name: String,

    used_swap: bool,
    used_swap_name: String,
}

impl State for MemoryState {
    fn interval(&self) -> u64 {
        self.interval
    }
}

impl StateFromConfig for MemoryState {
    fn new_from_config(
        config: &crate::config::SysStatConfig,
        addrs: Arc<AddressGroup<MeasurementReceiver>>,
    ) -> Option<Self> {
        config.memory.as_ref().map(|config| MemoryState {
            interval: config.interval_ms().get(),
            send_to: addrs,
            sys: sysinfo::System::new_with_specifics({ sysinfo::RefreshKind::new().with_memory() }),

            total_memory: config.total_memory,
            total_memory_name: config.total_memory_name.clone(),

            free_memory: config.free_memory,
            free_memory_name: config.free_memory_name.clone(),

            available_memory: config.available_memory,
            available_memory_name: config.available_memory_name.clone(),

            used_memory: config.used_memory,
            used_memory_name: config.used_memory_name.clone(),

            free_swap: config.free_swap,
            free_swap_name: config.free_swap_name.clone(),

            used_swap: config.used_swap,
            used_swap_name: config.used_swap_name.clone(),
        })
    }
}

#[tracing::instrument(name = "plugin.sysstat.main-memory", skip(state))]
pub async fn main_memory(state: Arc<Mutex<MemoryState>>) -> Result<(), PluginError> {
    let mut lock = state.lock().await;
    let mut state = lock.deref_mut();
    state.sys.refresh_memory();
    let state = lock.deref();
    let mut hm = HashMap::new();

    macro_rules! measure {
        ($config:expr, $name:expr, $value:expr) => {
            if $config {
                let name = $name.to_string();
                let value = MeasurementValue::Float($value);
                hm.insert(name, value);
            }
        };
    }

    measure!(
        state.total_memory,
        state.total_memory_name,
        state.sys.total_memory() as f64
    );
    measure!(
        state.free_memory,
        state.free_memory_name,
        state.sys.free_memory() as f64
    );
    measure!(
        state.available_memory,
        state.available_memory_name,
        state.sys.available_memory() as f64
    );
    measure!(
        state.used_memory,
        state.used_memory_name,
        state.sys.used_memory() as f64
    );
    measure!(
        state.free_swap,
        state.free_swap_name,
        state.sys.free_swap() as f64
    );
    measure!(
        state.used_swap,
        state.used_swap_name,
        state.sys.used_swap() as f64
    );

    let value = MeasurementValue::Map(hm);
    let measurement = Measurement::new("memory".to_string(), value);

    state
        .send_to
        .send_and_wait(measurement)
        .collect::<SendAllResult<Measurement>>()
        .instrument(tracing::debug_span!(
            "plugin.sysstat.main-memory.sending_measurements"
        ))
        .await
        .into_result()
        .map_err(|_| crate::error::Error::FailedToSendMeasurement)?;
    Ok(())
}
