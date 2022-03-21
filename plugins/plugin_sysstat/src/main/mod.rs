#![allow(unused)]

pub mod cpu;
pub mod disk_usage;
pub mod load;
pub mod memory;
pub mod network;
pub mod process;

use std::sync::Arc;

use tedge_api::Address;

use crate::{config::SysStatConfig, plugin::MeasurementReceiver};

pub trait StateFromConfig: Sized {
    fn new_from_config(config: &SysStatConfig, addrs: Arc<Vec<Address<MeasurementReceiver>>>) -> Option<Self>;
}

pub trait State {
    fn interval(&self) -> u64;
}
