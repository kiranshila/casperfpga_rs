//! The casperfpga transport implementations for TAPCP

use super::Transport;
use crate::core::{Device, DeviceMap};
use anyhow::bail;
use std::{
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

const DEFAULT_TIMEOUT: f32 = 0.1;

#[derive(Debug)]
/// A TAPCP Connection (newtype for a UdpSocket)
pub struct Tapcp(UdpSocket);

impl Tapcp {
    /// Create and connect to a TAPCP transport
    pub fn connect(host: SocketAddr) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        // Set a default timeout
        let timeout = Duration::from_secs_f32(DEFAULT_TIMEOUT);
        socket.set_read_timeout(Some(timeout))?;
        // Connect
        socket.connect(host)?;
        // And return
        Ok(Self(socket))
    }
}

// Transport trait implementations

impl Transport for Tapcp {
    fn is_running(&mut self) -> anyhow::Result<bool> {
        // Check if sys_clkcounter exists
        match tapcp::read_device("sys_clkcounter", 0, 1, &mut self.0) {
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

    fn write_bytes(&mut self, device: &str, offset: usize, data: &[u8]) -> anyhow::Result<()> {
        // The inverted version of `read_vec`. The problem here is if we are not writing a 4 byte
        // chunk (which we need to), we have to read the bytes that are already there and include them.
        // Because we don't want to do this read when we don't have to, we will branch
        if (offset % 4) == 0 && (data.len() % 4) == 0 {
            // Just do the write
            tapcp::write_device(device, offset % 4, data, &mut self.0)?;
        } else {
            todo!()
        }
        Ok(())
    }

    fn listdev(&mut self) -> anyhow::Result<DeviceMap> {
        let devices = tapcp::listdev(&mut self.0)?;
        Ok(devices
            .iter()
            .map(|(k, (addr, len))| {
                (
                    k.clone(),
                    Device {
                        addr: *addr as usize,
                        length: *len as usize,
                    },
                )
            })
            .collect())
    }

    fn program(&mut self, _filename: &std::path::Path) -> anyhow::Result<()> {
        todo!()
    }

    fn deprogram(&mut self) -> anyhow::Result<()> {
        todo!()
    }

    fn read_bytes<const N: usize>(
        &mut self,
        device: &str,
        offset: usize,
    ) -> anyhow::Result<[u8; N]> {
        // TAPCP works on a block of size 4 bytes, so we need to do some chunking and slicing
        // The goal here is to be efficient, we don't want to query bytes we don't need.
        // The "worst case" is when we want to read bytes between words
        // i.e. If the device contains [1,2,3,4,5,6,7,8] and we want to read offset=2, N=3
        // Which is the last 2 bytes of the first word and the first byte of the second word.
        // In that case, we need to read both words.
        // First, grab enough multiple of 4 bytes
        let first_word = offset / 4;
        let last_word = (offset + N) / 4;
        let word_n = last_word - first_word;
        let bytes = tapcp::read_device(device, first_word, word_n, &mut self.0)?;
        // Now we slice out the the relevant chunk
        let start_idx = offset % 4;
        Ok(bytes[start_idx..start_idx + N]
            .try_into()
            .expect("This will always be N long"))
    }

    fn temperature(&mut self) -> anyhow::Result<f32> {
        tapcp::temp(&mut self.0)
    }
}
