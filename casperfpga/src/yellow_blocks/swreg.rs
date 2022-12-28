//! Routines for interacting with CASPER software register yellow blocks. This uses the `fixed`
//! crate to interact with fixed point numbers.

use crate::transport::Transport;
use anyhow::bail;
use std::sync::{
    Mutex,
    Weak,
};

/// The IO direction of this register
#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    /// Client applications can read registers of this kind
    ToProcessor,
    /// Client applications can write registers of this kind
    FromProcessor,
}

/// The unidirectional 32-bit signed fixed point software register yellow block
#[derive(Debug)]
pub struct FixedSoftwareRegister<T> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
    /// IO direction of this register
    direction: Direction,
    /// The binary point
    bin_pts: usize,
    /// The name of the register
    name: String,
}

/// The unidirectional 32-bit unsigned fixed point software register yellow block
#[derive(Debug)]
pub struct UFixedSoftwareRegister<T> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
    /// IO direction of this register
    direction: Direction,
    /// The binary point
    bin_pts: usize,
    /// The name of the register
    name: String,
}

/// The unidirectional 32-bit unsigned fixed point software register yellow block
#[derive(Debug)]
pub struct BooleanSoftwareRegister<T> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
    /// IO direction of this register
    direction: Direction,
    /// The name of the register
    name: String,
}

impl<T> FixedSoftwareRegister<T>
where
    T: Transport,
{
    pub fn from_fpg(
        transport: Weak<Mutex<T>>,
        reg_name: &str,
        io_dir: &str,
        bin_pts: &str,
    ) -> anyhow::Result<Self> {
        let direction = match io_dir {
            "To\\_Processor" => Direction::ToProcessor,
            "From\\_Processor" => Direction::FromProcessor,
            _ => bail!("Malformed FpgDevice metadata entry"),
        };

        let bin_pts = bin_pts.parse()?;

        Ok(Self {
            transport,
            direction,
            bin_pts,
            name: reg_name.to_string(),
        })
    }

    pub fn read(&self) -> anyhow::Result<i32> {
        if self.direction == Direction::FromProcessor {
            bail!("This software register is write-only");
        }
        // TODO
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Perform the read
        transport.read(&self.name, 0)
    }
}

impl<T> UFixedSoftwareRegister<T>
where
    T: Transport,
{
    pub fn from_fpg(
        transport: Weak<Mutex<T>>,
        reg_name: &str,
        io_dir: &str,
        bin_pts: &str,
    ) -> anyhow::Result<Self> {
        let direction = match io_dir {
            "To\\_Processor" => Direction::ToProcessor,
            "From\\_Processor" => Direction::FromProcessor,
            _ => bail!("Malformed FpgDevice metadata entry"),
        };

        let bin_pts = bin_pts.parse()?;

        Ok(Self {
            transport,
            direction,
            bin_pts,
            name: reg_name.to_string(),
        })
    }

    pub fn read(&self) -> anyhow::Result<u32> {
        if self.direction == Direction::FromProcessor {
            bail!("This software register is write-only");
        }
        // TODO
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Perform the read
        transport.read(&self.name, 0)
    }
}

impl<T> BooleanSoftwareRegister<T>
where
    T: Transport,
{
    pub fn from_fpg(
        transport: Weak<Mutex<T>>,
        reg_name: &str,
        io_dir: &str,
    ) -> anyhow::Result<Self> {
        let direction = match io_dir {
            "To\\_Processor" => Direction::ToProcessor,
            "From\\_Processor" => Direction::FromProcessor,
            _ => bail!("Malformed FpgDevice metadata entry"),
        };

        Ok(Self {
            transport,
            direction,
            name: reg_name.to_string(),
        })
    }

    pub fn read(&self) -> anyhow::Result<bool> {
        if self.direction == Direction::FromProcessor {
            bail!("This software register is write-only");
        }
        // TODO
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Perform the read
        let raw: u32 = transport.read(&self.name, 0)?;
        Ok(raw == 1)
    }
}
