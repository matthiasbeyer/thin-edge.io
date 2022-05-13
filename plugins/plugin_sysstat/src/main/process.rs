use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use futures::StreamExt;
use sysinfo::PidExt;
use sysinfo::ProcessExt;
use sysinfo::SystemExt;
use tokio::sync::Mutex;

use tedge_api::Address;
use tedge_api::PluginError;
use tedge_lib::iter::IntoSendAll;
use tedge_lib::measurement::Measurement;
use tedge_lib::measurement::MeasurementValue;
use tracing::Instrument;

use crate::config::AllProcessConfig;
use crate::config::HasBaseConfig;
use crate::config::ProcessStatConfig;
use crate::main::State;
use crate::main::StateFromConfig;
use crate::plugin::MeasurementReceiver;

#[derive(Debug)]
pub struct ProcessState {
    interval: u64,
    send_to: Arc<Vec<Address<MeasurementReceiver>>>,
    sys: sysinfo::System,

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
        addrs: Arc<Vec<Address<MeasurementReceiver>>>,
    ) -> Option<Self> {
        config.process.as_ref().map(|config| ProcessState {
            interval: config.interval_ms().get(),
            send_to: addrs,
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

            all_processes: config.all_processes.clone(),
            by_name: config.by_name.clone(),
        })
    }
}

#[tracing::instrument(name = "plugin.sysstat.main-process", skip(state))]
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
        .map(|value| {
            let measurement = Measurement::new("processes".to_string(), value);
            std::iter::repeat(measurement).zip(state.send_to.iter())
        })
        .flatten()
        .collect::<Vec<_>>();

    messages
        .into_iter()
        .send_all()
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<Result<_, _>>>()
        .instrument(tracing::debug_span!(
            "plugin.sysstat.main-process.sending_measurements"
        ))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| PluginError::from(crate::error::Error::FailedToSendMeasurement))
        .map(|_| ())
}

fn get_measurement(state: &ProcessState, process: &sysinfo::Process) -> MeasurementValue {
    let mut hm = HashMap::new();

    macro_rules! mk_measurement {
        ($all_config:expr, $cfgclosure:expr, $measurement:expr, $measurement_name:expr) => {
            if $all_config
                || state
                    .by_name
                    .get(process.name())
                    .map($cfgclosure)
                    .unwrap_or(false)
            {
                hm.insert($measurement_name.to_string(), $measurement);
            }
        };
    }

    mk_measurement!(
        state.all_processes.config.cmd,
        |cfg| cfg.cmd,
        { MeasurementValue::Text(process.name().to_string()) },
        "cmd"
    );

    mk_measurement!(
        state.all_processes.config.cwd,
        |cfg| cfg.cwd,
        { MeasurementValue::Text(process.cwd().display().to_string()) },
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
            MeasurementValue::Map({
                let mut hm = HashMap::new();
                hm.insert(
                    "total_written_bytes".to_string(),
                    MeasurementValue::Float(du.total_written_bytes as f64),
                );
                hm.insert(
                    "written_bytes".to_string(),
                    MeasurementValue::Float(du.written_bytes as f64),
                );
                hm.insert(
                    "total_read_bytes".to_string(),
                    MeasurementValue::Float(du.total_read_bytes as f64),
                );
                hm.insert(
                    "read_bytes".to_string(),
                    MeasurementValue::Float(du.read_bytes as f64),
                );
                hm
            })
        },
        "disk_usage"
    );

    // Currently not supported because we do not yet have a way to report a list of values
    // (without naming each)
    // mk_measurement!(
    //     state.all_processes.config.environ,
    //     |cfg| cfg.environ,
    //     { process.environ().into_iter()
    //         .map(|env| MeasurementValue::Text(env.clone()))
    //         .collect()
    //     ""
    // );

    mk_measurement!(
        state.all_processes.config.exe,
        |cfg| cfg.exe,
        { MeasurementValue::Text(process.exe().display().to_string()) },
        "exe"
    );

    mk_measurement!(
        state.all_processes.config.memory,
        |cfg| cfg.memory,
        { MeasurementValue::Float(process.memory() as f64) },
        "memory"
    );

    mk_measurement!(
        state.all_processes.config.name,
        |cfg| cfg.name,
        { MeasurementValue::Text(process.name().to_string()) },
        "name"
    );

    mk_measurement!(
        state.all_processes.config.pid,
        |cfg| cfg.pid,
        { MeasurementValue::Float(process.pid().as_u32() as f64) },
        "pid"
    );

    mk_measurement!(
        state.all_processes.config.root,
        |cfg| cfg.root,
        { MeasurementValue::Text(process.root().display().to_string()) },
        "root"
    );

    mk_measurement!(
        state.all_processes.config.run_time,
        |cfg| cfg.run_time,
        { MeasurementValue::Float(process.run_time() as f64) },
        "run_time"
    );

    mk_measurement!(
        state.all_processes.config.start_time,
        |cfg| cfg.start_time,
        { MeasurementValue::Float(process.start_time() as f64) },
        "start_time"
    );

    mk_measurement!(
        state.all_processes.config.vmemory,
        |cfg| cfg.vmemory,
        { MeasurementValue::Float(process.virtual_memory() as f64) },
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
            let m = MeasurementValue::Float(parent_pid.as_u32() as f64);
            hm.insert("parent".to_string(), m);
        }
    }

    MeasurementValue::Map(hm)
}
