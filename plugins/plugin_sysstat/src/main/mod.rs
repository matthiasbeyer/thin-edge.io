#![allow(unused)]

pub mod cpu;
pub mod disk_usage;
pub mod load;
pub mod memory;
pub mod network;
pub mod process;

use tedge_api::plugin::CoreCommunication;

use crate::config::SysStatConfig;

pub trait StateFromConfig: Sized {
    fn new_from_config(config: &SysStatConfig, comms: CoreCommunication) -> Option<Self>;
}

pub trait State {
    fn interval(&self) -> u64;
}
