use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use futures::StreamExt;
use sysinfo::SystemExt;
use tokio::sync::Mutex;

use tedge_api::address::EndpointKind;
use tedge_api::message::MeasurementValue;
use tedge_api::plugin::CoreCommunication;
use tedge_api::Address;
use tedge_api::Message;
use tedge_api::MessageKind;
use tedge_api::PluginError;
use tedge_lib::iter::IntoSendAll;
use tedge_lib::iter::MapSendResult;
use tedge_lib::reply::IntoReplyable;

use crate::config::HasBaseConfig;
use crate::main::State;
use crate::main::StateFromConfig;

pub struct MemoryState {
    interval: u64,
    send_to: Vec<String>,
    sys: sysinfo::System,
    comms: CoreCommunication,

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
        comms: CoreCommunication,
    ) -> Option<Self> {
        config.memory.as_ref().map(|config| MemoryState {
            interval: config.interval_ms().get(),
            send_to: config.send_to().to_vec(),
            sys: sysinfo::System::new_with_specifics({ sysinfo::RefreshKind::new().with_memory() }),
            comms,

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
    let lock = state.lock().await;
    let state = lock.deref();
    let timeout_duration = std::time::Duration::from_millis(state.interval);
    let mut aggregate = Vec::new();

    macro_rules! measure {
        ($config:expr, $name:expr, $value:expr) => {
            if $config {
                let name = $name.to_string();
                let value = MeasurementValue::Int($value);
                aggregate.push((name, value));
            }
        };
    }

    measure!(
        state.total_memory,
        state.total_memory_name,
        state.sys.total_memory()
    );
    measure!(
        state.free_memory,
        state.free_memory_name,
        state.sys.free_memory()
    );
    measure!(
        state.available_memory,
        state.available_memory_name,
        state.sys.available_memory()
    );
    measure!(
        state.used_memory,
        state.used_memory_name,
        state.sys.used_memory()
    );
    measure!(state.free_swap, state.free_swap_name, state.sys.free_swap());
    measure!(state.used_swap, state.used_swap_name, state.sys.used_swap());

    let value = MeasurementValue::Aggregate(aggregate);

    state
        .send_to
        .iter()
        .map(|target| {
            let addr = Address::new(EndpointKind::Plugin { id: target.clone() });
            let kind = MessageKind::Measurement {
                name: "memory".to_string(),
                value: value.clone(),
            };

            (kind, addr)
        })
        .send_all(state.comms.clone())
        .wait_for_reply()
        .with_timeout(timeout_duration)
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<Result<tedge_lib::iter::SendResult, PluginError>>>()
        .await
        .into_iter()
        .map_send_result(tedge_lib::iter::log_and_ignore_timeout)
        .collect::<Result<Vec<_>, PluginError>>()
        .map(|_| ())
}
