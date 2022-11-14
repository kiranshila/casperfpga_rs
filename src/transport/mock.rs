//! Mock transport implementations used in testing the interface

use super::Transport;
use crate::core::{Device, DeviceMap};
use anyhow::{anyhow, bail};
use std::collections::HashMap;

struct Mock {
    memory: HashMap<usize, u8>,
    devices: DeviceMap,
}

impl Mock {
    fn new(devices: DeviceMap) -> Self {
        // We'll represent each address lazily instead of havig a dense array
        // but it really shouldn't matter
        let mut memory: HashMap<usize, u8> = Default::default();

        for (_, Device { addr, length }) in devices.iter() {
            for i in 0..*length {
                memory.insert((addr + i) as usize, 0u8);
            }
        }
        Self { devices, memory }
    }
}

impl Transport for Mock {
    fn connect(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn disconnect(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn is_running(&mut self) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn read_vec(&mut self, device: &str, n: usize, offset: usize) -> anyhow::Result<Vec<u8>> {
        // Get the address in memory
        let dev = self
            .devices
            .get(device)
            .ok_or(anyhow!("Device not found"))?;
        // Construct the vector
        let mut bytes = vec![];
        for i in offset..(offset + n) {
            // Pull bytes from memory into bytes vector
            let byte = self
                .memory
                .get(&(dev.addr + i))
                .ok_or(anyhow!("Out of bounds indexing"))?;
            bytes.push(*byte);
        }
        Ok(bytes)
    }

    fn write(&mut self, device: &str, offset: usize, data: &[u8]) -> anyhow::Result<()> {
        // Get the address in memory
        let dev = self
            .devices
            .get(device)
            .ok_or(anyhow!("Device not found"))?;
        if dev.length - offset < data.len() {
            bail!("Attempting to write to a nonexisten address");
        }
        for (i, byte) in data.into_iter().enumerate() {
            self.memory.insert(dev.addr + i + offset, *byte);
        }
        Ok(())
    }

    fn listdev(&mut self) -> anyhow::Result<DeviceMap> {
        Ok(self.devices.clone())
    }

    fn program(&mut self, filename: &std::path::Path) -> anyhow::Result<()> {
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
                fn [<test_rw_$num>]() {
                    let mut transport = Mock::new(HashMap::from([(
                        "sys_scratchpad".to_owned(),
                        Device { addr: 0, length: core::mem::size_of::<$num>() },
                    )]));
                    transport.[<write_$num>]("sys_scratchpad",0,$v).unwrap();
                    let read_num = transport.[<read_$num>]("sys_scratchpad", 0).unwrap();
                    assert_eq!(read_num, $v);
                }
            }
        };
    }

    #[test]
    fn test_read() {
        let mut transport = Mock::new(HashMap::from([(
            "sys_scratchpad".to_owned(),
            Device { addr: 0, length: 4 },
        )]));
        let bytes = transport.read_vec("sys_scratchpad", 4, 0).unwrap();
        assert_eq!(bytes, vec![0, 0, 0, 0]);
    }

    #[test]
    fn test_read_offset() {
        let mut transport = Mock::new(HashMap::from([(
            "sys_scratchpad".to_owned(),
            Device { addr: 0, length: 4 },
        )]));
        let bytes = transport.read_vec("sys_scratchpad", 2, 2).unwrap();
        assert_eq!(bytes, vec![0, 0]);
    }

    #[test]
    fn test_write_read() {
        let mut transport = Mock::new(HashMap::from([(
            "sys_scratchpad".to_owned(),
            Device { addr: 0, length: 4 },
        )]));
        let write_bytes = [1, 2, 3, 4];
        transport.write("sys_scratchpad", 0, &write_bytes).unwrap();
        let read_bytes = transport.read_vec("sys_scratchpad", 4, 0).unwrap();
        assert_eq!(read_bytes, write_bytes);
    }

    #[test]
    fn test_write_read_offset() {
        let mut transport = Mock::new(HashMap::from([(
            "sys_scratchpad".to_owned(),
            Device { addr: 0, length: 4 },
        )]));
        let write_bytes = [7, 8];
        transport.write("sys_scratchpad", 2, &write_bytes).unwrap();
        let read_bytes = transport.read_vec("sys_scratchpad", 4, 0).unwrap();
        assert_eq!(read_bytes, vec![0, 0, 7, 8]);
        let read_bytes = transport.read_vec("sys_scratchpad", 2, 2).unwrap();
        assert_eq!(read_bytes, vec![7, 8]);
    }

    #[test]
    fn test_const_size() {
        let mut transport = Mock::new(HashMap::from([(
            "sys_scratchpad".to_owned(),
            Device { addr: 0, length: 4 },
        )]));
        let write_bytes = [1, 2, 3, 4];
        transport.write("sys_scratchpad", 0, &write_bytes).unwrap();
        let read_bytes = transport.read("sys_scratchpad", 0).unwrap();
        assert_eq!(read_bytes, write_bytes);
    }

    test_rw_num!(u8,42);
    test_rw_num!(u16,0xDEAD);
    test_rw_num!(u32,0xDEAD_BEEF);
    test_rw_num!(u64,0xDEAD_BEEF_B0BA_CAFE);
    test_rw_num!(u128,0xDEAD_BEEF_B0BA_CAFE_0000_0000_0000);
    test_rw_num!(i8,-42);
    test_rw_num!(i16,-0xDEA);
    test_rw_num!(i32,-0xDEAD_BEE);
    test_rw_num!(i64,-0xDEAD_BEEF_B0BA_CAF);
    test_rw_num!(i128,-0xDEAD_BEEF_B0BA_CAFE_0000_0000_0000);
    test_rw_num!(f32,3.1415926);
    test_rw_num!(f64,6.022e23);

}
