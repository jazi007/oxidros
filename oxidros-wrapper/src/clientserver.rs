//! RPC a simplified version of oxidros client/server

use crate::common::Result;
use log::{debug, error, trace};
use oxidros::{
    msg::ServiceMsg,
    service::{client::Client as SdClient, server::Server as SdServer},
};
use std::{fmt::Debug, time::Duration};
use tokio::time;

/// RPC Client
#[allow(missing_debug_implementations)]
pub struct Client<T>(pub(crate) Option<SdClient<T>>);

impl<T: ServiceMsg> Client<T>
where
    <T as ServiceMsg>::Request: Debug,
    <T as ServiceMsg>::Response: Debug,
{
    /// Send a request
    pub async fn send(
        &mut self,
        data: &<T as ServiceMsg>::Request,
    ) -> Result<<T as ServiceMsg>::Response> {
        let client = self.0.take();
        let Some(client) = client else {
            return Err("Client not yet added".into());
        };
        debug!("Request: {:?}", data);
        let mut receiver = client.send(data)?.recv();
        // Send a request.
        let resp = match (&mut receiver).await {
            Ok((c, response, header)) => {
                trace!("Header: {header:?}");
                debug!("Response: {:?}", response);
                self.0 = Some(c);
                response
            }
            Err(e) => {
                self.0 = Some(receiver.give_up());
                return Err(e);
            }
        };
        Ok(resp)
    }
    /// Send a request with a timeout
    pub async fn send_timeout(
        &mut self,
        data: &<T as ServiceMsg>::Request,
        timeout: Duration,
    ) -> Result<<T as ServiceMsg>::Response> {
        match time::timeout(timeout, self.send(data)).await {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(e.into()),
        }
    }
}

/// RPC Server
#[allow(missing_debug_implementations)]
pub struct Server<T>(pub(crate) Option<SdServer<T>>);
pub(crate) type ServerCallback<T> =
    Box<dyn FnMut(<T as ServiceMsg>::Request) -> <T as ServiceMsg>::Response + Send>;

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
                Ok((sender, request, header)) => {
                    trace!("Header: {header:?}");
                    debug!("Request: {request:?}");
                    let response = callback(request);
                    debug!("Response: {response:?}");
                    match sender.send(&response) {
                        Ok(s) => server = s, // Get a new server to handle next request.
                        Err((s, e)) => {
                            error!("Failed to send response {:?}", e);
                            server = s.give_up();
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
