//! # Software Register
//!
//! This block is a semi-unidirectional 32-bit register shared between the FPGA design and a client
//! application. The design itself can specify a custom bitwidth up to 32 bits, but I/O will always
//! be to 32 bits, bailing at runtime on overflow conditions.
//!
//! There are two unique types for this register, signed fixed point ([`FixedSoftwareRegister`]) and
//! boolean ([`BooleanSoftwareRegister`]). Both of these types will have read
//! and write methods, bailing on write if [Direction] isn't [`Direction::FromProcessor`].
//!
//! Interactions with this block require the use of types from the [fixed](https://docs.rs/fixed/latest/fixed/) crate,
//! and are currently a little clunky as that crate hasn't fully updated to use const-generics for
//! the binary point. This will improve once those features arrive in rust stable.
//!
//! ## Toolflow Documentation
//! <https://casper-toolflow.readthedocs.io/en/latest/src/blockdocs/Software_register.html>

use crate::transport::Transport;
use fixed::traits::Fixed;
use std::{
    marker::PhantomData,
    sync::{
        Arc,
        Mutex,
        Weak,
    },
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Transport(#[from] crate::transport::Error),
    #[error("We tried to write to a read-only register")]
    ReadOnly,
    #[error("Invalid direction specified from fpg file")]
    BadDirection,
    #[error("Failed to parse the bitwidth field from the fpg file")]
    BadBitwidth,
    #[error("The number we tried to write doesn't fit in the destination")]
    Overflow,
}

/// The IO direction of this register
#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    /// Client applications can read registers of this kind
    ToProcessor,
    /// Client applications can read and write registers of this kind
    FromProcessor,
}

/// The unidirectional signed fixed point software register yellow block
#[derive(Debug)]
pub struct FixedSoftwareRegister<T, F> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
    /// IO direction of this register
    direction: Direction,
    /// Number of bits
    width: usize,
    /// The name of the register
    name: String,
    /// Marker for the fixed point type
    phantom: PhantomData<F>,
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

impl<T, F> FixedSoftwareRegister<T, F>
where
    T: Transport,
    F: Fixed<Bytes = [u8; 4]>,
{
    #[must_use]
    pub fn new(
        transport: &Arc<Mutex<T>>,
        reg_name: &str,
        direction: Direction,
        width: usize,
    ) -> Self {
        let transport = Arc::downgrade(transport);
        Self {
            transport,
            direction,
            width,
            name: reg_name.to_string(),
            phantom: PhantomData,
        }
    }

    /// Builds a [`FixedSoftwareRegister`] from FPG description strings
    /// # Errors
    /// Returns an error on bad string arguments
    pub fn from_fpg(
        transport: Weak<Mutex<T>>,
        reg_name: &str,
        io_dir: &str,
        bitwidths: &str,
    ) -> Result<Self, Error> {
        let direction = match io_dir {
            "To\\_Processor" => Direction::ToProcessor,
            "From\\_Processor" => Direction::FromProcessor,
            _ => return Err(Error::BadDirection),
        };
        let width = bitwidths.parse().map_err(|_| Error::BadBitwidth)?;
        Ok(Self {
            transport,
            direction,
            width,
            name: reg_name.to_string(),
            phantom: PhantomData,
        })
    }

    /// Reads a fixed point number from the register
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn read(&self) -> Result<F, Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Perform the read
        Ok(F::from_be_bytes(transport.read(&self.name, 0)?))
    }

    /// Write a fixed point number to the register
    /// # Errors
    /// Returns an error on bad transport
    /// # Panics
    /// Panics if the width of the register is more than 32 bits (it should never be)
    pub fn write(&self, val: F) -> Result<(), Error> {
        // Check direction
        if self.direction == Direction::ToProcessor {
            return Err(Error::ReadOnly);
        }
        // Check width
        if val > (2_usize.pow(self.width.try_into().unwrap()) - 1 / 2_usize.pow(F::FRAC_NBITS)) {
            return Err(Error::Overflow);
        }
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Perform the write
        Ok(transport.write(&self.name, 0, &(val.to_be_bytes()))?)
    }
}

impl<T> BooleanSoftwareRegister<T>
where
    T: Transport,
{
    #[must_use]
    pub fn new(transport: &Arc<Mutex<T>>, reg_name: &str, direction: Direction) -> Self {
        let transport = Arc::downgrade(transport);
        Self {
            transport,
            direction,
            name: reg_name.to_string(),
        }
    }

    /// Builds a [`BooleanSoftwareRegister`] from FPG description strings
    /// # Errors
    /// Returns an error on bad string arguments
    pub fn from_fpg(
        transport: Weak<Mutex<T>>,
        reg_name: &str,
        io_dir: &str,
    ) -> Result<Self, Error> {
        let direction = match io_dir {
            "To\\_Processor" => Direction::ToProcessor,
            "From\\_Processor" => Direction::FromProcessor,
            _ => return Err(Error::BadDirection),
        };

        Ok(Self {
            transport,
            direction,
            name: reg_name.to_string(),
        })
    }

    /// Reads a boolean from the register
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn read(&self) -> Result<bool, Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Perform the read
        let raw: u32 = transport.read(&self.name, 0)?;
        Ok(raw == 1)
    }

    /// Writes a boolean to the register
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn write(&self, val: bool) -> Result<(), Error> {
        if self.direction == Direction::ToProcessor {
            return Err(Error::ReadOnly);
        }
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Perform the write
        Ok(transport.write(&self.name, 0, &(u32::from(val)))?)
    }
}

#[cfg(test)]
mod tests {
    use fixed::types::{
        I25F7,
        U27F5,
    };

    use super::*;
    use crate::{
        core::Register,
        transport::mock::Mock,
    };
    use std::collections::HashMap;

    #[test]
    fn test_fixed_readwrite() {
        let transport = Mock::new(HashMap::from([(
            "my_reg".into(),
            Register { addr: 0, length: 4 },
        )]));
        let transport = Arc::new(Mutex::new(transport));
        let my_reg = FixedSoftwareRegister::<_, U27F5>::new(
            &transport,
            "my_reg",
            Direction::FromProcessor,
            32,
        );
        let test_num = U27F5::from_num(2.75);
        my_reg.write(test_num).unwrap();
        assert_eq!(test_num, my_reg.read().unwrap());
    }

    #[test]
    fn test_ufixed_readwrite() {
        let transport = Mock::new(HashMap::from([(
            "my_reg".into(),
            Register { addr: 0, length: 4 },
        )]));
        let transport = Arc::new(Mutex::new(transport));
        let my_reg = FixedSoftwareRegister::<_, I25F7>::new(
            &transport,
            "my_reg",
            Direction::FromProcessor,
            32,
        );
        let test_num = I25F7::from_num(3.15625);
        my_reg.write(test_num).unwrap();
        assert_eq!(test_num, my_reg.read().unwrap());
    }

    #[test]
    fn test_bool_readwrite() {
        let transport = Mock::new(HashMap::from([(
            "my_reg".into(),
            Register { addr: 0, length: 4 },
        )]));
        let transport = Arc::new(Mutex::new(transport));
        let my_reg = BooleanSoftwareRegister::new(&transport, "my_reg", Direction::FromProcessor);
        let test_val = false;
        my_reg.write(test_val).unwrap();
        assert_eq!(test_val, my_reg.read().unwrap());
        let test_val = true;
        my_reg.write(test_val).unwrap();
        assert_eq!(test_val, my_reg.read().unwrap());
    }
}
