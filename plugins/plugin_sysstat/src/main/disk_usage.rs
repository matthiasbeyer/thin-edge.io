use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use futures::StreamExt;
use sysinfo::DiskExt;
use sysinfo::SystemExt;
use tedge_lib::iter::SendAllResult;
use tokio::sync::Mutex;
use tracing::Instrument;

use tedge_api::Address;
use tedge_api::Message;
use tedge_api::PluginError;
use tedge_lib::address::AddressGroup;
use tedge_lib::iter::IntoSendAll;
use tedge_lib::measurement::Measurement;
use tedge_lib::measurement::MeasurementValue;

use crate::plugin::MeasurementReceiver;

use crate::config::HasBaseConfig;
use crate::main::State;
use crate::main::StateFromConfig;

#[derive(Debug)]
pub struct DiskUsageState {
    interval: u64,
    send_to: Arc<AddressGroup<MeasurementReceiver>>,
    sys: sysinfo::System,
}

impl State for DiskUsageState {
    fn interval(&self) -> u64 {
        self.interval
    }
}

impl StateFromConfig for DiskUsageState {
    fn new_from_config(
        config: &crate::config::SysStatConfig,
        addrs: Arc<AddressGroup<MeasurementReceiver>>,
    ) -> Option<Self> {
        config.disk_usage.as_ref().map(|config| DiskUsageState {
            interval: config.interval_ms().get(),
            send_to: addrs,
            sys: sysinfo::System::new_with_specifics({ sysinfo::RefreshKind::new().with_disks() }),
        })
    }
}

#[tracing::instrument(name = "plugin.sysstat.main-diskusage", skip(state))]
pub async fn main_disk_usage(state: Arc<Mutex<DiskUsageState>>) -> Result<(), PluginError> {
    use futures::stream::StreamExt;

    let mut lock = state.lock().await;
    lock.deref_mut().sys.refresh_disks_list();
    lock.deref_mut().sys.refresh_disks();

    let streams = lock
        .deref()
        .sys
        .disks()
        .into_iter()
        .map(|disk| measure_to_message(disk).map(|msg| lock.send_to.send_and_wait(msg)))
        .collect::<Result<Vec<_>, PluginError>>()?;

    futures::stream::iter(streams)
        .flatten()
        .collect::<SendAllResult<Measurement>>()
        .instrument(tracing::debug_span!(
            "plugin.sysstat.main-diskusage.sending_measurements"
        ))
        .await
        .into_result()
        .map_err(|_| crate::error::Error::FailedToSendMeasurement)?;
    Ok(())
}

fn measure_to_message(disk: &sysinfo::Disk) -> Result<Measurement, PluginError> {
    let disk_name = disk
        .name()
        .to_os_string()
        .into_string()
        .map_err(|_| crate::error::Error::CannotReadDiskName)?;

    let disk_fs =
        std::str::from_utf8(disk.file_system()).map_err(crate::error::Error::Utf8Error)?;

    let disk_type = match disk.type_() {
        sysinfo::DiskType::HDD => "HDD",
        sysinfo::DiskType::SSD => "SSD",
        sysinfo::DiskType::Unknown(_) => "Unknown",
    };
    let disk_mountpoint = disk.mount_point().display();
    let disk_totalspace = disk.total_space();
    let disk_availspace = disk.available_space();
    let disk_removable = disk.is_removable();

    let mut hm = HashMap::new();
    hm.insert(
        "fs".to_string(),
        MeasurementValue::Text(disk_fs.to_string()),
    );
    hm.insert(
        "type".to_string(),
        MeasurementValue::Text(disk_type.to_string()),
    );
    hm.insert(
        "mountpoint".to_string(),
        MeasurementValue::Text(disk_mountpoint.to_string()),
    );
    hm.insert(
        "total".to_string(),
        MeasurementValue::Float(disk_totalspace as f64),
    );
    hm.insert(
        "avail".to_string(),
        MeasurementValue::Float(disk_availspace as f64),
    );
    hm.insert(
        "removable".to_string(),
        MeasurementValue::Bool(disk_removable),
    );
    let value = MeasurementValue::Map(hm);
    Ok(Measurement::new(disk_name, value))
}
