use std::sync::Arc;

use async_trait::async_trait;
use criterion::{criterion_group, criterion_main, Criterion};
use criterion::{BatchSize, BenchmarkId};
use tedge_api::plugin::{Handle, HandleTypes};
use tedge_api::PluginConfiguration;
use tedge_api::PluginDirectory;
use tedge_api::PluginError;
use tedge_api::PluginExt;
use tedge_api::{make_receiver_bundle, PluginBuilder};
use tedge_api::{Address, Message, Plugin};
use tedge_core::TedgeApplication;
use tokio::sync::{Mutex, Notify};

#[derive(Debug)]
struct Measurement(u64);

impl Message for Measurement {}

pub struct ProducerPluginBuilder(Mutex<Option<tokio::sync::mpsc::Receiver<u64>>>);

#[async_trait::async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for ProducerPluginBuilder {
    fn kind_name() -> &'static str {
        "producer"
    }

    async fn verify_configuration(
        &self,
        _config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        Ok(())
    }

    async fn instantiate(
        &self,
        _config: PluginConfiguration,
        _cancellation_token: tedge_api::CancellationToken,
        plugin_dir: &PD,
    ) -> Result<tedge_api::plugin::BuiltPlugin, PluginError> {
        Ok(ProducerPlugin(
            self.0.lock().await.take(),
            plugin_dir.get_address_for("destination")?,
        )
        .finish())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        ProducerPlugin::get_handled_types()
    }
}

make_receiver_bundle!(struct MeasurementBundle(Measurement));

struct ProducerPlugin(
    Option<tokio::sync::mpsc::Receiver<u64>>,
    Address<MeasurementBundle>,
);

impl tedge_api::plugin::PluginDeclaration for ProducerPlugin {
    type HandledMessages = ();
}

#[async_trait]
impl Plugin for ProducerPlugin {
    #[allow(unreachable_code)]
    async fn main(&self) -> Result<(), PluginError> {
        let mut rec = self.0.take().unwrap();
        let addr = self.1.clone();
        let mut count = 0;
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    num = rec.recv() => {
                        if let Some(num) = num {
                            count += 1;
                            //println!("Sending msg #{}", count);
                            addr.send_and_wait(Measurement(num)).await
                                .unwrap_or_else(|_| {
                                    println!("Could not send in sender for msg num #{}", count);
                                    std::process::abort()
                                });
                        } else {
                            break
                        }
                    }
                }
            }
        });

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

pub struct ReceiverPluginBuilder(tokio::sync::mpsc::Sender<f64>);

#[async_trait::async_trait]
impl<PD: PluginDirectory> PluginBuilder<PD> for ReceiverPluginBuilder {
    fn kind_name() -> &'static str {
        "receiver"
    }

    async fn verify_configuration(
        &self,
        _config: &PluginConfiguration,
    ) -> Result<(), tedge_api::error::PluginError> {
        Ok(())
    }

    async fn instantiate(
        &self,
        _config: PluginConfiguration,
        _cancellation_token: tedge_api::CancellationToken,
        _plugin_dir: &PD,
    ) -> Result<tedge_api::plugin::BuiltPlugin, PluginError> {
        Ok(ReceiverPlugin(self.0.clone(), Mutex::new(vec![])).finish())
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        ReceiverPlugin::get_handled_types()
    }
}

struct ReceiverPlugin(tokio::sync::mpsc::Sender<f64>, Mutex<Vec<u64>>);

impl tedge_api::plugin::PluginDeclaration for ReceiverPlugin {
    type HandledMessages = (Measurement,);
}

#[async_trait]
impl Plugin for ReceiverPlugin {
    #[allow(unreachable_code)]
    async fn start(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

#[async_trait]
impl Handle<Measurement> for ReceiverPlugin {
    async fn handle_message(
        &self,
        message: Measurement,
        _sender: tedge_api::address::ReplySenderFor<Measurement>,
    ) -> Result<(), PluginError> {
        let mut vals = self.1.lock().await;
        vals.push(message.0);

        //println!("Received message, now containing #{}", vals.len());

        if vals.len() == 10 {
            self.0
                .send(vals.drain(..).sum::<u64>() as f64 / 10.0)
                .await
                .unwrap_or_else(|_| {
                    println!("Could not send in receiver");
                    std::process::abort()
                });
        }

        Ok(())
    }
}

async fn start_application(
    stopper: Arc<tokio::sync::Notify>,
    receiver: tokio::sync::mpsc::Receiver<u64>,
    sender: tokio::sync::mpsc::Sender<f64>,
) -> Result<(), Box<(dyn std::error::Error + Sync + Send + 'static)>> {
    let _ = tracing_subscriber::fmt::try_init();

    let config_file_path = {
        let dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join("../../../");
        let mut name = std::path::PathBuf::from(std::file!());
        name.set_extension("toml");
        let filepath = dir.join(name);
        assert!(
            filepath.exists(),
            "Config file does not exist: {}",
            filepath.display()
        );
        filepath
    };

    let (cancel_sender, application) = TedgeApplication::builder()
        .with_plugin_builder(ProducerPluginBuilder(Mutex::new(Some(receiver))))?
        .with_plugin_builder(ReceiverPluginBuilder(sender))?
        .with_config_from_path(config_file_path)
        .await?;

    let app = application.run();
    tokio::pin!(app);

    let mut cancelled = false;

    loop {
        tokio::select! {
            output = &mut app => {
                output.unwrap();
                break;
            }
            _ = stopper.notified(), if !cancelled => {
                cancel_sender.cancel_app();
                cancelled = true;
            }
        }
    }

    Ok(())
}

fn bench_throughput(c: &mut Criterion) {
    static KILO: u64 = 1000;

    let mut group = c.benchmark_group("throughput");

    for size in [
        KILO,
        10 * KILO,
        50 * KILO,
        100 * KILO,
        500 * KILO,
        1000 * KILO,
    ] {
        group.throughput(criterion::Throughput::Elements(size));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let notify = Arc::new(Notify::new());
            let (sender, receiver) = tokio::sync::mpsc::channel(100);
            let (fsender, freceiver) = tokio::sync::mpsc::channel(100);
            let app = rt.spawn(start_application(notify.clone(), receiver, fsender));

            let freceiver = Arc::new(Mutex::new(freceiver));

            b.to_async(&rt).iter_batched(
                || vec![123; size as usize],
                |data| {
                    let sender = sender.clone();
                    let freceiver = freceiver.clone();
                    async move {
                        let mut freceiver = freceiver.lock().await;
                        let mut count = 0;
                        let mut len = 0;
                        while len < data.len() {
                            tokio::select! {
                                res = sender.send(data[len]) => {
                                    len += 1;
                                    res.unwrap();
                                    //println!("Sent message!");
                                },
                                Some(_) = freceiver.recv() => {
                                    count += 1;
                                }
                            }
                        }
                        //println!("Done sending batch of {:?}, draining receiver", data.len());
                        while let Some(_) = freceiver.recv().await {
                            count += 1;
                            if count >= size / 10 {
                                break;
                            }
                        }
                    }
                },
                BatchSize::SmallInput,
            );

            //println!("Stopping app");
            notify.notify_one();

            //println!("Waiting for app to stop");
            rt.block_on(app).unwrap().unwrap();

            rt.shutdown_background();
        });
    }
}

criterion_group!(benches, bench_throughput);
criterion_main!(benches);
