use std::{net::SocketAddr, sync::Arc};

use tokio::net::{TcpListener, ToSocketAddrs};

use crate::{
    ConnectionHandle, ConnectionLimits, Result, ServerConnectionConfig, ServerConnectionDriver,
};

pub struct ServerListener {
    listener: TcpListener,
}

impl ServerListener {
    pub async fn bind(address: impl ToSocketAddrs) -> Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(address).await?,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    pub async fn accept(
        &self,
        config: Arc<ServerConnectionConfig>,
        limits: ConnectionLimits,
    ) -> Result<(SocketAddr, ConnectionHandle, ServerConnectionDriver)> {
        let (stream, remote_address) = self.listener.accept().await?;
        let (handle, driver) = ServerConnectionDriver::new(stream, config, limits)?;
        Ok((remote_address, handle, driver))
    }
}
