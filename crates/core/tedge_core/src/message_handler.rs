use std::sync::Arc;

use futures::FutureExt;
use tedge_api::{
    address::{MessageFutureProducer, ShouldWait},
    plugin::BuiltPlugin,
};
use tokio::sync::{mpsc, RwLock, Semaphore, TryAcquireError};
use tracing::{debug_span, error, trace, warn, Instrument, Span};

pub fn make_message_handler(
    sema: Arc<Semaphore>,
    built_plugin: Arc<RwLock<BuiltPlugin>>,
    panic_signal: mpsc::Sender<()>,
) -> Box<MessageFutureProducer> {
    trace!("Registering message handler");
    Box::new(move |msg, should_wait| {
        let sema = sema.clone();
        let built_plugin = built_plugin.clone();
        let handle_span = debug_span!("plugin.handle_message", ?msg).or_current();
        let panic_signal = panic_signal.clone();
        trace!("Building another message handler");
        async move {
            let sema = sema;
            let permit = match should_wait {
                ShouldWait::Wait => match sema.acquire_owned().await {
                    Ok(permit) => permit,
                    Err(_acquire_err) => {
                        error!("Semaphore closed in CoreTask unexpectedly");
                        return Err(msg);
                    }
                },
                ShouldWait::DontWait => match sema.try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(TryAcquireError::NoPermits) => {
                        return Err(msg);
                    }

                    Err(TryAcquireError::Closed) => {
                        error!("Semaphore closed in CoreTask unexpectedly");
                        return Err(msg);
                    }
                },
                ShouldWait::Timeout(duration) => {
                    let elapsed = tokio::time::timeout(duration, async move {
                        match sema.acquire_owned().await {
                            Ok(permit) => Some(permit),
                            Err(_acquire_err) => {
                                error!("Semaphore closed in CoreTask unexpectedly");
                                return None;
                            }
                        }
                    })
                    .await;

                    match elapsed {
                        Err(_) | Ok(None) => return Err(msg),
                        Ok(Some(permit)) => permit,
                    }
                }
            };

            trace!("Spawning handler!");
            tokio::spawn(
                async move {
                    let _permit = permit;
                    let read_plug = built_plugin.read().await;
                    let handled_message =
                        std::panic::AssertUnwindSafe(read_plug.handle_message(msg))
                            .catch_unwind()
                            .instrument(Span::current())
                            .await;

                    match handled_message {
                        Err(panic) => {
                            let message: &str = {
                                if let Some(message) = panic.downcast_ref::<&'static str>() {
                                    message
                                } else if let Some(message) = panic.downcast_ref::<String>() {
                                    &*message
                                } else {
                                    "Unknown panic message"
                                }
                            };
                            error!(panic = %message, "Message handling panicked");
                            let _ = panic_signal.send(()).await;
                        }
                        Ok(Ok(())) => trace!("Handled message succesfully"),
                        Ok(Err(error)) => warn!(%error, "Handling message failed"),
                    }
                }
                .in_current_span(),
            );
            Ok(())
        }
        .instrument(handle_span)
        .boxed()
    })
}
