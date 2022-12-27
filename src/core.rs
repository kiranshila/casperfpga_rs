//! The core types and functions for interacting with CasperFpga objects
use kstring::KString;
use std::collections::HashMap;

/// The representation of an interal register
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Register {
    /// The offset in FPGA memory of this register
    pub addr: usize,
    /// The number of bytes stored at this location
    pub length: usize,
}

/// The mapping from register names and their data (address and size)
pub type RegisterMap = HashMap<KString, Register>;

/// The Core type of CasperFPGA. This encapsulates the transport method and holds the record of the "current" devices.
pub struct CasperFpga<T> {
    transport: T,
    registers: RegisterMap,
}

// FIXME
// impl CasperFpga<Tapcp> {
//     pub fn new(mut transport: Tapcp) -> anyhow::Result<Self> {
//         if transport.is_running()? {
//             let devices = transport.listdev()?;
//             Ok(CasperFpga { transport, devices })
//         } else {
//             bail!("FPGA is not running")
//         }
//     }
// }
