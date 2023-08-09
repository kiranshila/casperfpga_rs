//! Defines all the transport mechanisms for which all casperfpga transports must implement
pub mod mock;
pub mod tapcp;

use crate::{
    core::RegisterMap,
    yellow_blocks::Address,
};
use casper_utils::design_sources::FpgaDesign;
use thiserror::Error;

#[derive(Error, Debug)]
/// Transport errors
pub enum Error {
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),
    #[error("Trying to transport through a packed struct yeilded a packing error")]
    Packing(#[from] packed_struct::PackingError),
    #[error("The requested device was not found - `{0}`")]
    DeviceNotFound(String),
    #[error(transparent)]
    Mock(#[from] mock::Error),
    #[error(transparent)]
    Tapcp(#[from] tapcp::Error),
}

/// All methods involving transports will have this signature
#[allow(clippy::module_name_repetitions)]
pub type TransportResult<T> = Result<T, Error>;

/// Types that implement this trait can be serialized such that they can be written to FPGA software
/// registers
pub trait Serialize {
    type Chunk;
    fn serialize(&self) -> Self::Chunk;
}

/// Types that implement this trait can be deserialized such that they can be read from FPGA
/// software registers
pub trait Deserialize: Sized {
    type Chunk;
    type Error;
    /// Deserializes from a fixed-size byte slice
    /// # Errors
    /// Errors on invalid bytes for the deserialization
    fn deserialize(chunk: Self::Chunk) -> Result<Self, Self::Error>;
}

macro_rules! ser_num {
    ($num:ty) => {
        impl Serialize for $num {
            type Chunk = [u8; core::mem::size_of::<$num>()];
            fn serialize(&self) -> Self::Chunk {
                self.to_be_bytes()
            }
        }
    };
}

macro_rules! deser_num {
    ($num:ty) => {
        impl Deserialize for $num {
            type Chunk = [u8; core::mem::size_of::<$num>()];
            type Error = std::convert::Infallible;
            fn deserialize(chunk: Self::Chunk) -> Result<Self, Self::Error> {
                Ok(<$num>::from_be_bytes(chunk))
            }
        }
    };
}

// Implement serdes for all builtin numeric types
ser_num!(u8);
ser_num!(u16);
ser_num!(u32);
ser_num!(u64);
ser_num!(u128);
ser_num!(i8);
ser_num!(i16);
ser_num!(i32);
ser_num!(i64);
ser_num!(i128);
ser_num!(f32);
ser_num!(f64);

deser_num!(u8);
deser_num!(u16);
deser_num!(u32);
deser_num!(u64);
deser_num!(u128);
deser_num!(i8);
deser_num!(i16);
deser_num!(i32);
deser_num!(i64);
deser_num!(i128);
deser_num!(f32);
deser_num!(f64);

// Serde for sized slice
impl<const N: usize> Serialize for [u8; N] {
    type Chunk = Self;

    fn serialize(&self) -> Self::Chunk {
        *self
    }
}

impl<const N: usize> Deserialize for [u8; N] {
    type Chunk = Self;
    type Error = std::convert::Infallible;

    fn deserialize(chunk: Self::Chunk) -> Result<Self, Self::Error> {
        Ok(chunk)
    }
}

/// The trait that is implemented for CASPER FPGA transport mechanisms.
/// The methods of this trait *assume* that the device is already connected.
pub trait Transport {
    /// Tests to see if the connected FPGA is programmed and running
    /// # Errors
    /// Returns errors on bad transport
    fn is_running(&mut self) -> TransportResult<bool>;

    /// Read an arbitrary number of bytes `n` from `device` at `offset`
    /// # Errors
    /// Returns errors on bad transport
    fn read_n_bytes(&mut self, device: &str, offset: usize, n: usize) -> TransportResult<Vec<u8>>;

    /// Read `n` bytes from `device` from byte offset `offset` into a const-sized array
    /// # Errors
    /// Returns errors on bad transport
    fn read_bytes<const N: usize>(
        &mut self,
        device: &str,
        offset: usize,
    ) -> TransportResult<[u8; N]> {
        Ok(self
            .read_n_bytes(device, offset, N)?
            .try_into()
            .expect("We read exactly N bytes"))
    }

    /// Generically read a `Deserializable` type `T` from the connected platform at `device` and
    /// offset `offset`.
    /// # Example
    /// ```
    /// # use casperfpga::core::Register;
    /// # use std::collections::HashMap;
    /// # use casperfpga::transport::mock::Mock;
    /// # let mut transport = Mock::new(HashMap::from([("sys_scratchpad".into(),Register { addr: 0, length: 4 },)]));
    /// # use crate::casperfpga::transport::Transport;
    /// let my_num: u32 = transport.read("sys_scratchpad",0).unwrap();
    /// ```
    /// # Errors
    /// Returns errors on bad transport or deserialization
    fn read<T, const N: usize>(&mut self, device: &str, offset: usize) -> TransportResult<T>
    where
        T: Deserialize<Chunk = [u8; N]>,
        Error: std::convert::From<<T as Deserialize>::Error>,
    {
        let bytes: [u8; N] = self.read_bytes(device, offset)?;
        Ok(T::deserialize(bytes)?)
    }

    /// Generically read a `Deserializable` + `Address` type `T` from the connected platform at
    /// `device` and offset specified in the type's address.
    /// # Errors
    /// Returns errors on bad transport or deserialization
    fn read_addr<T, const N: usize>(&mut self, device: &str) -> TransportResult<T>
    where
        T: Deserialize<Chunk = [u8; N]> + Address,
        Error: std::convert::From<<T as Deserialize>::Error>,
    {
        let bytes: [u8; N] = self.read_bytes(device, T::addr() as usize)?;
        Ok(T::deserialize(bytes)?)
    }

    /// Write `data` to `device` from byte offset `offset`
    /// # Errors
    /// Returns errors on bad transport
    fn write_bytes(&mut self, device: &str, offset: usize, data: &[u8]) -> TransportResult<()>;

    /// Generically write a `Serializable` type `T` to the connected platform at `device` and offset
    /// `offset`.
    /// # Example
    /// ```
    /// # use casperfpga::core::Register;
    /// # use std::collections::HashMap;
    /// # use casperfpga::transport::mock::Mock;
    /// # let mut transport = Mock::new(HashMap::from([("sys_scratchpad".into(),Register { addr: 0, length: 4 },)]));
    /// # use crate::casperfpga::transport::Transport;
    /// let my_num = 3.14f32;
    /// transport.write("sys_scratchpad",0, &my_num).unwrap();
    /// ```
    /// # Errors
    /// Returns errors on bad transport
    fn write<T, const N: usize>(
        &mut self,
        device: &str,
        offset: usize,
        data: &T,
    ) -> TransportResult<()>
    where
        T: Serialize<Chunk = [u8; N]>,
    {
        // Create bytes from the data and write with `write_bytes`
        self.write_bytes(device, offset, &data.serialize())
    }

    /// Generically write a `Deserializable` + `Address` type `T` from the connected platform at
    /// `device` and offset specified in the type's address.
    /// # Errors
    /// Returns errors on bad transport
    fn write_addr<T, const N: usize>(&mut self, device: &str, data: &T) -> TransportResult<()>
    where
        T: Serialize<Chunk = [u8; N]> + Address,
    {
        // Create bytes from the data and write with `write_bytes`
        self.write_bytes(device, T::addr() as usize, &data.serialize())
    }

    /// Retrieve a list of available devices on the (potentially programmed) connected platform
    /// # Errors
    /// Returns errors on bad transport
    fn listdev(&mut self) -> TransportResult<RegisterMap>;

    /// Program a bitstream file from `filename` to the connected platform.
    /// Some transports can cache programed bitstreams, so the `force` variable turns off noop-ing
    /// if the bitstream is already programmed.
    /// # Errors
    /// Returns errors on bad transport
    fn program<D>(&mut self, design: &D, force: bool) -> TransportResult<()>
    where
        D: FpgaDesign;

    /// Deprograms the connected platform
    /// # Errors
    /// Returns errors on bad transport
    fn deprogram(&mut self) -> TransportResult<()>;
}
