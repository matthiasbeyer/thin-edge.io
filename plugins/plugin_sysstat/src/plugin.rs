use std::sync::Arc;

use async_trait::async_trait;
use tedge_api::Address;
use tedge_lib::measurement::Measurement;
use tokio::sync::Mutex;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tracing::debug;

use crate::config::SysStatConfig;
use crate::main::State;
use crate::main::StateFromConfig;

pub struct SysStatPlugin {
    config: SysStatConfig,
    addr_config: AddressConfig,
    stoppers: Vec<tedge_lib::mainloop::MainloopStopper>,
}

impl tedge_api::plugin::PluginDeclaration for SysStatPlugin {
    type HandledMessages = ();
}

tedge_api::make_receiver_bundle!(pub struct MeasurementReceiver(Measurement));

pub struct AddressConfig {
    pub(crate) memory: Arc<Vec<Address<MeasurementReceiver>>>,
    pub(crate) network: Arc<Vec<Address<MeasurementReceiver>>>,
    pub(crate) cpu: Arc<Vec<Address<MeasurementReceiver>>>,
    pub(crate) disk_usage: Arc<Vec<Address<MeasurementReceiver>>>,
    pub(crate) load: Arc<Vec<Address<MeasurementReceiver>>>,
    pub(crate) process: Arc<Vec<Address<MeasurementReceiver>>>,
}

impl SysStatPlugin {
    pub(crate) fn new(config: SysStatConfig, addr_config: AddressConfig) -> Self {
        Self {
            config,
            addr_config,
            stoppers: Vec::with_capacity(8), // We have 8 main loops in this crate right now
        }
    }
}

#[async_trait]
impl Plugin for SysStatPlugin {
    async fn start(&mut self) -> Result<(), PluginError> {
        macro_rules! run {
            ($t:ty, $sender:expr, $main:expr) => {
                if let Some(state) = <$t>::new_from_config(&self.config, $sender.clone()) {
                    let duration = std::time::Duration::from_millis(state.interval());
                    let (stopper, mainloop) =
                        tedge_lib::mainloop::Mainloop::ticking_every(duration, Mutex::new(state));
                    self.stoppers.push(stopper);
                    let _ = tokio::spawn(mainloop.run($main));
                }
            };
        }

        run!(
            crate::main::cpu::CPUState,
            self.addr_config.cpu,
            crate::main::cpu::main_cpu
        );
        run!(
            crate::main::disk_usage::DiskUsageState,
            self.addr_config.disk_usage,
            crate::main::disk_usage::main_disk_usage
        );
        run!(
            crate::main::load::LoadState,
            self.addr_config.load,
            crate::main::load::main_load
        );
        run!(
            crate::main::memory::MemoryState,
            self.addr_config.memory,
            crate::main::memory::main_memory
        );
        run!(
            crate::main::network::NetworkState,
            self.addr_config.network,
            crate::main::network::main_network
        );
        run!(
            crate::main::process::ProcessState,
            self.addr_config.process,
            crate::main::process::main_process
        );

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down sysstat plugin!");

        while let Some(stopper) = self.stoppers.pop() {
            stopper
                .stop()
                .map_err(|_| miette::miette!("Failed to stop mainloop"))?
        }

        Ok(())
    }
}
