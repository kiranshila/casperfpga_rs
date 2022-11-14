//! The casperfpga transport implementations for TAPCP

use super::Transport;
use crate::core::DeviceMap;
use anyhow::bail;
use std::{
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

const DEFAULT_TIMEOUT: f32 = 0.1;

#[derive(Debug)]
pub struct Tapcp {
    host: SocketAddr,
    connection: UdpSocket,
}

impl Tapcp {
    /// Create (but don't connect to) a TAPCP transport
    pub fn new(host: SocketAddr) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        // Set a default timeout
        let timeout = Duration::from_secs_f32(DEFAULT_TIMEOUT);
        socket.set_read_timeout(Some(timeout))?;
        Ok(Self {
            host,
            connection: socket,
        })
    }
}

// Transport trait implementations

impl Transport for Tapcp {
    fn connect(&mut self) -> anyhow::Result<()> {
        self.connection.connect(self.host)?;
        Ok(())
    }

    fn disconnect(&mut self) -> anyhow::Result<()> {
        todo!()
    }

    fn is_running(&mut self) -> anyhow::Result<bool> {
        // Check if sys_clkcounter exists
        match tapcp::read_device("sys_clkcounter", 0, 1, &mut self.connection) {
            Ok(_) => Ok(true),
            // In the case we get back a file not found error,
            // that implies the device is not running a user program.
            // Any other error is actually an error
            Err(e1) => match e1.downcast_ref::<tapcp::tftp::Error>() {
                Some(e2) => match e2 {
                    tapcp::tftp::Error::ErrorResponse(code, _) => match code {
                        tapcp::tftp::ErrorCode::NotFound => Ok(false),
                        _ => bail!(e1),
                    },
                    _ => bail!(e1),
                },
                None => bail!(e1),
            },
        }
    }

    fn write(&mut self, device: &str, offset: usize, data: &[u8]) -> anyhow::Result<()> {
        todo!()
    }

    fn listdev(&mut self) -> anyhow::Result<DeviceMap> {
        todo!()
    }

    fn program(&mut self, filename: &std::path::Path) -> anyhow::Result<()> {
        todo!()
    }

    fn deprogram(&mut self) -> anyhow::Result<()> {
        todo!()
    }

    fn read_vec(&mut self, device: &str, n: usize, offset: usize) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
}
