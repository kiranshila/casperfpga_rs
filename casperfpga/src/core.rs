//! The core types and functions for interacting with CasperFpga objects
use crate::transport::{
    tapcp::Tapcp,
    Transport,
};
use anyhow::bail;
use kstring::KString;
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::Path,
};

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

/// The Core type of CasperFPGA. This encapsulates the transport method and holds the record of the
/// "current" devices.
pub struct CasperFpga<T> {
    pub transport: std::sync::Arc<T>,
    registers: RegisterMap,
    foo: Foo<T>,
}

struct Foo<T> {
    transport: std::sync::Weak<T>,
}

// Constructors

impl CasperFpga<Tapcp> {
    pub fn new<T>(host: SocketAddr) -> anyhow::Result<Self>
    where
        T: AsRef<Path>,
    {
        let mut transport = Tapcp::connect(host)?;
        if transport.is_running()? {
            let registers = transport.listdev()?;
            let tarc = std::sync::Arc::new(transport);
            let foo = Foo {
                transport: std::sync::Arc::downgrade(&tarc),
            };
            Ok(CasperFpga {
                transport: tarc,
                registers,
                foo,
            })
        } else {
            bail!("FPGA is not running")
        }
    }
}
