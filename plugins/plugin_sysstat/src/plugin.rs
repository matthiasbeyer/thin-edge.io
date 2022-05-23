use std::sync::Arc;

use async_trait::async_trait;
use tedge_api::Plugin;
use tedge_api::PluginError;
use tedge_lib::address::AddressGroup;
use tedge_lib::measurement::Measurement;
use tokio::sync::Mutex;
use tracing::debug;
use tracing::trace;
use tracing::Instrument;

use crate::config::SysStatConfig;
use crate::main::State;
use crate::main::StateFromConfig;

#[derive(Debug)]
pub struct SysStatPlugin {
    config: SysStatConfig,
    addr_config: AddressConfig,
    stoppers: Vec<tedge_lib::mainloop::MainloopStopper>,
}

impl tedge_api::plugin::PluginDeclaration for SysStatPlugin {
    type HandledMessages = ();
}

tedge_api::make_receiver_bundle!(pub struct MeasurementReceiver(Measurement));

#[derive(Debug)]
pub struct AddressConfig {
    pub(crate) memory: Option<Arc<AddressGroup<MeasurementReceiver>>>,
    pub(crate) network: Option<Arc<AddressGroup<MeasurementReceiver>>>,
    pub(crate) cpu: Option<Arc<AddressGroup<MeasurementReceiver>>>,
    pub(crate) disk_usage: Option<Arc<AddressGroup<MeasurementReceiver>>>,
    pub(crate) load: Option<Arc<AddressGroup<MeasurementReceiver>>>,
    pub(crate) process: Option<Arc<AddressGroup<MeasurementReceiver>>>,
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
    #[tracing::instrument(name = "plugin.sysstat.start", skip(self))]
    async fn start(&mut self) -> Result<(), PluginError> {
        debug!("Starting sysstat plugin");
        macro_rules! run {
            ($t:ty, $sender:expr, $main:expr, $dbgspan:literal) => {
                if let Some(sender) = $sender.as_ref() {
                    if let Some(state) = <$t>::new_from_config(&self.config, sender.clone()) {
                        trace!(sysstat.backend = ?std::any::type_name::<$t>(), "Starting sysstat plugin with backend");
                        let duration = std::time::Duration::from_millis(state.interval());
                        let (stopper, mainloop) =
                            tedge_lib::mainloop::Mainloop::ticking_every(duration, Mutex::new(state));
                        self.stoppers.push(stopper);
                        let _ = tokio::spawn(mainloop.run($main).instrument(tracing::debug_span!($dbgspan)));
                    }
                }
            };
        }

        run!(
            crate::main::cpu::CPUState,
            self.addr_config.cpu,
            crate::main::cpu::main_cpu,
            "plugin.sysstat.main-cpu"
        );
        run!(
            crate::main::disk_usage::DiskUsageState,
            self.addr_config.disk_usage,
            crate::main::disk_usage::main_disk_usage,
            "plugin.sysstat.main-diskusage"
        );
        run!(
            crate::main::load::LoadState,
            self.addr_config.load,
            crate::main::load::main_load,
            "plugin.sysstat.main-load"
        );
        run!(
            crate::main::memory::MemoryState,
            self.addr_config.memory,
            crate::main::memory::main_memory,
            "plugin.sysstat.main-memory"
        );
        run!(
            crate::main::network::NetworkState,
            self.addr_config.network,
            crate::main::network::main_network,
            "plugin.sysstat.main-network"
        );
        run!(
            crate::main::process::ProcessState,
            self.addr_config.process,
            crate::main::process::main_process,
            "plugin.sysstat.main-process"
        );

        Ok(())
    }

    #[tracing::instrument(name = "plugin.sysstat.shutdown", skip(self))]
    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down sysstat plugin!");

        while let Some(stopper) = self.stoppers.pop() {
            stopper
                .stop()
                .map_err(|()| crate::error::Error::FailedToStopMainloop)?
        }

        Ok(())
    }
}
