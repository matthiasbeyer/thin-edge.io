use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use futures::StreamExt;
use sysinfo::DiskExt;
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
use tedge_lib::reply::ReplyableCoreCommunication;

use crate::config::HasBaseConfig;
use crate::main::State;
use crate::main::StateFromConfig;

pub struct DiskUsageState {
    interval: u64,
    send_to: Vec<String>,
    sys: sysinfo::System,
    comms: CoreCommunication,
}

impl State for DiskUsageState {
    fn interval(&self) -> u64 {
        self.interval
    }
}

impl StateFromConfig for DiskUsageState {
    fn new_from_config(
        config: &crate::config::SysStatConfig,
        comms: CoreCommunication,
    ) -> Option<Self> {
        config.disk_usage.as_ref().map(|config| DiskUsageState {
            interval: config.interval_ms().get(),
            send_to: config.send_to().to_vec(),
            sys: sysinfo::System::new_with_specifics({ sysinfo::RefreshKind::new().with_disks() }),
            comms,
        })
    }
}

pub async fn main_disk_usage(state: Arc<Mutex<DiskUsageState>>) -> Result<(), PluginError> {
    let mut lock = state.lock().await;

    let timeout_duration = std::time::Duration::from_millis(lock.deref().interval);
    lock.deref_mut().sys.refresh_disks();
    lock.deref()
        .sys
        .disks()
        .into_iter()
        .map(|disk| async {
            measure_to_messages(lock.deref(), &lock.deref().send_to, disk)?
                .send_all(lock.deref().comms.clone())
                .wait_for_reply()
                .with_timeout(timeout_duration)
                .collect::<futures::stream::FuturesUnordered<_>>()
                .collect::<Vec<Result<_, PluginError>>>()
                .await
                .into_iter()
                .map_send_result(tedge_lib::iter::log_and_ignore_timeout)
                .collect::<Result<Vec<_>, PluginError>>()
                .map(|_| ())
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<Result<(), PluginError>>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, PluginError>>()
        .map(|_| ())
}

fn measure_to_messages<'a>(
    state: &'a DiskUsageState,
    targets: &'a [impl AsRef<str>],
    disk: &sysinfo::Disk,
) -> Result<impl Iterator<Item = (MessageKind, Address)> + 'a, PluginError> {
    let disk_name = disk
        .name()
        .to_os_string()
        .into_string()
        .map_err(|_| anyhow::anyhow!("Cannot read disk name"))?;

    let disk_fs = std::str::from_utf8(disk.file_system())
        .map_err(|_| anyhow::anyhow!("Disk Filesystem name not valid UTF-8"))?;

    let disk_type = match disk.type_() {
        sysinfo::DiskType::HDD => "HDD",
        sysinfo::DiskType::SSD => "SSD",
        sysinfo::DiskType::Unknown(_) => "Unknown",
    };
    let disk_mountpoint = disk.mount_point().display();
    let disk_totalspace = disk.total_space();
    let disk_availspace = disk.available_space();
    let disk_removable = disk.is_removable();

    let measurement = MeasurementValue::Aggregate(vec![
        ("fs".to_string(), MeasurementValue::Str(disk_fs.to_string())),
        (
            "type".to_string(),
            MeasurementValue::Str(disk_type.to_string()),
        ),
        (
            "mountpoint".to_string(),
            MeasurementValue::Str(disk_mountpoint.to_string()),
        ),
        ("total".to_string(), MeasurementValue::Int(disk_totalspace)),
        ("avail".to_string(), MeasurementValue::Int(disk_availspace)),
        (
            "removable".to_string(),
            MeasurementValue::Bool(disk_removable),
        ),
    ]);

    let iter = targets.into_iter().map(move |target| {
        let addr = Address::new(EndpointKind::Plugin {
            id: target.as_ref().to_string(),
        });
        let kind = MessageKind::Measurement {
            name: disk_name.to_string(),
            value: measurement.clone(),
        };

        (kind, addr)
    });

    Ok(iter)
}
