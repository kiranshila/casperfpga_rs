//! The core types and functions for interacting with CasperFpga objects

use std::collections::HashMap;

use crate::transport::Transport;

/// The representation of an interal "yellow block" device, returned from `listdev`
#[derive(Debug, Copy, Clone)]
pub struct Device {
    /// The offset in FPGA memory of this register
    pub addr: usize,
    /// The number of bytes stored at this location
    pub length: usize,
}

/// The main FPGA object that contains the state needed to interface
/// with a CASPER device
pub struct CasperFpga<T> {
    transport: T,
}

impl<T> CasperFpga<T>
where
    T: Transport,
{
    pub fn new() -> Self {
        todo!()
    }
}

/// The mapping from yellow block device names and their `Device` parameters
pub type DeviceMap = HashMap<String, Device>;
