//! The core types and functions for interacting with CasperFpga objects
use crate::transport::{tapcp::Tapcp, Transport};
use anyhow::bail;
use kstring::KString;
use std::{collections::HashMap, net::SocketAddr, path::Path};

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

/// The default type of CasperFPGA. This encapsulates the transport method and holds the record of
/// the "current" devices, but provides no high level typesafe interfaces into yellow blocks.
pub struct CasperFpga<T> {
    pub transport: T,
    pub registers: RegisterMap,
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
            Ok(CasperFpga {
                transport,
                registers,
            })
        } else {
            bail!("FPGA is not running")
        }
    }
}

#[cfg(feature = "python")]
mod python {
    use crate::transport::tapcp::python::add_tapcp;
    use pyo3::prelude::*;

    #[pymodule]
    // Build the module hierarchy to match this one
    fn casperfpga(py: Python<'_>, m: &PyModule) -> PyResult<()> {
        register_child_modules(py, m)?;
        Ok(())
    }

    fn register_child_modules(py: Python<'_>, parent_module: &PyModule) -> PyResult<()> {
        let transport = PyModule::new(py, "transport")?;
        let yellow_blocks = PyModule::new(py, "yellow_blocks")?;

        // Add the members of each submodule
        add_tapcp(py, transport)?;

        parent_module.add_submodule(transport)?;
        parent_module.add_submodule(yellow_blocks)?;
        Ok(())
    }
}
