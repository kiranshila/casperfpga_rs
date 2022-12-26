//! The core types and functions for interacting with CasperFpga objects

use crate::transport::{tapcp::Tapcp, Transport};
use anyhow::bail;
use kstring::KString;
use std::collections::HashMap;

/// The representation of an interal "yellow block" device, returned from `listdev`
#[derive(Debug, Copy, Clone)]
pub struct Device {
    /// The offset in FPGA memory of this register
    pub addr: usize,
    /// The number of bytes stored at this location
    pub length: usize,
}

/// The mapping from yellow block device names and their `Device` parameters
pub type DeviceMap = HashMap<KString, Device>;

/// The Core type of CasperFPGA. This encapsulates the transport method and holds the record of the "current" devices.
pub struct CasperFpga<T> {
    transport: T,
    devices: DeviceMap,
}

// FIXME
impl CasperFpga<Tapcp> {
    pub fn new(mut transport: Tapcp) -> anyhow::Result<Self> {
        if transport.is_running()? {
            let devices = transport.listdev()?;
            Ok(CasperFpga { transport, devices })
        } else {
            bail!("FPGA is not running")
        }
    }
}
