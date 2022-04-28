use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use miette::IntoDiagnostic;
use tokio_util::sync::CancellationToken;

use tedge_api::plugin::BuiltPlugin;
use tedge_api::plugin::HandleTypes;
use tedge_api::plugin::PluginExt;
use tedge_api::Address;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_lib::mainloop::MainloopStopper;
use tedge_lib::measurement::Measurement;
use tedge_lib::measurement::MeasurementValue;
use tracing::debug;
use tracing::trace;

mod config;
use config::*;

pub struct InotifyPluginBuilder;

tedge_api::make_receiver_bundle!(pub struct MeasurementReceiver(Measurement));

#[async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for InotifyPluginBuilder {
    fn kind_name() -> &'static str {
        "inotify"
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        InotifyPlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .clone()
            .try_into()
            .map(|_: InotifyConfig| ())
            .map_err(|_| miette::miette!("Failed to parse inotify configuration"))
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        _cancellation_token: CancellationToken,
        plugin_dir: &PD,
    ) -> Result<BuiltPlugin, PluginError> {
        let config = config
            .try_into::<InotifyConfig>()
            .map_err(|_| miette::miette!("Failed to parse inotify configuration"))?;

        let addr = plugin_dir.get_address_for(&config.target)?;
        Ok(InotifyPlugin::new(addr, config).finish())
    }
}

struct InotifyPlugin {
    addr: Address<MeasurementReceiver>,
    config: InotifyConfig,
    stopper: Option<MainloopStopper>,
}

impl tedge_api::plugin::PluginDeclaration for InotifyPlugin {
    type HandledMessages = ();
}

impl InotifyPlugin {
    fn new(addr: Address<MeasurementReceiver>, config: InotifyConfig) -> Self {
        Self {
            addr,
            config,
            stopper: None,
        }
    }
}

struct State {
    addr: Address<MeasurementReceiver>,
    fail_on_err: bool,
    inotify: inotify::Inotify,
    watches: HashMap<inotify::WatchDescriptor, PathBuf>,
}

#[async_trait]
impl Plugin for InotifyPlugin {
    async fn start(&mut self) -> Result<(), PluginError> {
        let mut inotify = inotify::Inotify::init().into_diagnostic()?;

        let mut watches = HashMap::new();
        for (path, modes) in self.config.pathes.iter() {
            let mask = modes.iter().fold(inotify::WatchMask::empty(), |mask, el| {
                mask | inotify::WatchMask::from(*el)
            });

            let descriptor = inotify.add_watch(path, mask).into_diagnostic()?;
            watches.insert(descriptor, path.clone());
        }

        let state = State {
            addr: self.addr.clone(),
            fail_on_err: self.config.fail_on_err,
            watches,
            inotify,
        };

        let (stopper, mainloop) = tedge_lib::mainloop::Mainloop::detach(state);
        self.stopper = Some(stopper);

        let _ = tokio::spawn(mainloop.run(main_inotify));
        trace!("Mainloop spawned");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        trace!("Shutdown");
        if let Some(stopper) = self.stopper.take() {
            stopper
                .stop()
                .map_err(|_| miette::miette!("Failed to stop mainloop"))?
        }
        Ok(())
    }
}

async fn main_inotify(
    mut state: State,
    mut stopper: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), PluginError> {
    use futures::stream::StreamExt;

    let mut stream = state
        .inotify
        .event_stream(Vec::from([0; 1024]))
        .into_diagnostic()?;

    loop {
        tokio::select! {
            next_event = stream.next() => {
                match next_event {
                    Some(Ok(event)) => {
                        debug!("Received inotify event = {:?}", event);
                        if let Some(path) = state.watches.get(&event.wd) {
                            let value = MeasurementValue::Text(path.display().to_string());
                            let measurement = Measurement::new(mask_to_string(event.mask).to_string(), value);

                            let _ = state.addr.send_and_wait(measurement).await;
                        } else {
                            // what happened? Got a descriptor for a file that we don't watch?
                        }
                    },

                    Some(Err(err)) => {
                        debug!("Received inotify event = {:?}", err);
                        if state.fail_on_err {
                            return Err(err).into_diagnostic()
                        }
                    },

                    None => break, // according to inotify doc, this will never happen
                }
            },

            _ = &mut stopper => {
                debug!("Stopping main loop");
                break;
            },
        }
    }

    Ok(())
}

/// Transform an EventMask to a String
///
/// MUST only be called with one event type
fn mask_to_string(mask: inotify::EventMask) -> &'static str {
    match mask {
        inotify::EventMask::ACCESS => "ACCESS",
        inotify::EventMask::ATTRIB => "ATTRIB",
        inotify::EventMask::CLOSE_WRITE => "CLOSE_WRITE",
        inotify::EventMask::CLOSE_NOWRITE => "CLOSE_NOWRITE",
        inotify::EventMask::CREATE => "CREATE",
        inotify::EventMask::DELETE => "DELETE",
        inotify::EventMask::DELETE_SELF => "DELETE_SELF",
        inotify::EventMask::MODIFY => "MODIFY",
        inotify::EventMask::MOVE_SELF => "MOVE_SELF",
        inotify::EventMask::MOVED_FROM => "MOVED_FROM",
        inotify::EventMask::MOVED_TO => "MOVED_TO",
        inotify::EventMask::OPEN => "OPEN",
        inotify::EventMask::IGNORED => "IGNORED",
        inotify::EventMask::ISDIR => "ISDIR",
        inotify::EventMask::Q_OVERFLOW => "Q_OVERFLOW",
        inotify::EventMask::UNMOUNT => "UNMOUNT",
        _ => "unknown",
    }
}
