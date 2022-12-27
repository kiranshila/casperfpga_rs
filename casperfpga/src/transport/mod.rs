//! Defines all the transport mechanisms for which all casperfpga transports must implement

pub mod mock;
pub mod tapcp;

use crate::core::RegisterMap;
use std::path::Path;

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
    fn deserialize(chunk: Self::Chunk) -> anyhow::Result<Self>;
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
            fn deserialize(chunk: Self::Chunk) -> anyhow::Result<Self> {
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

/// The trait that is implemented for CASPER FPGA transport mechanisms.
/// The methods of this trait *assume* that the device is already connected.
pub trait Transport {
    /// Tests to see if the connected FPGA is programmed and running
    fn is_running(&mut self) -> anyhow::Result<bool>;

    /// Read `n` bytes from `device` from byte offset `offset` into a const-sized array
    fn read_bytes<const N: usize>(
        &mut self,
        device: &str,
        offset: usize,
    ) -> anyhow::Result<[u8; N]>;

    /// Generically read a `Deserializable` type `T` from the connected platform at `device` and
    /// offset `offset`. # Example
    /// ```
    /// # use casperfpga::core::Register;
    /// # use std::collections::HashMap;
    /// # use casperfpga::transport::mock::Mock;
    /// # let mut transport = Mock::new(HashMap::from([("sys_scratchpad".into(),Register { addr: 0, length: 4 },)]));
    /// # use crate::casperfpga::transport::Transport;
    /// let my_num: u32 = transport.read("sys_scratchpad",0).unwrap();
    /// ```
    fn read<T, const N: usize>(&mut self, device: &str, offset: usize) -> anyhow::Result<T>
    where
        T: Deserialize<Chunk = [u8; N]>,
    {
        let bytes: [u8; N] = self.read_bytes(device, offset)?;
        T::deserialize(bytes)
    }

    /// Write `data` to `device` from byte offset `offset`
    fn write_bytes(&mut self, device: &str, offset: usize, data: &[u8]) -> anyhow::Result<()>;

    /// Generically write a `Serializable` type `T` to the connected platform at `device` and offset
    /// `offset`. # Example
    /// ```
    /// # use casperfpga::core::Register;
    /// # use std::collections::HashMap;
    /// # use casperfpga::transport::mock::Mock;
    /// # let mut transport = Mock::new(HashMap::from([("sys_scratchpad".into(),Register { addr: 0, length: 4 },)]));
    /// # use crate::casperfpga::transport::Transport;
    /// let my_num = 3.14f32;
    /// transport.write("sys_scratchpad",0, &my_num).unwrap();
    /// ```
    fn write<T, const N: usize>(
        &mut self,
        device: &str,
        offset: usize,
        data: &T,
    ) -> anyhow::Result<()>
    where
        T: Serialize<Chunk = [u8; N]>,
    {
        // Create bytes from the data and write with `write_bytes`
        self.write_bytes(device, offset, &data.serialize())
    }

    /// Retrieve a list of available devices on the (potentially programmed) connected platform
    fn listdev(&mut self) -> anyhow::Result<RegisterMap>;

    /// Program a bitstream file from `filename` to the connected platform
    fn program(&mut self, filename: &Path) -> anyhow::Result<()>;

    /// Deprograms the connected platform
    fn deprogram(&mut self) -> anyhow::Result<()>;
}
