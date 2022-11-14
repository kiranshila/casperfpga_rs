//! Defines all the transport mechanisms for which all casperfpga transports must implement

use crate::core::DeviceMap;
use anyhow::bail;
use paste::paste;
use std::path::Path;

mod mock;
pub mod tapcp;

macro_rules! read_num {
    ($num:ty) => {
        paste! {
            #[doc = "Read a `" $num "` from `device` from byte offset `offset`"]
            fn [<read_$num>](&mut self, device: &str, offset: usize) -> anyhow::Result<$num> {
                Ok($num::from_be_bytes(self.read(device, offset)?))
            }
        }
    };
}

macro_rules! write_num {
    ($num:ty) => {
        paste! {
            #[doc = "Read a `" $num "` from `device` from byte offset `offset`"]
            fn [<write_$num>](&mut self, device: &str, offset: usize, num: $num) -> anyhow::Result<()> {
                self.write(device,offset,&num.to_be_bytes())?;
                Ok(())
            }
        }
    };
}

pub trait Transport {
    /// Connect to a device running a supported transport mechanism
    fn connect(&mut self) -> anyhow::Result<()>;

    /// Disconnect from the device
    fn disconnect(&mut self) -> anyhow::Result<()>;

    /// Tests to see if the connected FPGA is programmed and running
    fn is_running(&mut self) -> anyhow::Result<bool>;

    /// Tests to see if the transport layer is connected to the platform
    /// By default, this will call is_running
    fn is_connected(&mut self) -> anyhow::Result<bool> {
        self.is_running()
    }

    /// Read `n` bytes from `device` from byte offset `offset` into a vector
    fn read_vec(&mut self, device: &str, n: usize, offset: usize) -> anyhow::Result<Vec<u8>>;

    /// Read `n` bytes from `device` from byte offset `offset` into a const-sized array
    /// This is useful for deserializing into statically-sized containers such as numbers and packed structs
    fn read<const N: usize>(&mut self, device: &str, offset: usize) -> anyhow::Result<[u8; N]> {
        let bytes = self.read_vec(device, N, offset)?;
        // Ensure size
        let slice = bytes.as_slice();
        let array = match slice.try_into() {
            Ok(a) => a,
            Err(_) => bail!("We asked for {} bytes but received {}", N, bytes.len()),
        };
        Ok(array)
    }

    // Generate the reads for the numbers
    read_num!(u8);
    read_num!(u16);
    read_num!(u32);
    read_num!(u64);
    read_num!(u128);
    read_num!(i8);
    read_num!(i16);
    read_num!(i32);
    read_num!(i64);
    read_num!(i128);
    read_num!(f32);
    read_num!(f64);

    /// Write `data` to `device` from byte offset `offset`
    fn write(&mut self, device: &str, offset: usize, data: &[u8]) -> anyhow::Result<()>;

    // Generate the writes for the numbers
    write_num!(u8);
    write_num!(u16);
    write_num!(u32);
    write_num!(u64);
    write_num!(u128);
    write_num!(i8);
    write_num!(i16);
    write_num!(i32);
    write_num!(i64);
    write_num!(i128);
    write_num!(f32);
    write_num!(f64);

    /// Retrieve a list of available devices on the (potentially programmed) connected platform
    fn listdev(&mut self) -> anyhow::Result<DeviceMap>;

    /// Program a bitstream file from `filename` to the connected platform
    fn program(&mut self, filename: &Path) -> anyhow::Result<()>;

    /// Deprograms the connected platform
    fn deprogram(&mut self) -> anyhow::Result<()>;
}
