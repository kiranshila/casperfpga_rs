//! The casperfpga transport implementations for TAPCP
use super::{
    Transport,
    TransportResult,
};
use crate::core::{
    Register,
    RegisterMap,
};
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
use thiserror::Error;

const DEFAULT_TIMEOUT: f32 = 0.5;
const DEFAULT_RETRIES: usize = 5;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Internal system IO error")]
    Io(#[from] std::io::Error),
    #[error("Error from the lower-level TAPCP library")]
    Lower(#[from] tapcp::Error),
}

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
    pub fn connect(host: SocketAddr, platform: Platform) -> TransportResult<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(Error::from)?;
        // Set explicit nonblocking
        socket.set_nonblocking(false).map_err(Error::from)?;
        // Set a default timeout
        let timeout = Duration::from_secs_f32(DEFAULT_TIMEOUT);
        socket
            .set_write_timeout(Some(timeout))
            .map_err(Error::from)?;
        socket
            .set_read_timeout(Some(timeout))
            .map_err(Error::from)?;
        // Connect
        socket.connect(host).map_err(Error::from)?;
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
    fn is_running(&mut self) -> TransportResult<bool> {
        // Check if sys_clkcounter exists
        match tapcp::read_device("sys_clkcounter", 0, 1, &self.socket, self.retries) {
            Ok(_) => Ok(true),
            // In the case we get back a file not found error,
            // that implies the device is not running a user program.
            // Any other error is actually an error
            Err(e) => match e {
                tapcp::Error::Tftp(tftp_client::Error::Protocol {
                    code: tftp_client::parser::ErrorCode::NoFile,
                    msg: _,
                }) => Ok(false),
                _ => Err(Error::Lower(e).into()),
            },
        }
    }

    fn write_bytes(&mut self, device: &str, offset: usize, data: &[u8]) -> TransportResult<()> {
        // The inverted version of `read_vec`. The problem here is if we are not writing a 4 byte
        // chunk (which we need to), we have to read the bytes that are already there and include
        // them. Because we don't want to do this read when we don't have to, we will branch
        if (offset % 4) == 0 && (data.len() % 4) == 0 {
            // Just do the write
            tapcp::write_device(device, offset % 4, data, &self.socket, self.retries)
                .map_err(Error::from)?;
        } else {
            unimplemented!()
        }
        Ok(())
    }

    fn listdev(&mut self) -> TransportResult<RegisterMap> {
        let devices = tapcp::listdev(&self.socket, self.retries).map_err(Error::from)?;
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
    fn program<D>(&mut self, design: &D, force: bool) -> TransportResult<()>
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
                &self.socket,
                retries,
            )
            .map_err(Error::from)?;
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
            &self.socket,
        )
        .map_err(Error::from)?;
        Ok(())
    }

    fn deprogram(&mut self) -> TransportResult<()> {
        Ok(tapcp::progdev(0, &self.socket).map_err(Error::from)?)
    }

    fn read_n_bytes(&mut self, device: &str, offset: usize, n: usize) -> TransportResult<Vec<u8>> {
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
        let bytes = tapcp::read_device(device, first_word, word_n, &self.socket, self.retries)
            .map_err(Error::from)?;
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
    pub fn temperature(&mut self) -> Result<f32, Error> {
        Ok(tapcp::temp(&self.socket, self.retries)?)
    }

    /// Gets the metadata for the currently programed design
    /// # Errors
    /// Returns errors on transport failures
    pub fn metadata(&mut self) -> Result<HashMap<KString, String>, Error> {
        Ok(tapcp::get_metadata(
            &self.socket,
            self.platform.flash_location(),
            self.retries,
        )?)
    }

    /// Update the metadata entry given a design
    /// Currently not completley compatible with python as we only store the md5
    /// # Panics
    /// Panics if the filename of fpg file is not a valid rust string
    fn update_metadata<D>(&mut self, design: &D) -> Result<(), Error>
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
        Ok(tapcp::set_metadata(
            &meta,
            &self.socket,
            self.platform.flash_location(),
            self.retries,
        )?)
    }
}
