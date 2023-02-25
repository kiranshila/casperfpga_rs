//! Mock transport implementations used in testing the interface

use super::Transport;
use crate::core::{Register, RegisterMap};
use anyhow::{anyhow, bail};
use casper_utils::design_sources::FpgaDesign;
use std::collections::HashMap;

/// A platform that mocks reads and writes, useful for testing
#[derive(Debug)]
pub struct Mock {
    memory: HashMap<usize, u8>,
    registers: RegisterMap,
}

impl Mock {
    /// Construct a new mock platform by providing a device map `devices`
    #[must_use]
    pub fn new(registers: RegisterMap) -> Self {
        // We'll represent each address lazily instead of havig a dense array
        // but it really shouldn't matter
        let mut memory: HashMap<usize, u8> = HashMap::default();

        for Register { addr, length } in registers.values() {
            for i in 0..*length {
                memory.insert(addr + i, 0u8);
            }
        }
        Self { memory, registers }
    }
}

impl Transport for Mock {
    fn is_running(&mut self) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn read_n_bytes(&mut self, device: &str, offset: usize, n: usize) -> anyhow::Result<Vec<u8>> {
        // Get the address in memory
        let dev = self
            .registers
            .get(device)
            .ok_or_else(|| anyhow!("Device not found"))?;
        // Construct the array
        let mut bytes = vec![0u8; n];
        for i in offset..(offset + n) {
            // Pull bytes from memory into bytes vector
            let byte = self
                .memory
                .get(&(dev.addr + i))
                .ok_or_else(|| anyhow!("Out of bounds indexing"))?;
            bytes[i - offset] = *byte;
        }
        Ok(bytes)
    }

    fn read<T, const N: usize>(&mut self, device: &str, offset: usize) -> anyhow::Result<T>
    where
        T: super::Deserialize<Chunk = [u8; N]>,
    {
        let bytes: [u8; N] = self.read_bytes(device, offset)?;
        T::deserialize(bytes)
    }

    fn write_bytes(&mut self, device: &str, offset: usize, data: &[u8]) -> anyhow::Result<()> {
        // Get the address in memory
        let dev = self
            .registers
            .get(device)
            .ok_or_else(|| anyhow!("Device not found"))?;
        if dev.length - offset < data.len() {
            bail!("Attempting to write to a nonexistent address");
        }
        for (i, byte) in data.iter().enumerate() {
            self.memory.insert(dev.addr + i + offset, *byte);
        }
        Ok(())
    }

    fn write<T, const N: usize>(
        &mut self,
        device: &str,
        offset: usize,
        data: &T,
    ) -> anyhow::Result<()>
    where
        T: super::Serialize<Chunk = [u8; N]>,
    {
        // Create bytes from the data and write with `write_bytes`
        self.write_bytes(device, offset, &data.serialize())
    }

    fn listdev(&mut self) -> anyhow::Result<RegisterMap> {
        Ok(self.registers.clone())
    }

    fn program<D>(&mut self, _design: &D, _force: bool) -> anyhow::Result<()>
    where
        D: FpgaDesign,
    {
        todo!()
    }

    fn deprogram(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paste::paste;

    macro_rules! test_rw_num {
        ($num:ty, $v:literal) => {
            paste! {
                #[test]
                #[allow(clippy::float_cmp)]
                fn [<test_rw_$num>]() {
                    let mut transport = Mock::new(HashMap::from([(
                        "sys_scratchpad".into(),
                        Register { addr: 0, length: core::mem::size_of::<$num>() },
                    )]));
                    let num: $num = $v;
                    transport.write("sys_scratchpad", 0, &num).unwrap();
                    let read_num: $num = transport.read("sys_scratchpad", 0).unwrap();
                    assert_eq!(read_num, num);
                }
            }
        };
    }

    #[test]
    fn test_read() {
        let mut transport = Mock::new(HashMap::from([(
            "sys_scratchpad".into(),
            Register { addr: 0, length: 4 },
        )]));
        let bytes = transport.read_bytes("sys_scratchpad", 0).unwrap();
        assert_eq!(bytes, [0, 0, 0, 0]);
    }

    #[test]
    fn test_read_offset() {
        let mut transport = Mock::new(HashMap::from([(
            "sys_scratchpad".into(),
            Register { addr: 0, length: 4 },
        )]));
        let bytes = transport.read_bytes("sys_scratchpad", 2).unwrap();
        assert_eq!(bytes, [0, 0]);
    }

    #[test]
    fn test_write_read() {
        let mut transport = Mock::new(HashMap::from([(
            "sys_scratchpad".into(),
            Register { addr: 0, length: 4 },
        )]));
        let write_bytes = [1, 2, 3, 4];
        transport
            .write_bytes("sys_scratchpad", 0, &write_bytes)
            .unwrap();
        let read_bytes = transport.read_bytes("sys_scratchpad", 0).unwrap();
        assert_eq!(read_bytes, write_bytes);
    }

    #[test]
    fn test_write_read_offset() {
        let mut transport = Mock::new(HashMap::from([(
            "sys_scratchpad".into(),
            Register { addr: 0, length: 4 },
        )]));
        let write_bytes = [7, 8];
        transport
            .write_bytes("sys_scratchpad", 2, &write_bytes)
            .unwrap();
        let read_bytes = transport.read_bytes("sys_scratchpad", 0).unwrap();
        assert_eq!(read_bytes, [0, 0, 7, 8]);
        let read_bytes = transport.read_bytes("sys_scratchpad", 2).unwrap();
        assert_eq!(read_bytes, [7, 8]);
    }

    #[test]
    fn test_const_size() {
        let mut transport = Mock::new(HashMap::from([(
            "sys_scratchpad".into(),
            Register { addr: 0, length: 4 },
        )]));
        let write_bytes = [1, 2, 3, 4];
        transport
            .write_bytes("sys_scratchpad", 0, &write_bytes)
            .unwrap();
        let read_bytes = transport.read_bytes("sys_scratchpad", 0).unwrap();
        assert_eq!(read_bytes, write_bytes);
    }

    test_rw_num!(u8, 42);
    test_rw_num!(u16, 0xDEAD);
    test_rw_num!(u32, 0xDEAD_BEEF);
    test_rw_num!(u64, 0xDEAD_BEEF_B0BA_CAFE);
    test_rw_num!(u128, 0xDEAD_BEEF_B0BA_CAFE_0000_0000_0000);
    test_rw_num!(i8, -42);
    test_rw_num!(i16, -0xDEA);
    test_rw_num!(i32, -0x0DEA_DBEE);
    test_rw_num!(i64, -0x0DEA_DBEE_FB0B_ACAF);
    test_rw_num!(i128, -0xDEAD_BEEF_B0BA_CAFE_0000_0000_0000);
    test_rw_num!(f32, 1.618);
    test_rw_num!(f64, -6.022e23);
}
