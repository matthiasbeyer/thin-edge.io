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

pub struct LoadState {
    interval: u64,
    send_to: Vec<String>,
    sys: sysinfo::System,
    comms: CoreCommunication,
}

impl State for LoadState {
    fn interval(&self) -> u64 {
        self.interval
    }
}

impl StateFromConfig for LoadState {
    fn new_from_config(
        config: &crate::config::SysStatConfig,
        comms: CoreCommunication,
    ) -> Option<Self> {
        config.load.as_ref().map(|config| LoadState {
            interval: config.interval_ms().get(),
            send_to: config.send_to().to_vec(),
            sys: sysinfo::System::new(),
            comms,
        })
    }
}

pub async fn main_load(state: Arc<Mutex<LoadState>>) -> Result<(), PluginError> {
    let lock = state.lock().await;
    let state = lock.deref();
    let timeout_duration = std::time::Duration::from_millis(state.interval);
    let load = state.sys.load_average();

    let value = MeasurementValue::Aggregate(vec![
        (String::from("one"), MeasurementValue::Float(load.one)),
        (String::from("five"), MeasurementValue::Float(load.five)),
        (
            String::from("fifteen"),
            MeasurementValue::Float(load.fifteen),
        ),
    ]);

    state
        .send_to
        .iter()
        .map(|target| {
            let addr = Address::new(EndpointKind::Plugin { id: target.clone() });
            let kind = MessageKind::Measurement {
                name: "load".to_string(),
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
