use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use futures::StreamExt;
use sysinfo::SystemExt;
use tedge_lib::iter::SendAllResult;
use tedge_lib::measurement::Measurement;
use tedge_lib::measurement::MeasurementValue;
use tokio::sync::Mutex;

use tedge_api::Address;
use tedge_api::PluginError;
use tedge_lib::address::AddressGroup;
use tedge_lib::iter::IntoSendAll;
use tracing::Instrument;

use crate::config::AllNetworkStatConfig;
use crate::config::HasBaseConfig;
use crate::config::NetworkStatConfig;
use crate::main::State;
use crate::main::StateFromConfig;
use crate::plugin::MeasurementReceiver;

#[derive(Debug)]
pub struct NetworkState {
    interval: u64,
    send_to: Arc<AddressGroup<MeasurementReceiver>>,
    sys: sysinfo::System,

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
        addrs: Arc<AddressGroup<MeasurementReceiver>>,
    ) -> Option<Self> {
        config.network.as_ref().map(|config| NetworkState {
            interval: config.interval_ms().get(),
            send_to: addrs,
            sys: sysinfo::System::new_with_specifics({
                sysinfo::RefreshKind::new().with_networks()
            }),

            all_networks: config.all_networks.clone(),
            by_name: config.by_name.clone(),
        })
    }
}

#[tracing::instrument(name = "plugin.sysstat.main-networks", skip(state))]
pub async fn main_network(state: Arc<Mutex<NetworkState>>) -> Result<(), PluginError> {
    use sysinfo::NetworkExt;

    let lock = state.lock().await;
    let state = lock.deref();

    let measurements = state
        .sys
        .networks()
        .into_iter()
        .filter(|(name, _)| state.all_networks.enable || state.by_name.keys().any(|n| n == *name))
        .map(|(name, network)| {
            let config = if state.all_networks.enable {
                &state.all_networks.config
            } else {
                state.by_name.get(name).unwrap() // TODO this cannot fail because of above filtering. Make me nice.
            };

            let value = get_network_info_measurements(network, config);
            Measurement::new(name.to_string(), value)
        })
        .collect::<Vec<_>>();

    futures::stream::iter(measurements)
        .map(|msmt| state.send_to.send_and_wait(msmt))
        .flatten()
        .collect::<SendAllResult<Measurement>>()
        .instrument(tracing::debug_span!(
            "plugin.sysstat.main-networks.sending_measurements"
        ))
        .await
        .into_result()
        .map_err(|_| crate::error::Error::FailedToSendMeasurement)?;
    Ok(())
}

fn get_network_info_measurements(
    info: &sysinfo::NetworkData,
    config: &NetworkStatConfig,
) -> MeasurementValue {
    use sysinfo::NetworkExt;

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
        config.received,
        config.received_name,
        info.received() as f64
    );
    measure!(
        config.total_received,
        config.total_received_name,
        info.total_received() as f64
    );
    measure!(
        config.transmitted,
        config.transmitted_name,
        info.transmitted() as f64
    );
    measure!(
        config.total_transmitted,
        config.total_transmitted_name,
        info.total_transmitted() as f64
    );
    measure!(
        config.packets_received,
        config.packets_received_name,
        info.packets_received() as f64
    );
    measure!(
        config.total_packets_received,
        config.total_packets_received_name,
        info.total_packets_received() as f64
    );
    measure!(
        config.packets_transmitted,
        config.packets_transmitted_name,
        info.packets_transmitted() as f64
    );
    measure!(
        config.total_packets_transmitted,
        config.total_packets_transmitted_name,
        info.total_packets_transmitted() as f64
    );
    measure!(
        config.errors_on_received,
        config.errors_on_received_name,
        info.errors_on_received() as f64
    );
    measure!(
        config.total_errors_on_received,
        config.total_errors_on_received_name,
        info.total_errors_on_received() as f64
    );

    MeasurementValue::Map(hm)
}
