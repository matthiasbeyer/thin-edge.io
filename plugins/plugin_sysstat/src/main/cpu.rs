use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use futures::FutureExt;
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
use tedge_lib::reply::ReplyableCoreCommunication;

use crate::config::HasBaseConfig;
use crate::config::PhysicalCoreCountConfig;
use crate::config::ProcessorInfoConfig;
use crate::main::State;
use crate::main::StateFromConfig;

pub struct CPUState {
    interval: u64,
    send_to: Vec<String>,
    sys: sysinfo::System,
    comms: CoreCommunication,

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
        comms: CoreCommunication,
    ) -> Option<Self> {
        config.cpu.as_ref().map(|config| CPUState {
            interval: config.interval_ms().get(),
            send_to: config.send_to().to_vec(),

            sys: sysinfo::System::new_with_specifics({ sysinfo::RefreshKind::new().with_cpu() }),

            comms,

            report_global_processor_info: config.report_global_processor_info.clone(),
            global_processor_info_name: config.global_processor_info_name.clone(),

            report_processor_info: config.report_processor_info.clone(),
            processor_info_name: config.processor_info_name.clone(),

            report_physical_core_count: config.report_physical_core_count.clone(),
            physical_core_count_name: config.physical_core_count_name.clone(),
        })
    }
}

pub async fn main_cpu(state: Arc<Mutex<CPUState>>) -> Result<(), PluginError> {
    let mut lock = state.lock().await;
    let mut state = lock.deref();

    let mut messages = Vec::new();

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

        for target in state.send_to.iter() {
            let addr = Address::new(EndpointKind::Plugin { id: target.clone() });
            let kind = MessageKind::Measurement {
                name: state.global_processor_info_name.to_string(),
                value: measurement.clone(),
            };

            messages.push((kind, addr));
        }
    }

    if state.report_processor_info.enable {
        let iter = state
            .sys
            .processors()
            .iter()
            .map(|processor| {
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

                state.send_to.iter().map(move |target| {
                    let addr = Address::new(EndpointKind::Plugin { id: target.clone() });
                    let kind = MessageKind::Measurement {
                        name: state.global_processor_info_name.to_string(),
                        value: measurement.clone(),
                    };

                    (kind, addr)
                })
            })
            .flatten();

        messages.extend(iter);
    }

    if state.report_physical_core_count.enable {
        if let Some(core_count) = state.sys.physical_core_count() {
            match core_count.try_into() {
                Err(_) => {
                    // TODO usize is bigger than u64
                    // not going to handle this for now
                }
                Ok(core_count) => {
                    let measurement = MeasurementValue::Int(core_count);
                    for target in state.send_to.iter() {
                        let addr = Address::new(EndpointKind::Plugin { id: target.clone() });
                        let kind = MessageKind::Measurement {
                            name: state.physical_core_count_name.to_string(),
                            value: measurement.clone(),
                        };

                        messages.push((kind, addr));
                    }
                }
            }
        } else {
            // TODO cannot get core count
        }
    }

    let timeout_duration = std::time::Duration::from_millis(state.interval);

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

    let mut aggregate = Vec::new();

    if frequency {
        aggregate.push((
            frequency_name.to_string(),
            MeasurementValue::Int(info.frequency()),
        ));
    }

    if cpu_usage {
        aggregate.push((
            cpu_usage_name.to_string(),
            MeasurementValue::Float(info.cpu_usage().into()),
        ));
    }

    if name {
        aggregate.push((
            name_name.to_string(),
            MeasurementValue::Str(info.name().to_string()),
        ));
    }

    if vendor_id {
        aggregate.push((
            vendor_id_name.to_string(),
            MeasurementValue::Str(info.vendor_id().to_string()),
        ));
    }

    if brand {
        aggregate.push((
            brand_name.to_string(),
            MeasurementValue::Str(info.brand().to_string()),
        ));
    }

    MeasurementValue::Aggregate(aggregate)
}
