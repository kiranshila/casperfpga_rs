//! Routines for interacting with CASPER software register yellow blocks. This uses the `fixed`
//! crate to interact with fixed point numbers.

use crate::transport::Transport;
use anyhow::bail;
use std::{
    cell::RefCell,
    sync::Weak,
};

/// The IO direction of this register
#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    /// Client applications can read registers of this kind
    ToProcessor,
    /// Client applications can write registers of this kind
    FromProcessor,
}

/// The kind of software register
#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    /// This register contains boolean data
    Bool,
    /// This register contains fixed point data
    Fixed { bin_pts: usize, signed: bool },
}

/// The unidirectional 32-bit fixed point software register yellow block
#[derive(Debug)]
pub struct SoftwareRegister<T> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<RefCell<T>>,
    /// IO direction of this register
    direction: Direction,
    /// The kind of software register
    kind: Kind,
    /// The name of the register
    name: String,
}

impl<T> SoftwareRegister<T>
where
    T: Transport,
{
    pub fn from_fpg(
        transport: Weak<RefCell<T>>,
        reg_name: &str,
        io_dir: &str,
        bin_pts: &str,
        arith_types: &str,
    ) -> anyhow::Result<Self> {
        let direction = match io_dir {
            "To\\_Processor" => Direction::ToProcessor,
            "From\\_Processor" => Direction::FromProcessor,
            _ => bail!("Malformed FpgDevice metadata entry"),
        };

        let bin_pts = bin_pts.parse()?;

        let kind = match arith_types {
            "0" => Kind::Fixed {
                bin_pts,
                signed: false,
            },
            "1" => Kind::Fixed {
                bin_pts,
                signed: true,
            },
            "2" => Kind::Bool,
            _ => bail!("Malformed FpgDevice metadata entry"),
        };

        Ok(SoftwareRegister {
            transport,
            direction,
            kind,
            name: reg_name.to_string(),
        })
    }

    pub fn read(&self) -> anyhow::Result<i64> {
        let transport = self.transport.upgrade().unwrap();

        todo!()
    }
}
