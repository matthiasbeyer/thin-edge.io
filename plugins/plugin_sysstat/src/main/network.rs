use std::collections::HashMap;
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

use crate::config::AllNetworkStatConfig;
use crate::config::HasBaseConfig;
use crate::config::NetworkStatConfig;
use crate::main::State;
use crate::main::StateFromConfig;

pub struct NetworkState {
    interval: u64,
    send_to: Vec<String>,
    sys: sysinfo::System,
    comms: CoreCommunication,

    all_networks: AllNetworkStatConfig,
    by_name: HashMap<String, NetworkStatConfig>,
}

impl State for NetworkState {
    fn interval(&self) -> u64 {
        self.interval
    }
}

impl StateFromConfig for NetworkState {
    fn new_from_config(
        config: &crate::config::SysStatConfig,
        comms: CoreCommunication,
    ) -> Option<Self> {
        config.network.as_ref().map(|config| NetworkState {
            interval: config.interval_ms().get(),
            send_to: config.send_to().to_vec(),
            sys: sysinfo::System::new_with_specifics({
                sysinfo::RefreshKind::new().with_networks()
            }),
            comms,

            all_networks: config.all_networks.clone(),
            by_name: config.by_name.clone(),
        })
    }
}

pub async fn main_network(state: Arc<Mutex<NetworkState>>) -> Result<(), PluginError> {
    use sysinfo::NetworkExt;

    let lock = state.lock().await;
    let state = lock.deref();
    let timeout_duration = std::time::Duration::from_millis(state.interval);

    let messages = state
        .sys
        .networks()
        .into_iter()
        .filter(|(name, _)| {
            state.all_networks.enable || state.by_name.keys().find(|n| n == name).is_some()
        })
        .map(|(name, network)| {
            let config = if state.all_networks.enable {
                &state.all_networks.config
            } else {
                state.by_name.get(name).unwrap() // TODO this cannot fail because of above filtering. Make me nice.
            };

            let measurement = get_network_info_measurements(network, config);

            (name, measurement)
        })
        .map(|(name, measurement)| {
            state.send_to.iter().map(move |target| {
                let addr = Address::new(EndpointKind::Plugin { id: target.clone() });
                let kind = MessageKind::Measurement {
                    name: name.to_string(),
                    value: measurement.clone(),
                };

                (kind, addr)
            })
        })
        .flatten()
        .collect::<Vec<_>>();

    messages
        .into_iter()
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

fn get_network_info_measurements(
    info: &sysinfo::NetworkData,
    config: &NetworkStatConfig,
) -> MeasurementValue {
    use sysinfo::NetworkExt;

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

    measure!(config.received, config.received_name, info.received());
    measure!(
        config.total_received,
        config.total_received_name,
        info.total_received()
    );
    measure!(
        config.transmitted,
        config.transmitted_name,
        info.transmitted()
    );
    measure!(
        config.total_transmitted,
        config.total_transmitted_name,
        info.total_transmitted()
    );
    measure!(
        config.packets_received,
        config.packets_received_name,
        info.packets_received()
    );
    measure!(
        config.total_packets_received,
        config.total_packets_received_name,
        info.total_packets_received()
    );
    measure!(
        config.packets_transmitted,
        config.packets_transmitted_name,
        info.packets_transmitted()
    );
    measure!(
        config.total_packets_transmitted,
        config.total_packets_transmitted_name,
        info.total_packets_transmitted()
    );
    measure!(
        config.errors_on_received,
        config.errors_on_received_name,
        info.errors_on_received()
    );
    measure!(
        config.total_errors_on_received,
        config.total_errors_on_received_name,
        info.total_errors_on_received()
    );

    MeasurementValue::Aggregate(aggregate)
}
