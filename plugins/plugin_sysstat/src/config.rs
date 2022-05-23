use std::collections::HashMap;

use tedge_lib::config::Address;
use tedge_lib::config::OneOrMany;

#[derive(serde::Deserialize, Debug, tedge_api::Config)]
pub struct SysStatConfig {
    pub(crate) memory: Option<MemoryConfig>,
    pub(crate) network: Option<NetworkConfig>,
    pub(crate) cpu: Option<CpuConfig>,
    pub(crate) disk_usage: Option<DiskUsageConfig>,
    pub(crate) load: Option<LoadConfig>,
    pub(crate) process: Option<ProcessConfig>,
}

#[derive(serde::Deserialize, Debug, tedge_api::Config)]
pub struct BaseConfig {
    send_to: OneOrMany<Address>,
    interval_ms: std::num::NonZeroU64,
}

pub trait HasBaseConfig {
    fn send_to(&self) -> &OneOrMany<Address>;
    fn interval_ms(&self) -> std::num::NonZeroU64;
}

#[derive(serde::Deserialize, Debug, tedge_api::Config)]
pub struct MemoryConfig {
    #[serde(flatten)]
    base: BaseConfig,

    /// Whether to report total_memory measurements
    pub(crate) total_memory: bool,

    /// The name under which the total_memory_name measurement is reported
    ///
    /// default: "total_memory_name"
    ///
    /// ```
    /// assert_eq!("total_memory_name", total_memory_name_default());
    /// ```
    #[serde(default = "total_memory_name_default")]
    pub(crate) total_memory_name: String,

    /// Whether to report free_memory measurements
    pub(crate) free_memory: bool,

    /// The name under which the free_memory_name measurement is reported
    ///
    /// default: "free_memory_name"
    ///
    /// ```
    /// assert_eq!("free_memory_name", free_memory_name_default());
    /// ```
    #[serde(default = "free_memory_name_default")]
    pub(crate) free_memory_name: String,

    /// Whether to report available_memory measurements
    pub(crate) available_memory: bool,

    /// The name under which the available_memory_name measurement is reported
    ///
    /// default: "available_memory_name"
    ///
    /// ```
    /// assert_eq!("available_memory_name", available_memory_name_default());
    /// ```
    #[serde(default = "available_memory_name_default")]
    pub(crate) available_memory_name: String,

    /// Whether to report used_memory measurements
    pub(crate) used_memory: bool,

    /// The name under which the used_memory_name measurement is reported
    ///
    /// default: "used_memory_name"
    ///
    /// ```
    /// assert_eq!("used_memory_name", used_memory_name_default());
    /// ```
    #[serde(default = "used_memory_name_default")]
    pub(crate) used_memory_name: String,

    /// Whether to report free_swap measurements
    pub(crate) free_swap: bool,

    /// The name under which the free_swap_name measurement is reported
    ///
    /// default: "free_swap_name"
    ///
    /// ```
    /// assert_eq!("free_swap_name", free_swap_name_default());
    /// ```
    #[serde(default = "free_swap_name_default")]
    pub(crate) free_swap_name: String,

    /// Whether to report used_swap measurements
    pub(crate) used_swap: bool,

    /// The name under which the used_swap_name measurement is reported
    ///
    /// default: "used_swap_name"
    ///
    /// ```
    /// assert_eq!("used_swap_name", used_swap_name_default());
    /// ```
    #[serde(default = "used_swap_name_default")]
    pub(crate) used_swap_name: String,
}

fn total_memory_name_default() -> String {
    "total_memory".to_string()
}

fn free_memory_name_default() -> String {
    "free_memory".to_string()
}

fn available_memory_name_default() -> String {
    "available_memory".to_string()
}

fn used_memory_name_default() -> String {
    "used_memory".to_string()
}

fn free_swap_name_default() -> String {
    "free_swap".to_string()
}

fn used_swap_name_default() -> String {
    "used_swap".to_string()
}

impl HasBaseConfig for MemoryConfig {
    fn send_to(&self) -> &OneOrMany<Address> {
        &self.base.send_to
    }

    fn interval_ms(&self) -> std::num::NonZeroU64 {
        self.base.interval_ms
    }
}

#[derive(serde::Deserialize, Debug, tedge_api::Config)]
pub struct NetworkConfig {
    #[serde(flatten)]
    base: BaseConfig,

    /// Configure this to report stats for all network interfaces
    pub(crate) all_networks: AllNetworkStatConfig,

    /// Configure this to report stats for network interfaces by name
    pub(crate) by_name: HashMap<String, NetworkStatConfig>,
}

#[derive(serde::Deserialize, Clone, Debug, tedge_api::Config)]
pub struct AllNetworkStatConfig {
    pub(crate) enable: bool,

    #[serde(flatten)]
    pub(crate) config: NetworkStatConfig,
}

#[derive(serde::Deserialize, Clone, Debug, tedge_api::Config)]
pub struct NetworkStatConfig {
    /// Enable to report the "received" measurement
    pub(crate) received: bool,

    /// Name to use to report the "received" measurement
    ///
    /// default: "received"
    ///
    /// ```
    /// assert_eq!("received", received_name_default());
    /// ```
    #[serde(default = "received_name_default")]
    pub(crate) received_name: String,

    /// Enable to report the "total_received" measurement
    pub(crate) total_received: bool,

    /// Name to use to report the "total_received" measurement
    ///
    ///
    /// default: "total_received"
    ///
    /// ```
    /// assert_eq!("total_received", total_received_name_default());
    /// ```
    #[serde(default = "total_received_name_default")]
    pub(crate) total_received_name: String,

    /// Enable to report the "transmitted" measurement
    pub(crate) transmitted: bool,

    /// Name to use to report the "transmitted" measurement
    ///
    ///
    /// default: "transmitted"
    ///
    /// ```
    /// assert_eq!("transmitted", transmitted_name_default());
    /// ```
    #[serde(default = "transmitted_name_default")]
    pub(crate) transmitted_name: String,

    /// Enable to report the "total_transmitted" measurement
    pub(crate) total_transmitted: bool,

    /// Name to use to report the "total_transmitted" measurement
    ///
    ///
    /// default: "total_transmitted"
    ///
    /// ```
    /// assert_eq!("total_transmitted", total_transmitted_name_default());
    /// ```
    #[serde(default = "total_transmitted_name_default")]
    pub(crate) total_transmitted_name: String,

    /// Enable to report the "packets_received" measurement
    pub(crate) packets_received: bool,

    /// Name to use to report the "packets_received" measurement
    ///
    ///
    /// default: "packets_received"
    ///
    /// ```
    /// assert_eq!("packets_received", packets_received_name_default());
    /// ```
    #[serde(default = "packets_received_name_default")]
    pub(crate) packets_received_name: String,

    /// Enable to report the "total_packets_received" measurement
    pub(crate) total_packets_received: bool,

    /// Name to use to report the "total_packets_received" measurement
    ///
    ///
    /// default: "total_packets_received"
    ///
    /// ```
    /// assert_eq!("total_packets_received", total_packets_received_name_default());
    /// ```
    #[serde(default = "total_packets_received_name_default")]
    pub(crate) total_packets_received_name: String,

    /// Enable to report the "packets_transmitted" measurement
    pub(crate) packets_transmitted: bool,

    /// Name to use to report the "packets_transmitted" measurement
    ///
    ///
    /// default: "packets_transmitted"
    ///
    /// ```
    /// assert_eq!("packets_transmitted", packets_transmitted_name_default());
    /// ```
    #[serde(default = "packets_transmitted_name_default")]
    pub(crate) packets_transmitted_name: String,

    /// Enable to report the "total_packets_transmitted" measurement
    pub(crate) total_packets_transmitted: bool,

    /// Name to use to report the "total_packets_transmitted" measurement
    ///
    ///
    /// default: "total_packets_transmitted"
    ///
    /// ```
    /// assert_eq!("total_packets_transmitted", total_packets_transmitted_name_default());
    /// ```
    #[serde(default = "total_packets_transmitted_name_default")]
    pub(crate) total_packets_transmitted_name: String,

    /// Enable to report the "errors_on_received" measurement
    pub(crate) errors_on_received: bool,

    /// Name to use to report the "errors_on_received" measurement
    ///
    ///
    /// default: "errors_on_received"
    ///
    /// ```
    /// assert_eq!("errors_on_received", errors_on_received_name_default());
    /// ```
    #[serde(default = "errors_on_received_name_default")]
    pub(crate) errors_on_received_name: String,

    /// Enable to report the "total_errors_on_received" measurement
    pub(crate) total_errors_on_received: bool,

    /// Name to use to report the "total_errors_on_received" measurement
    ///
    ///
    /// default: "total_errors_on_received"
    ///
    /// ```
    /// assert_eq!("total_errors_on_received", total_errors_on_received_name_default());
    /// ```
    #[serde(default = "total_errors_on_received_name_default")]
    pub(crate) total_errors_on_received_name: String,
}

fn received_name_default() -> String {
    "received".to_string()
}

fn total_received_name_default() -> String {
    "total_received".to_string()
}

fn transmitted_name_default() -> String {
    "transmitted".to_string()
}

fn total_transmitted_name_default() -> String {
    "total_transmitted".to_string()
}

fn packets_received_name_default() -> String {
    "packets_received".to_string()
}

fn total_packets_received_name_default() -> String {
    "total_packets_received".to_string()
}

fn packets_transmitted_name_default() -> String {
    "packets_transmitted".to_string()
}

fn total_packets_transmitted_name_default() -> String {
    "total_packets_transmitted".to_string()
}

fn errors_on_received_name_default() -> String {
    "errors_on_received".to_string()
}

fn total_errors_on_received_name_default() -> String {
    "total_errors_on_received".to_string()
}

impl HasBaseConfig for NetworkConfig {
    fn send_to(&self) -> &OneOrMany<Address> {
        &self.base.send_to
    }

    fn interval_ms(&self) -> std::num::NonZeroU64 {
        self.base.interval_ms
    }
}

#[derive(serde::Deserialize, Debug, tedge_api::Config)]
pub struct CpuConfig {
    #[serde(flatten)]
    base: BaseConfig,

    pub(crate) report_global_processor_info: ProcessorInfoConfig,

    /// Report the global processor info measurement with the following name
    ///
    /// default: "global_processor_info"
    ///
    /// ```
    /// assert!("global_processor_info", global_processor_info_name_default())
    /// ```
    #[serde(default = "global_processor_info_name_default")]
    pub(crate) global_processor_info_name: String,

    /// Report the processor info measurement with the following name
    ///
    /// default: "processor_info"
    ///
    /// ```
    /// assert!("processor_info", processor_info_name_default())
    /// ```
    pub(crate) report_processor_info: ProcessorInfoConfig,

    #[serde(default = "processor_info_name_default")]
    pub(crate) processor_info_name: String,

    /// Report the physical core count measurement with the following name
    ///
    /// default: "physical_core_count"
    ///
    /// ```
    /// assert!("physical_core_count", physical_core_count_name_default())
    /// ```
    pub(crate) report_physical_core_count: PhysicalCoreCountConfig,

    #[serde(default = "physical_core_count_name_default")]
    pub(crate) physical_core_count_name: String,
}

fn global_processor_info_name_default() -> String {
    "global_processor_info".to_string()
}

fn processor_info_name_default() -> String {
    "processor_info".to_string()
}

fn physical_core_count_name_default() -> String {
    "physical_core_count".to_string()
}

#[derive(serde::Deserialize, Clone, Debug, tedge_api::Config)]
pub struct ProcessorInfoConfig {
    /// Enable reporting this stat
    pub(crate) enable: bool,

    /// Report the processor frequency
    pub(crate) frequency: bool,

    /// Report the processor frequency measurement with this name
    /// default: "frequency"
    ///
    /// ```
    /// assert_eq!("frequency", frequency_default_name());
    /// ```
    #[serde(default = "frequency_default_name")]
    pub(crate) frequency_name: String,

    /// Report the processor cpu usage
    pub(crate) cpu_usage: bool,

    /// Report the processor cpu usage measurement with this name
    /// default: "cpu_usage"
    ///
    /// ```
    /// assert_eq!("cpu_usage", cpu_usage_default_name());
    /// ```
    #[serde(default = "cpu_usage_default_name")]
    pub(crate) cpu_usage_name: String,

    /// Report the processor name
    pub(crate) name: bool,

    /// Report the processor name measurement with this name
    /// default: "name"
    ///
    /// ```
    /// assert_eq!("name", name_default_name());
    /// ```
    #[serde(default = "name_default_name")]
    pub(crate) name_name: String,

    /// Report the processor vendor id
    pub(crate) vendor_id: bool,

    /// Report the processor vendor id measurement with this name
    /// default: "vendor_id"
    ///
    /// ```
    /// assert_eq!("vendor_id", vendor_id_default_name());
    /// ```
    #[serde(default = "vendor_id_default_name")]
    pub(crate) vendor_id_name: String,

    /// Report the processor brand
    pub(crate) brand: bool,

    /// Report the processor brand measurement with this name
    /// default: "brand"
    ///
    /// ```
    /// assert_eq!("brand", brand_default_name());
    /// ```
    #[serde(default = "brand_default_name")]
    pub(crate) brand_name: String,
}

fn frequency_default_name() -> String {
    "frequency".to_string()
}

fn cpu_usage_default_name() -> String {
    "cpu_usage".to_string()
}

fn name_default_name() -> String {
    "name".to_string()
}

fn vendor_id_default_name() -> String {
    "vendor_id".to_string()
}

fn brand_default_name() -> String {
    "brand".to_string()
}

#[derive(serde::Deserialize, Clone, Debug, tedge_api::Config)]
pub struct PhysicalCoreCountConfig {
    pub(crate) enable: bool,
}

impl HasBaseConfig for CpuConfig {
    fn send_to(&self) -> &OneOrMany<Address> {
        &self.base.send_to
    }

    fn interval_ms(&self) -> std::num::NonZeroU64 {
        self.base.interval_ms
    }
}

#[derive(serde::Deserialize, Debug, tedge_api::Config)]
pub struct DiskUsageConfig {
    #[serde(flatten)]
    base: BaseConfig,
}

impl HasBaseConfig for DiskUsageConfig {
    fn send_to(&self) -> &OneOrMany<Address> {
        &self.base.send_to
    }

    fn interval_ms(&self) -> std::num::NonZeroU64 {
        self.base.interval_ms
    }
}

#[derive(serde::Deserialize, Debug, tedge_api::Config)]
pub struct LoadConfig {
    #[serde(flatten)]
    base: BaseConfig,
}

impl HasBaseConfig for LoadConfig {
    fn send_to(&self) -> &OneOrMany<Address> {
        &self.base.send_to
    }

    fn interval_ms(&self) -> std::num::NonZeroU64 {
        self.base.interval_ms
    }
}

#[derive(serde::Deserialize, Debug, tedge_api::Config)]
pub struct ProcessConfig {
    #[serde(flatten)]
    base: BaseConfig,

    pub(crate) all_processes: AllProcessConfig,
    pub(crate) by_name: HashMap<String, ProcessStatConfig>,
}

#[derive(serde::Deserialize, Clone, Debug, tedge_api::Config)]
pub struct AllProcessConfig {
    /// Report stats for all processes
    /// This automatically disables the "by_name" process reporting
    pub(crate) enable: bool,

    #[serde(flatten)]
    pub(crate) config: ProcessStatConfig,
}

#[derive(serde::Deserialize, Clone, Debug, tedge_api::Config)]
pub struct ProcessStatConfig {
    pub(crate) cmd: bool,
    pub(crate) cwd: bool,
    pub(crate) cpu_usage: bool,
    pub(crate) disk_usage: bool,

    // Currently not supported because we do not yet have a way to report a list of values
    // (without naming each)
    // pub(crate) environ: bool,
    pub(crate) exe: bool,
    pub(crate) memory: bool,
    pub(crate) name: bool,
    pub(crate) parent: bool,
    pub(crate) pid: bool,
    pub(crate) root: bool,
    pub(crate) run_time: bool,
    pub(crate) start_time: bool,
    pub(crate) vmemory: bool,
}

impl HasBaseConfig for ProcessConfig {
    fn send_to(&self) -> &OneOrMany<Address> {
        &self.base.send_to
    }

    fn interval_ms(&self) -> std::num::NonZeroU64 {
        self.base.interval_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_embedded_base() {
        let s = r#"
            [memory]
            send_to = []
            interval_ms = 100
        "#;

        let _: SysStatConfig = toml::from_str(s).unwrap();
    }
}
