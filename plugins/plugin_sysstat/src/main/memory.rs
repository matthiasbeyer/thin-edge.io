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

pub struct MemoryState {
    interval: u64,
    send_to: Arc<Vec<Address<MeasurementReceiver>>>,
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
        addrs: Arc<Vec<Address<MeasurementReceiver>>>,
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

pub async fn main_memory(state: Arc<Mutex<MemoryState>>) -> Result<(), PluginError> {
    let mut lock = state.lock().await;
    let mut state = lock.deref_mut();
    state.sys.refresh_memory();
    let state = lock.deref();
    let timeout_duration = std::time::Duration::from_millis(state.interval);
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

    std::iter::repeat(measurement)
        .zip(state.send_to.iter())
        .send_all()
        .wait_for_reply(timeout_duration)
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<Result<tedge_lib::iter::SendResult<_>, _>>>()
        .await
        .into_iter()
        .map(|res| {
            res.map_err(|_| PluginError::from(miette::miette!("Failed to send measurement")))
                .map(|_| ())
        })
        .collect::<Result<Vec<()>, PluginError>>()
        .map(|_| ())
}
