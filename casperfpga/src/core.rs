//! The core types and functions for interacting with casperfpga objects
use crate::transport::Transport;
use kstring::KString;
use std::{
    collections::HashMap,
    time::{
        Duration,
        SystemTime,
    },
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

/// Read the `sys_clkcounter` register a few times to estimate the clock rate in megahertz
/// # Errors
/// Returns an error on bad transport
#[allow(clippy::cast_precision_loss)]
pub fn estimate_fpga_clock<T>(transport: &mut T) -> anyhow::Result<f64>
where
    T: Transport,
{
    let delay_s = 2f64;
    let earlier = SystemTime::now();
    let first_count = u64::from(transport.read::<u32, 4>("sys_clkcounter", 0)?);
    let later = SystemTime::now();
    std::thread::sleep(Duration::from_secs_f64(delay_s));
    let mut second_count = u64::from(transport.read::<u32, 4>("sys_clkcounter", 0)?);
    if first_count > second_count {
        second_count += 2u64.pow(32);
    }
    let transport_elapsed = later.duration_since(earlier)?;
    let transport_delay = transport_elapsed.as_secs_f64();
    Ok((second_count - first_count) as f64 / ((delay_s - transport_delay) * 1_000_000_f64))
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
