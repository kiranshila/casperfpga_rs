//! The casperfpga transport implementations for TAPCP

use super::Transport;
use crate::core::{
    Register,
    RegisterMap,
};
use anyhow::bail;
use casper_utils::design_sources::FpgaDesign;
use indicatif::ProgressBar;
use kstring::KString;
use std::{
    collections::HashMap,
    net::{
        SocketAddr,
        UdpSocket,
    },
    time::Duration,
};

const DEFAULT_TIMEOUT: f32 = 0.1;
const DEFAULT_RETRIES: usize = 7; // up to 1.7s

/// Platforms that support TAPCP
#[derive(Debug, Copy, Clone)]
pub enum Platform {
    SNAP,
    SNAP2,
}

impl Platform {
    fn flash_location(self) -> u32 {
        match self {
            Platform::SNAP => 0x0080_0000,
            Platform::SNAP2 => 0x00C0_0000,
        }
    }

    fn program_location(self) -> u32 {
        self.flash_location() + tapcp::FLASH_SECTOR_SIZE
    }
}

#[derive(Debug)]
/// A TAPCP Connection (newtype for a [`UdpSocket`])
pub struct Tapcp {
    socket: UdpSocket,
    retries: usize,
    platform: Platform,
}

impl Tapcp {
    /// Create and connect to a TAPCP transport
    /// # Errors
    /// Will return an error if the UDP socket fails to connect
    pub fn connect(host: SocketAddr, platform: Platform) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        // Set a default timeout
        let timeout = Duration::from_secs_f32(DEFAULT_TIMEOUT);
        socket.set_write_timeout(Some(timeout))?;
        socket.set_read_timeout(Some(timeout))?;
        // Connect
        socket.connect(host)?;
        // And return
        Ok(Self {
            socket,
            retries: DEFAULT_RETRIES,
            platform,
        })
    }
}

// Transport trait implementations

impl Transport for Tapcp {
    fn is_running(&mut self) -> anyhow::Result<bool> {
        // Check if sys_clkcounter exists
        match tapcp::read_device("sys_clkcounter", 0, 1, &mut self.socket, self.retries) {
            Ok(_) => Ok(true),
            // In the case we get back a file not found error,
            // that implies the device is not running a user program.
            // Any other error is actually an error
            Err(e1) => match e1.downcast_ref::<tapcp::tftp::Error>() {
                Some(e2) => match e2 {
                    tapcp::tftp::Error::ErrorResponse(code, _) => match code {
                        tapcp::tftp::ErrorCode::NotFound => Ok(false),
                        _ => bail!(e1),
                    },
                    _ => bail!(e1),
                },
                None => bail!(e1),
            },
        }
    }

    fn write_bytes(&mut self, device: &str, offset: usize, data: &[u8]) -> anyhow::Result<()> {
        // The inverted version of `read_vec`. The problem here is if we are not writing a 4 byte
        // chunk (which we need to), we have to read the bytes that are already there and include
        // them. Because we don't want to do this read when we don't have to, we will branch
        if (offset % 4) == 0 && (data.len() % 4) == 0 {
            // Just do the write
            tapcp::write_device(device, offset % 4, data, &mut self.socket, self.retries)?;
        } else {
            todo!()
        }
        Ok(())
    }

    fn listdev(&mut self) -> anyhow::Result<RegisterMap> {
        let devices = tapcp::listdev(&mut self.socket, self.retries)?;
        Ok(devices
            .iter()
            .map(|(k, (addr, len))| {
                (
                    k.into(),
                    Register {
                        addr: *addr as usize,
                        length: *len as usize,
                    },
                )
            })
            .collect())
    }

    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_precision_loss)]
    fn program<D>(&mut self, design: &D, force: bool) -> anyhow::Result<()>
    where
        D: FpgaDesign,
    {
        // First check to see if we even need to program by comparing the hashes
        let meta = self.metadata()?;
        if let Some(hash) = meta.get("md5") {
            if hash == &design.md5_string() && !force {
                return Ok(());
            }
        }
        // Else we're programming!
        // Set the timeout high as flash writes can take up to 1s
        self.socket
            .set_read_timeout(Some(Duration::from_secs_f32(1.5)))
            .unwrap();
        self.socket
            .set_write_timeout(Some(Duration::from_secs_f32(1.5)))
            .unwrap();
        // And we'll also set the retries higher
        let retries = 8;

        // The bitstream will start one tapcp::FLASH_SECTOR_SIZE away from the platform-specific
        // flash location. We don't care about recording the header and this makes the program
        // location consistent.
        // We have to write in chunks of FLASH_SECTOR_SIZE as well
        let bar = ProgressBar::new(
            (design.bitstream().len() as f64 / f64::from(tapcp::FLASH_SECTOR_SIZE)).ceil() as u64,
        );
        bar.set_message("Writting bitstream");
        for (idx, chunk) in design
            .bitstream()
            .chunks(tapcp::FLASH_SECTOR_SIZE as usize)
            .enumerate()
        {
            tapcp::write_flash(
                self.platform.program_location() as usize + tapcp::FLASH_SECTOR_SIZE as usize * idx,
                chunk,
                &mut self.socket,
                retries,
            )?;
            bar.inc(1);
        }
        bar.finish();
        // Then readback to verify
        // TODO

        // Set the metadata (to also indicate that we successfully programmed)
        self.update_metadata(design)?;

        // And reboot from the program location
        // We expect an error because the whole design will freeze up

        // Mystery bitshift
        tapcp::progdev(
            match self.platform {
                Platform::SNAP => self.platform.program_location() >> 8,
                Platform::SNAP2 => self.platform.program_location(),
            },
            &mut self.socket,
        )?;
        // Then wait as the FPGA takes a while to reboot
        std::thread::sleep(Duration::from_secs(1));
        Ok(())
    }

    fn deprogram(&mut self) -> anyhow::Result<()> {
        tapcp::progdev(0, &mut self.socket)
    }

    fn read_n_bytes(&mut self, device: &str, offset: usize, n: usize) -> anyhow::Result<Vec<u8>> {
        // TAPCP works on a block of size 4 bytes, so we need to do some chunking and slicing
        // The goal here is to be efficient, we don't want to query bytes we don't need.
        // The "worst case" is when we want to read bytes between words
        // i.e. If the device contains [1,2,3,4,5,6,7,8] and we want to read offset=2, N=3
        // Which is the last 2 bytes of the first word and the first byte of the second word.
        // In that case, we need to read both words.
        // First, grab enough multiple of 4 bytes
        let first_word = offset / 4;
        let last_word = (offset + n) / 4;
        let word_n = last_word - first_word;
        let bytes = tapcp::read_device(device, first_word, word_n, &mut self.socket, self.retries)?;
        // Now we slice out the the relevant chunk
        let start_idx = offset % 4;
        Ok(bytes[start_idx..start_idx + n].to_vec())
    }
}

// Tapcp-specific methods
impl Tapcp {
    /// Gets the temperature from the connected device in Celsius
    /// # Errors
    /// Returns errors on transport failures
    pub fn temperature(&mut self) -> anyhow::Result<f32> {
        tapcp::temp(&mut self.socket, self.retries)
    }

    /// Gets the metadata for the currently programed design
    /// # Errors
    /// Returns errors on transport failures
    pub fn metadata(&mut self) -> anyhow::Result<HashMap<KString, String>> {
        tapcp::get_metadata(
            &mut self.socket,
            self.platform.flash_location(),
            self.retries,
        )
    }

    /// Update the metadata entry given a design
    /// Currently not completley compatible with python as we only store the md5
    /// # Panics
    /// Panics if the filename of fpg file is not a valid rust string
    fn update_metadata<D>(&mut self, design: &D) -> anyhow::Result<()>
    where
        D: FpgaDesign,
    {
        let meta = HashMap::from([
            ("sector_size", tapcp::FLASH_SECTOR_SIZE.to_string()),
            ("md5", design.md5_string()),
        ])
        .into_iter()
        .map(|(k, v)| (k.into(), v))
        .collect();
        tapcp::set_metadata(
            &meta,
            &mut self.socket,
            self.platform.flash_location(),
            self.retries,
        )
    }
}

#[cfg(feature = "python")]
#[allow(clippy::pedantic)]
pub(crate) mod python {
    use super::*;
    use crate::transport::Transport;
    use anyhow::anyhow;
    use casper_utils::design_sources::fpg;
    use pyo3::{
        conversion::ToPyObject,
        prelude::*,
        types::PyBytes,
    };
    pub(crate) fn add_tapcp(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
        /// Transport via TAPCP - connects on construction.
        /// Requires a platform string, either "SNAP" or "SNAP2".
        #[pyclass(text_signature = "(ip)")]
        struct Tapcp(super::Tapcp);

        #[pymethods]
        impl Tapcp {
            #[new]
            fn new(ip: String, platform: String) -> PyResult<Self> {
                let plat = match platform.as_ref() {
                    "snap" => Platform::SNAP,
                    "snap2" => Platform::SNAP2,
                    _ => return Err(anyhow!("Unsupported platform").into()),
                };
                let inner = super::Tapcp::connect(ip.parse()?, plat)?;
                Ok(Tapcp(inner))
            }

            fn is_running(&mut self) -> PyResult<bool> {
                Ok(self.0.is_running()?)
            }

            #[pyo3(text_signature = "($self,device,n, offset)")]
            #[args(offset = "0")]
            fn read_bytes(
                &mut self,
                py: Python,
                device: &str,
                n: usize,
                offset: usize,
            ) -> PyResult<PyObject> {
                Ok(PyBytes::new(py, &self.0.read_n_bytes(device, offset, n)?).into())
            }

            #[pyo3(text_signature = "($self,device,offset)")]
            #[args(offset = "0")]
            fn read_int(&mut self, device: &str, offset: usize) -> PyResult<i32> {
                let val: i32 = self.0.read(device, offset)?;
                Ok(val)
            }

            #[pyo3(text_signature = "($self,device,offset)")]
            #[args(offset = "0")]
            fn read_float(&mut self, device: &str, offset: usize) -> PyResult<f32> {
                let val: f32 = self.0.read(device, offset)?;
                Ok(val)
            }

            #[pyo3(text_signature = "($self,device,offset)")]
            #[args(offset = "0")]
            fn read_bool(&mut self, device: &str, offset: usize) -> PyResult<bool> {
                let val: i32 = self.0.read(device, offset)?;
                Ok(val == 1)
            }

            #[pyo3(text_signature = "($self,device,n, offset)")]
            #[args(offset = "0")]
            fn write_bytes(
                &mut self,
                py: Python,
                bytes: Py<PyBytes>,
                device: String,
                offset: usize,
            ) -> PyResult<()> {
                let data = bytes.as_bytes(py);
                Ok(self.0.write_bytes(&device, offset, data)?)
            }

            #[pyo3(text_signature = "($self,device,offset)")]
            #[args(offset = "0")]
            fn write_int(&mut self, val: i32, device: &str, offset: usize) -> PyResult<()> {
                Ok(self.0.write(device, offset, &val)?)
            }

            #[pyo3(text_signature = "($self,device,offset)")]
            #[args(offset = "0")]
            fn write_float(&mut self, val: f32, device: &str, offset: usize) -> PyResult<()> {
                Ok(self.0.write(device, offset, &val)?)
            }

            #[pyo3(text_signature = "($self,device,offset)")]
            #[args(offset = "0")]
            fn write_bool(&mut self, val: bool, device: &str, offset: usize) -> PyResult<()> {
                Ok(self.0.write(device, offset, &(u32::from(val)))?)
            }

            #[pyo3(text_signature = "($self)")]
            fn listdev(&mut self, py: Python) -> PyResult<PyObject> {
                let devices: Vec<_> = self
                    .0
                    .listdev()?
                    .into_keys()
                    .map(|k| k.to_string())
                    .collect();
                Ok(devices.to_object(py))
            }

            #[pyo3(text_signature = "($self, filename)")]
            fn program(&mut self, filename: String, force: bool) -> PyResult<()> {
                let file = fpg::read_fpg_file(filename)?;
                Ok(self.0.program(&file, force)?)
            }

            #[pyo3(text_signature = "($self)")]
            fn deprogram(&mut self) -> PyResult<()> {
                Ok(self.0.deprogram()?)
            }

            #[pyo3(text_signature = "($self)")]
            fn temperature(&mut self) -> PyResult<f32> {
                Ok(self.0.temperature()?)
            }
        }

        m.add_class::<Tapcp>()?;
        Ok(())
    }
}
