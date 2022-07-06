use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use futures::FutureExt;
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

use crate::config::HasBaseConfig;
use crate::config::PhysicalCoreCountConfig;
use crate::config::ProcessorInfoConfig;
use crate::main::State;
use crate::main::StateFromConfig;
use crate::plugin::MeasurementReceiver;

#[derive(Debug)]
pub struct CPUState {
    interval: u64,
    sys: sysinfo::System,
    addrs: Arc<AddressGroup<MeasurementReceiver>>,

    report_global_processor_info: ProcessorInfoConfig,
    global_processor_info_name: String,

    report_processor_info: ProcessorInfoConfig,
    processor_info_name: String,

    report_physical_core_count: PhysicalCoreCountConfig,
    physical_core_count_name: String,
}

impl State for CPUState {
    fn interval(&self) -> u64 {
        self.interval
    }
}

impl StateFromConfig for CPUState {
    fn new_from_config(
        config: &crate::config::SysStatConfig,
        addrs: Arc<AddressGroup<MeasurementReceiver>>,
    ) -> Option<Self> {
        config.cpu.as_ref().map(|config| CPUState {
            interval: config.interval_ms().get(),

            sys: sysinfo::System::new_with_specifics({ sysinfo::RefreshKind::new().with_cpu() }),

            addrs,

            report_global_processor_info: config.report_global_processor_info.clone(),
            global_processor_info_name: config.global_processor_info_name.clone(),

            report_processor_info: config.report_processor_info.clone(),
            processor_info_name: config.processor_info_name.clone(),

            report_physical_core_count: config.report_physical_core_count.clone(),
            physical_core_count_name: config.physical_core_count_name.clone(),
        })
    }
}

#[tracing::instrument(name = "plugin.sysstat.main-cpu", skip(state))]
pub async fn main_cpu(state: Arc<Mutex<CPUState>>) -> Result<(), PluginError> {
    let mut lock = state.lock().await;
    let mut state = lock.deref_mut();
    state.sys.refresh_cpu();
    let state = lock.deref();

    let mut sending = Vec::new();

    if state.report_global_processor_info.enable {
        let info = state.sys.global_processor_info();
        let measurement = get_processor_info_measurements(
            info,
            state.report_global_processor_info.frequency,
            &state.report_global_processor_info.frequency_name,
            state.report_global_processor_info.cpu_usage,
            &state.report_global_processor_info.cpu_usage_name,
            state.report_global_processor_info.name,
            &state.report_global_processor_info.name_name,
            state.report_global_processor_info.vendor_id,
            &state.report_global_processor_info.vendor_id_name,
            state.report_global_processor_info.brand,
            &state.report_global_processor_info.brand_name,
        );

        let fut = state
            .addrs
            .send_and_wait({
                Measurement::new(
                    state.global_processor_info_name.to_string(),
                    measurement.clone(),
                )
            })
            .collect::<SendAllResult<Measurement>>();
        sending.push(fut);
    }

    if state.report_processor_info.enable {
        for processor in state.sys.processors().iter() {
            let measurement = get_processor_info_measurements(
                processor,
                state.report_processor_info.frequency,
                &state.report_processor_info.frequency_name,
                state.report_processor_info.cpu_usage,
                &state.report_processor_info.cpu_usage_name,
                state.report_processor_info.name,
                &state.report_processor_info.name_name,
                state.report_processor_info.vendor_id,
                &state.report_processor_info.vendor_id_name,
                state.report_processor_info.brand,
                &state.report_processor_info.brand_name,
            );

            let fut = state
                .addrs
                .send_and_wait({
                    Measurement::new(
                        state.global_processor_info_name.to_string(),
                        measurement.clone(),
                    )
                })
                .collect::<SendAllResult<Measurement>>();
            sending.push(fut)
        }
    }

    if state.report_physical_core_count.enable {
        if let Some(core_count) = state.sys.physical_core_count() {
            let measurement = MeasurementValue::Float(core_count as f64);
            let fut = state
                .addrs
                .send_and_wait({
                    Measurement::new(state.physical_core_count_name.to_string(), measurement)
                })
                .collect::<SendAllResult<Measurement>>();
            sending.push(fut);
        } else {
            // TODO cannot get core count
        }
    }

    let timeout_duration = std::time::Duration::from_millis(state.interval);

    sending
        .into_iter()
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<SendAllResult<Measurement>>>()
        .instrument(tracing::debug_span!(
            "plugin.sysstat.main-cpu.sending_measurements"
        ))
        .await
        .into_iter()
        .map(SendAllResult::into_result)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| PluginError::from(crate::error::Error::FailedToSendMeasurement))
        .map(|_| ())
}

fn get_processor_info_measurements(
    info: &sysinfo::Processor,
    frequency: bool,
    frequency_name: &str,
    cpu_usage: bool,
    cpu_usage_name: &str,
    name: bool,
    name_name: &str,
    vendor_id: bool,
    vendor_id_name: &str,
    brand: bool,
    brand_name: &str,
) -> MeasurementValue {
    use sysinfo::ProcessorExt;

    let mut aggregate = HashMap::new();

    if frequency {
        aggregate.insert(
            frequency_name.to_string(),
            MeasurementValue::Float(info.frequency() as f64),
        );
    }

    if cpu_usage {
        aggregate.insert(
            cpu_usage_name.to_string(),
            MeasurementValue::Float(info.cpu_usage().into()),
        );
    }

    if name {
        aggregate.insert(
            name_name.to_string(),
            MeasurementValue::Text(info.name().to_string()),
        );
    }

    if vendor_id {
        aggregate.insert(
            vendor_id_name.to_string(),
            MeasurementValue::Text(info.vendor_id().to_string()),
        );
    }

    if brand {
        aggregate.insert(
            brand_name.to_string(),
            MeasurementValue::Text(info.brand().to_string()),
        );
    }

    MeasurementValue::Map(aggregate)
}
