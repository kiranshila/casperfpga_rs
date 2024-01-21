//! "Local" transport where we have access to `/dev/mem` mapped FPGA fabric

use casper_utils::design_sources::Devices;
use memmap2::{
    MmapMut,
    MmapOptions,
};
use nix::libc::O_SYNC;
use std::{
    fs::File,
    os::unix::fs::OpenOptionsExt,
};

use super::Transport;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("File IO error")]
    IO(#[from] std::io::Error),
    #[error("Requested register doesn't exist - `{0}`")]
    MissingRegister(String),
}

#[derive(Debug)]
/// A local connection to FPGA fabric via `/dev/mem`
pub struct Local {
    mem: MmapMut,
    devices: Devices,
    base_addr: u32,
}

impl Local {
    /// Construct a new local `/dev/mem` transport.
    ///
    /// Note: This may require some file permission bologna
    /// # Errors
    /// Returns errors on file IO errors
    pub fn new(devices: Devices) -> Result<Self, Error> {
        // Find the min and max device addrs to determine the memory space we want to map
        let mut base_addr = 0;
        let mut top_addr = 0;
        for dev in devices.values() {
            if let Some(reg) = &dev.register {
                if reg.addr < base_addr {
                    base_addr = reg.addr;
                }
                if reg.addr + reg.size > top_addr {
                    top_addr = reg.addr + reg.size;
                }
            }
        }
        // Open /dev/mem readonly
        let mem = File::options()
            .read(true)
            .write(true)
            .custom_flags(O_SYNC)
            .open("/dev/mem")?;
        let mmap = unsafe {
            MmapOptions::new()
                .len((top_addr - base_addr).try_into().unwrap())
                .offset(base_addr.into())
                .map(&mem)?
                .make_mut()?
        };
        Ok(Self {
            mem: mmap,
            devices,
            base_addr,
        })
    }
}

impl Transport for Local {
    fn is_running(&mut self) -> super::TransportResult<bool> {
        // Check to see if sys_clkcounter exists
        todo!()
    }

    fn read_n_bytes(
        &mut self,
        register: &str,
        offset: usize,
        n: usize,
    ) -> super::TransportResult<Vec<u8>> {
        if let Some(dev) = self.devices.get(register) {
            if let Some(reg) = &dev.register {
                let map_addr: usize = (reg.addr - self.base_addr).try_into().unwrap();
                let start = map_addr + offset;
                let stop = map_addr + offset + n;
                let slice = &self.mem[start..stop];
                return Ok(slice.to_vec());
            }
        }
        Err(super::Error::Local(Error::MissingRegister(
            register.to_string(),
        )))
    }

    fn write_bytes(
        &mut self,
        register: &str,
        offset: usize,
        data: &[u8],
    ) -> super::TransportResult<()> {
        // Determine mem addr
        if let Some(dev) = self.registers.get(register) {
            if let Some(reg) = &dev.register {
                let map_addr: usize = (reg.addr - self.base_addr).try_into().unwrap();
                let start = map_addr + offset;
                let stop = map_addr + offset + data.len();
                let slice = &mut self.mem[start..stop];
                // Write bytes
                slice.clone_from_slice(data);
                return Ok(());
            }
        }
        Err(super::Error::Local(Error::MissingRegister(
            device.to_string(),
        )))
    }

    fn listdev(&mut self) -> super::TransportResult<crate::core::RegisterMap> {
        todo!()
    }

    fn program<D>(&mut self, design: &D, force: bool) -> super::TransportResult<()>
    where
        D: casper_utils::design_sources::FpgaDesign,
    {
        todo!()
    }

    fn deprogram(&mut self) -> super::TransportResult<()> {
        todo!()
    }
}
