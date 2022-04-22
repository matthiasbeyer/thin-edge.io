use std::convert::Infallible;

use hyper::{Body, Request, Response, Server};
use tedge_api::{
    plugin::{HandleTypes, PluginExt},
    Address, CoreMessages, Plugin, PluginBuilder, PluginDirectory, PluginError,
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

#[derive(serde::Deserialize, Debug)]
struct HttpStopConfig {
    bind: std::net::SocketAddr,
}

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
enum Error {
    #[error("Failed to parse configuration")]
    ConfigParseFailed(#[from] toml::de::Error),

    #[error("HTTP Server stop failed")]
    FailedToStopMainloop(#[from] tokio::task::JoinError),
}

pub struct HttpStopPluginBuilder;

#[async_trait::async_trait]
impl<PD> PluginBuilder<PD> for HttpStopPluginBuilder
where
    PD: PluginDirectory,
{
    fn kind_name() -> &'static str
    where
        Self: Sized,
    {
        "httpstop"
    }

    fn kind_message_types() -> HandleTypes
    where
        Self: Sized,
    {
        HttpStopPlugin::get_handled_types()
    }

    async fn verify_configuration(
        &self,
        config: &tedge_api::PluginConfiguration,
    ) -> Result<(), tedge_api::PluginError> {
        debug!("Verifying HttpStopPlugin configuration");
        config
            .clone()
            .try_into::<HttpStopConfig>()
            .map(|_| ())
            .map_err(Error::from)
            .map_err(PluginError::from)
    }

    async fn instantiate(
        &self,
        config: tedge_api::PluginConfiguration,
        cancellation_token: tokio_util::sync::CancellationToken,
        plugin_dir: &PD,
    ) -> Result<tedge_api::plugin::BuiltPlugin, tedge_api::PluginError> {
        debug!("Instantiating HttpStopPlugin");
        let config = config
            .clone()
            .try_into::<HttpStopConfig>()
            .map_err(Error::from)?;

        let plugin = HttpStopPlugin {
            cancellation_token,
            bind: config.bind,
            core: plugin_dir.get_address_for_core(),

            join_handle: None,
        };

        Ok(plugin.finish())
    }
}

pub struct HttpStopPlugin {
    cancellation_token: CancellationToken,
    bind: std::net::SocketAddr,
    core: Address<CoreMessages>,

    join_handle: Option<JoinHandle<Result<(), hyper::Error>>>,
}

impl tedge_api::plugin::PluginDeclaration for HttpStopPlugin {
    type HandledMessages = ();
}

#[async_trait::async_trait]
impl Plugin for HttpStopPlugin {
    async fn start(&mut self) -> Result<(), PluginError> {
        debug!("Setting up HttpStopPlugin");
        let addr = self.core.clone();
        let svc = hyper::service::make_service_fn(move |_conn| {
            let addr = addr.clone();
            let service = hyper::service::service_fn(move |req| request_handler(addr.clone(), req));

            async move { Ok::<_, Infallible>(service) }
        });

        let cancellation_token = self.cancellation_token.clone();
        let serv = Server::bind(&self.bind)
            .serve(svc)
            .with_graceful_shutdown(async move {
                cancellation_token.cancelled().await;
            });

        self.join_handle = Some(tokio::spawn(serv));
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        debug!("Shutting down HttpStopPlugin");
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.await.map_err(Error::FailedToStopMainloop)?;
        }
        Ok(())
    }
}

async fn request_handler(
    addr: Address<CoreMessages>,
    _: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    debug!("Received request, stopping thin-edge now.");
    let _ = addr.send_and_wait(tedge_api::message::StopCore).await;
    Ok(Response::new("shutdown initiated".into()))
}
