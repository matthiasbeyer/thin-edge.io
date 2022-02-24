use async_trait::async_trait;
use tokio::sync::Mutex;

use tedge_api::Message;
use tedge_api::Plugin;
use tedge_api::PluginError;

use crate::config::SysStatConfig;
use crate::main::State;
use crate::main::StateFromConfig;

pub struct SysStatPlugin {
    comms: tedge_api::plugin::CoreCommunication,
    config: SysStatConfig,
    stoppers: Vec<tedge_lib::mainloop::MainloopStopper>,
}

impl SysStatPlugin {
    pub(crate) fn new(comms: tedge_api::plugin::CoreCommunication, config: SysStatConfig) -> Self {
        Self {
            comms,
            config,
            stoppers: Vec::with_capacity(8), // We have 8 main loops in this crate right now
        }
    }
}

#[async_trait]
impl Plugin for SysStatPlugin {
    async fn setup(&mut self) -> Result<(), PluginError> {
        macro_rules! run {
            ($t:ty, $main:expr) => {
                if let Some(state) = <$t>::new_from_config(&self.config, self.comms.clone()) {
                    let duration = std::time::Duration::from_millis(state.interval());
                    let (stopper, mainloop) =
                        tedge_lib::mainloop::Mainloop::ticking_every(duration, Mutex::new(state));
                    self.stoppers.push(stopper);
                    let _ = tokio::spawn(mainloop.run($main));
                }
            };
        }

        run!(crate::main::cpu::CPUState, crate::main::cpu::main_cpu);
        run!(
            crate::main::disk_usage::DiskUsageState,
            crate::main::disk_usage::main_disk_usage
        );
        run!(crate::main::load::LoadState, crate::main::load::main_load);
        run!(
            crate::main::memory::MemoryState,
            crate::main::memory::main_memory
        );
        run!(
            crate::main::network::NetworkState,
            crate::main::network::main_network
        );
        run!(
            crate::main::process::ProcessState,
            crate::main::process::main_process
        );

        Ok(())
    }

    async fn handle_message(&self, _message: Message) -> Result<(), PluginError> {
        // Ignoring all messages
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        log::debug!("Shutting down sysstat plugin!");

        while let Some(stopper) = self.stoppers.pop() {
            stopper
                .stop()
                .map_err(|_| anyhow::anyhow!("Failed to stop mainloop"))?
        }

        Ok(())
    }
}
