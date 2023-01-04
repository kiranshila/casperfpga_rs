//! Utilities for working with files that provide a bitstream

use kstring::KString;
use std::collections::HashMap;

pub mod fpg;

#[derive(Debug, PartialEq, Eq)]
/// A register on the FPGA bus described by its 32-bit address and size in bytes
pub struct Register {
    pub addr: u32,
    pub size: u32,
}

#[derive(Debug, PartialEq, Eq)]
/// An enumeratable "device" described by it's kind, potential corresponding register, and any
/// (String,String) metadata
pub struct Device {
    pub kind: String,
    pub register: Option<Register>,
    pub metadata: HashMap<KString, String>,
}

impl Device {
    fn add_meta(&mut self, k: KString, v: String) {
        self.metadata.insert(k, v);
    }
}

/// A map from device name (corresponding with a register name) to [`Device`]
pub type Devices = HashMap<KString, Device>;

/// Any type that provides all the information to concretly describe a CASPER design must implement
/// the [`FpgaDesign`] trait. Right now this is just FPG files, but could be extended to bitstream +
/// device tree, etc.
pub trait FpgaDesign {
    /// Get the uncompressed bitstream for a given design as bytes, ready to program
    fn bitstream(&self) -> &Vec<u8>;

    /// Get a hash used to validate a given design is currently programmed
    fn md5(&self) -> &[u8; 16];

    /// Get a string representation of the MD5 hash
    fn md5_string(&self) -> String {
        self.md5().iter().map(|&v| format!("{v:x}")).collect()
    }

    /// Get the list of potentially constructable devices
    fn devices(&self) -> &Devices;
}
