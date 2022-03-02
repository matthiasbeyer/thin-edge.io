use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;

use tedge_api::address::EndpointKind;
use tedge_api::message::MeasurementValue;
use tedge_api::Address;
use tedge_api::CoreCommunication;
use tedge_api::Message;
use tedge_api::MessageKind;
use tedge_api::Plugin;
use tedge_api::PluginBuilder;
use tedge_api::PluginConfiguration;
use tedge_api::PluginError;
use tedge_lib::mainloop::MainloopStopper;
use tracing::debug;
use tracing::trace;

mod config;
use config::*;

pub struct InotifyPluginBuilder;

#[async_trait]
impl PluginBuilder for InotifyPluginBuilder {
    fn kind_name(&self) -> &'static str {
        "inotify"
    }

    async fn verify_configuration(
        &self,
        config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        config
            .get_ref()
            .clone()
            .try_into()
            .map(|_: InotifyConfig| ())
            .map_err(|_| anyhow::anyhow!("Failed to parse inotify configuration"))
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: PluginConfiguration,
        comms: CoreCommunication,
    ) -> Result<Box<dyn Plugin>, PluginError> {
        let config = config
            .into_inner()
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to parse inotify configuration"))?;

        Ok(Box::new(InotifyPlugin::new(comms, config)))
    }
}

struct InotifyPlugin {
    comms: CoreCommunication,
    config: InotifyConfig,
    stopper: Option<MainloopStopper>,
}

impl InotifyPlugin {
    fn new(comms: CoreCommunication, config: InotifyConfig) -> Self {
        Self {
            comms,
            config,
            stopper: None,
        }
    }
}

struct State {
    target: Address,
    comms: CoreCommunication,
    fail_on_err: bool,
    inotify: inotify::Inotify,
    watches: HashMap<inotify::WatchDescriptor, PathBuf>,
}

#[async_trait]
impl Plugin for InotifyPlugin {
    async fn setup(&mut self) -> Result<(), PluginError> {
        let mut inotify = inotify::Inotify::init().map_err(anyhow::Error::from)?;

        let mut watches = HashMap::new();
        for (path, modes) in self.config.pathes.iter() {
            let mask = modes.iter().fold(inotify::WatchMask::empty(), |mask, el| {
                mask | inotify::WatchMask::from(*el)
            });

            let descriptor = inotify.add_watch(path, mask).map_err(anyhow::Error::from)?;
            watches.insert(descriptor, path.clone());
        }

        let state = State {
            target: Address::new(EndpointKind::Plugin {
                id: self.config.target.clone(),
            }),
            comms: self.comms.clone(),
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

    async fn handle_message(&self, _message: Message) -> Result<(), PluginError> {
        // ignore all messages
        trace!("Ignoring message");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        trace!("Shutdown");
        if let Some(stopper) = self.stopper.take() {
            stopper
                .stop()
                .map_err(|_| anyhow::anyhow!("Failed to stop mainloop"))?
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
        .map_err(anyhow::Error::from)?;

    loop {
        tokio::select! {
            next_event = stream.next() => {
                match next_event {
                    Some(Ok(event)) => {
                        debug!("Received inotify event = {:?}", event);
                        if let Some(path) = state.watches.get(&event.wd) {
                            let value = MeasurementValue::Str(path.display().to_string());
                            let measurement = MessageKind::Measurement {
                                name: mask_to_string(event.mask).to_string(),
                                value
                            };

                            state.comms
                                .send(measurement, state.target.clone())
                                .await?;
                        } else {
                            // what happened? Got a descriptor for a file that we don't watch?
                        }
                    },

                    Some(Err(err)) => {
                        debug!("Received inotify event = {:?}", err);
                        if state.fail_on_err {
                            return Err(PluginError::from(anyhow::Error::from(err)))
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
