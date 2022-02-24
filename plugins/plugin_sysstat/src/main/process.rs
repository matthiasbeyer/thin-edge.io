use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use futures::StreamExt;
use sysinfo::PidExt;
use sysinfo::ProcessExt;
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

use crate::config::AllProcessConfig;
use crate::config::HasBaseConfig;
use crate::config::ProcessStatConfig;
use crate::main::State;
use crate::main::StateFromConfig;

pub struct ProcessState {
    interval: u64,
    send_to: Vec<String>,
    sys: sysinfo::System,
    comms: CoreCommunication,

    all_processes: AllProcessConfig,
    by_name: HashMap<String, ProcessStatConfig>,
}

impl State for ProcessState {
    fn interval(&self) -> u64 {
        self.interval
    }
}

impl StateFromConfig for ProcessState {
    fn new_from_config(
        config: &crate::config::SysStatConfig,
        comms: CoreCommunication,
    ) -> Option<Self> {
        config.process.as_ref().map(|config| ProcessState {
            interval: config.interval_ms().get(),
            send_to: config.send_to().to_vec(),
            sys: sysinfo::System::new_with_specifics({
                sysinfo::RefreshKind::new().with_processes({
                    let pr = sysinfo::ProcessRefreshKind::new();
                    let pr = if config.all_processes.config.cpu_usage {
                        pr.with_cpu()
                    } else {
                        pr
                    };
                    let pr = if config.all_processes.config.disk_usage {
                        pr.with_disk_usage()
                    } else {
                        pr
                    };

                    pr
                })
            }),
            comms,

            all_processes: config.all_processes.clone(),
            by_name: config.by_name.clone(),
        })
    }
}

pub async fn main_process(state: Arc<Mutex<ProcessState>>) -> Result<(), PluginError> {
    let mut lock = state.lock().await;
    let mut state = lock.deref();
    let timeout_duration = std::time::Duration::from_millis(state.interval);

    let messages: Vec<_> = state
        .sys
        .processes()
        .iter()
        .filter(|(_pid, process)| {
            state.all_processes.enable
                || state
                    .by_name
                    .keys()
                    .find(|name| *name == process.name())
                    .is_some()
        })
        .map(|(_pid, process)| get_measurement(&state, process))
        .map(|measurement| {
            state.send_to.iter().map(move |target| {
                let addr = Address::new(EndpointKind::Plugin { id: target.clone() });
                let kind = MessageKind::Measurement {
                    name: "processes".to_string(),
                    value: measurement.clone(),
                };

                (kind, addr)
            })
        })
        .flatten()
        .collect::<Vec<_>>();

    messages
        .into_iter()
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
}

fn get_measurement(state: &ProcessState, process: &sysinfo::Process) -> MeasurementValue {
    let mut measurements = vec![];

    macro_rules! mk_measurement {
        ($all_config:expr, $cfgclosure:expr, $measurement:expr, $measurement_name:expr) => {
            if $all_config
                || state
                    .by_name
                    .get(process.name())
                    .map($cfgclosure)
                    .unwrap_or(false)
            {
                let m = $measurement;
                measurements.push(($measurement_name.to_string(), m));
            }
        };
    }

    mk_measurement!(
        state.all_processes.config.cmd,
        |cfg| cfg.cmd,
        { MeasurementValue::Str(process.name().to_string()) },
        "cmd"
    );

    mk_measurement!(
        state.all_processes.config.cwd,
        |cfg| cfg.cwd,
        { MeasurementValue::Str(process.cwd().display().to_string()) },
        "cwd"
    );

    mk_measurement!(
        state.all_processes.config.cpu_usage,
        |cfg| cfg.cpu_usage,
        { MeasurementValue::Float(process.cpu_usage().into()) },
        "cpu_usage"
    );

    mk_measurement!(
        state.all_processes.config.disk_usage,
        |cfg| cfg.disk_usage,
        {
            let du = process.disk_usage();
            MeasurementValue::Aggregate(vec![
                (
                    "total_written_bytes".to_string(),
                    MeasurementValue::Int(du.total_written_bytes.into()),
                ),
                (
                    "written_bytes".to_string(),
                    MeasurementValue::Int(du.written_bytes.into()),
                ),
                (
                    "total_read_bytes".to_string(),
                    MeasurementValue::Int(du.total_read_bytes.into()),
                ),
                (
                    "read_bytes".to_string(),
                    MeasurementValue::Int(du.read_bytes.into()),
                ),
            ])
        },
        "disk_usage"
    );

    // Currently not supported because we do not yet have a way to report a list of values
    // (without naming each)
    // mk_measurement!(
    //     state.all_processes.config.environ,
    //     |cfg| cfg.environ,
    //     { process.environ().into_iter()
    //         .map(|env| MeasurementValue::Str(env.clone()))
    //         .collect()
    //     ""
    // );

    mk_measurement!(
        state.all_processes.config.exe,
        |cfg| cfg.exe,
        { MeasurementValue::Str(process.exe().display().to_string()) },
        "exe"
    );

    mk_measurement!(
        state.all_processes.config.memory,
        |cfg| cfg.memory,
        { MeasurementValue::Int(process.memory()) },
        "memory"
    );

    mk_measurement!(
        state.all_processes.config.name,
        |cfg| cfg.name,
        { MeasurementValue::Str(process.name().to_string()) },
        "name"
    );

    mk_measurement!(
        state.all_processes.config.pid,
        |cfg| cfg.pid,
        { MeasurementValue::Int(process.pid().as_u32().into()) },
        "pid"
    );

    mk_measurement!(
        state.all_processes.config.root,
        |cfg| cfg.root,
        { MeasurementValue::Str(process.root().display().to_string()) },
        "root"
    );

    mk_measurement!(
        state.all_processes.config.run_time,
        |cfg| cfg.run_time,
        { MeasurementValue::Int(process.run_time()) },
        "run_time"
    );

    mk_measurement!(
        state.all_processes.config.start_time,
        |cfg| cfg.start_time,
        { MeasurementValue::Int(process.start_time()) },
        "start_time"
    );

    mk_measurement!(
        state.all_processes.config.vmemory,
        |cfg| cfg.vmemory,
        { MeasurementValue::Int(process.virtual_memory()) },
        "vmemory"
    );

    if state.all_processes.config.parent
        || state
            .by_name
            .get(process.name())
            .map(|cfg| cfg.parent)
            .unwrap_or(false)
    {
        if let Some(parent_pid) = process.parent() {
            let m = MeasurementValue::Int(parent_pid.as_u32().into());
            measurements.push(("parent".to_string(), m));
        }
    }

    MeasurementValue::Aggregate(measurements)
}
