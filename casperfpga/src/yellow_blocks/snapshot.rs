//! TODO - support bitsnap, integrate with bram lib

use crate::transport::{Deserialize, Serialize, Transport};
use casperfpga_derive::CasperSerde;
use num_traits::Unsigned;
use packed_struct::prelude::*;
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Transport(#[from] crate::transport::Error),
    #[error("Failed to parse number of samples from fpg file")]
    BadSampleN,
    #[error("The snapshot block that we tried to set an offset on didn't support offsets")]
    NoOffsets,
}

/// The snapshot yellow block to capture a chunk of samples
#[derive(Debug)]
pub struct Snapshot<T, F> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
    /// The name of the register
    name: String,
    /// Marker for the integer type of the data type
    phantom: PhantomData<F>,
    /// Flag for whether this snapshot block has separate "offset" control
    has_offset: bool,
    /// Number of samples (2^n)
    samples_n: u32,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone, CasperSerde)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "4")]
#[allow(clippy::struct_excessive_bools)]
pub struct Control {
    #[packed_field(bits = "0")]
    arm: bool,
    #[packed_field(bits = "1")]
    trig_override: bool,
    #[packed_field(bits = "2")]
    write_enable_override: bool,
    #[packed_field(bits = "3")]
    circular_capture: bool, // This isn't documented, so I'm not sure if it's real
}

#[derive(Debug, PackedStruct, Default, Copy, Clone, CasperSerde)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "4")]
pub struct Status {
    #[packed_field(bits = "0..31", endian = "msb")]
    addr: u32,
    #[packed_field(bits = "31")]
    done: bool,
}

impl<T, F> Snapshot<T, F>
where
    T: Transport,
    F: Unsigned,
{
    #[must_use]
    pub fn new(
        transport: &Arc<Mutex<T>>,
        reg_name: &str,
        has_offset: bool,
        samples_n: u32,
    ) -> Self {
        let transport = Arc::downgrade(transport);
        Self {
            transport,
            name: reg_name.to_string(),
            phantom: PhantomData,
            has_offset,
            samples_n,
        }
    }

    /// Builds a [`Snapshot`] from fpg details
    /// # Errors
    /// Returns an error on bad string arguments
    pub fn from_fpg(
        transport: Weak<Mutex<T>>,
        reg_name: &str,
        nsamples: &str,
        offset: &str,
    ) -> Result<Self, Error> {
        let samples_n = nsamples.parse().map_err(|_| Error::BadSampleN)?;
        let has_offset = match offset {
            "off" => false,
            "on" => true,
            _ => unreachable!(),
        };
        Ok(Self {
            transport,
            name: reg_name.to_string(),
            phantom: PhantomData,
            has_offset,
            samples_n,
        })
    }

    /// Arm the snapshot block so that the next trigger starts capture
    /// # Errors
    /// Returns an error on transport errors
    #[allow(clippy::missing_panics_doc)]
    pub fn arm(&self) -> Result<(), Error> {
        let control_reg = format!("{}_ctrl", self.name);
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let mut ctrl = Control::default();
        transport.write(&control_reg, 0, &ctrl)?;
        ctrl.arm = true;
        transport.write(&control_reg, 0, &ctrl)?;
        Ok(())
    }

    /// Read the data from the snapshot block.
    /// This will not check if we captured a full block and will return an error if it's not "done"
    /// as indicated by the status register.
    /// # Errors
    /// Returns an error on transport errors
    #[allow(clippy::missing_panics_doc)]
    pub fn read(&self) -> Result<Vec<u8>, Error> {
        let status_reg = format!("{}_status", self.name);
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let _status: Status = transport.read(&status_reg, 0)?;
        // FIXME
        let bram_reg = format!("{}_bram", self.name);
        let bytes =
            transport.read_n_bytes(&bram_reg, 0, 2u32.pow(self.samples_n).try_into().unwrap())?;
        // There's a way to reinterpret this inplace...somehow
        Ok(bytes)
    }

    /// Force a trigger
    /// # Errors
    /// Returns an error on transport errors
    #[allow(clippy::missing_panics_doc)]
    pub fn trigger(&self) -> Result<(), Error> {
        let control_reg = format!("{}_ctrl", self.name);
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let mut ctrl: Control = transport.read(&control_reg, 0)?;
        ctrl.trig_override = true;
        transport.write(&control_reg, 0, &ctrl)?;
        Ok(())
    }

    /// Set the capture trigger offset
    /// # Errors
    /// Returns an error on transport errors and when the snapshot block doesn't support offsets
    #[allow(clippy::missing_panics_doc)]
    pub fn set_offset(&self, offset: u32) -> Result<(), Error> {
        if self.has_offset {
            let offset_reg = format!("{}_trig_offset", self.name);
            let tarc = self.transport.upgrade().unwrap();
            let mut transport = (*tarc).lock().unwrap();
            transport.write(&offset_reg, 0, &offset)?;
        } else {
            return Err(Error::NoOffsets);
        }
        Ok(())
    }
}
