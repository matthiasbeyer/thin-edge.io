use std::sync::Arc;

use futures::FutureExt;
use tedge_api::{
    address::{MessageFutureProducer, ShouldWait},
    plugin::BuiltPlugin,
};
use tokio::sync::{mpsc, RwLock, Semaphore, TryAcquireError};
use tracing::{debug, debug_span, error, warn, Instrument, Span};

pub fn make_message_handler(
    name: String,
    sema: Arc<Semaphore>,
    built_plugin: Arc<RwLock<BuiltPlugin>>,
    panic_signal: mpsc::Sender<()>,
) -> Box<MessageFutureProducer> {
    Box::new(move |msg, should_wait| {
        let sema = sema.clone();
        let built_plugin = built_plugin.clone();
        let handle_span =
            debug_span!("plugin.handle_message", plugin.name = %name, ?msg).or_current();
        let panic_signal = panic_signal.clone();
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

            tokio::spawn(async move {
                let _permit = permit;
                let read_plug = built_plugin.read().await;
                let handled_message = std::panic::AssertUnwindSafe(read_plug.handle_message(msg))
                    .catch_unwind()
                    .instrument(Span::current())
                    .await;

                match handled_message {
                    Err(_panic) => {
                        error!("Message handling panicked");
                        let _ = panic_signal.send(()).await;
                    }
                    Ok(Ok(())) => debug!("Handled message succesfully"),
                    Ok(Err(error)) => warn!(%error, "Handling message failed"),
                }
            });
            Ok(())
        }
        .instrument(handle_span)
        .boxed()
    })
}
