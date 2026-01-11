//! RPC a simplified version of oxidros client/server

use crate::common::Result;
use log::{debug, error, trace};
use oxidros::{
    prelude::{Message, ServiceMsg},
    service::{client::Client as SdClient, server::Server as SdServer},
};
use std::{fmt::Debug, time::Duration};
use tokio::time;

/// RPC Client
#[allow(missing_debug_implementations)]
pub struct Client<T: ServiceMsg>(pub(crate) SdClient<T>);

impl<T: ServiceMsg> Client<T>
where
    <T as ServiceMsg>::Request: Debug,
    <T as ServiceMsg>::Response: Debug,
{
    /// Send a request
    pub async fn send(
        &mut self,
        data: &<T as ServiceMsg>::Request,
    ) -> Result<Message<<T as ServiceMsg>::Response>> {
        let client = &mut self.0;
        debug!("Waiting for service availability");
        while !client.is_service_available() {
            time::sleep(Duration::from_millis(100)).await;
        }
        debug!("Request: {:?}", data);
        loop {
            let receiver = client.send(data)?.recv();
            // Send a request.
            match time::timeout(Duration::from_secs(1), receiver).await {
                Ok(Ok(response)) => {
                    trace!("Header: {:?}", response.info);
                    debug!("Response: {:?}", response.sample);
                    return Ok(response);
                }
                Ok(Err(e)) => {
                    return Err(e);
                }
                Err(_) => {
                    log::error!("Timeout retrying ...");
                }
            };
        }
    }
}

/// RPC Server
#[allow(missing_debug_implementations)]
pub struct Server<T>(pub(crate) Option<SdServer<T>>);
pub(crate) type ServerCallback<T> =
    Box<dyn FnMut(Message<<T as ServiceMsg>::Request>) -> <T as ServiceMsg>::Response + Send>;

impl<T: ServiceMsg> Server<T>
where
    <T as ServiceMsg>::Request: Debug,
    <T as ServiceMsg>::Response: Debug,
{
    /// Register a callback
    pub async fn register_callback(&mut self, mut callback: ServerCallback<T>) -> Result<()> {
        let server = self.0.take();
        let Some(mut server) = server else {
            return Err("Server not yet added".into());
        };
        loop {
            // Receive a request.
            let req = server.recv().await;
            match req {
                Ok(service_req) => {
                    let (sender, request) = service_req.split();
                    trace!("Header: {:?}", request.info);
                    debug!("Request: {:?}", request.sample);
                    let response = callback(request);
                    debug!("Response: {response:?}");
                    match sender.send(&response) {
                        Ok(()) => {} // Get a new server to handle next request.
                        Err(e) => {
                            error!("Failed to send response {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("error: {e}");
                    return Err(e);
                }
            }
        }
    }
    /// Get the inner Server
    pub fn into_inner(self) -> Option<SdServer<T>> {
        self.0
    }
}
