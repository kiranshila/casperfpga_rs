#![deny(clippy::all)]
#![warn(clippy::pedantic)]

use casper_utils::csl;
use kstring::KString;
use std::{
    collections::HashMap,
    ffi::{
        c_char,
        c_uchar,
        CStr,
    },
    fmt::Write,
    net::UdpSocket,
    time::Duration,
};
use tftp_client::{
    download,
    upload,
};
use thiserror::Error;
use tracing::debug;

pub const FLASH_SECTOR_SIZE: u32 = 0x10000;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);
pub const MAX_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Tftp(#[from] tftp_client::Error),
    #[error("Some part of the received payload was incomplete")]
    Incomplete,
    #[error("While trying to parse a string from a response, we received invalid UTF8")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("No metadata returned when we requested metadata")]
    MissingMetadata,
    #[error(transparent)]
    Csl(#[from] csl::Error),
}

// The FPGA handles errors poorly, so when we try to move to quick (esp with sequential commands),
// we want to retry. We'll create wrappers around the tftp functions to retry on procotol errors,
// but bail on all others
fn retrying_download(
    filename: &str,
    socket: &UdpSocket,
    timeout: Duration,
    max_timeout: Duration,
    retries: usize,
) -> Result<Vec<u8>, Error> {
    let mut local_retries = 0;
    let mut this_timeout = timeout;
    loop {
        if local_retries == retries {
            return Err(Error::Tftp(tftp_client::Error::Timeout));
        }
        let res = download(filename, socket, timeout, max_timeout, retries);
        match res {
            Ok(v) => return Ok(v),
            Err(tftp_client::Error::Protocol { code, msg }) => {
                debug!("Protocol error: {:?} {msg}", code);
                std::thread::sleep(this_timeout);
                local_retries += 1;
                this_timeout += this_timeout / 2;
                if this_timeout > MAX_TIMEOUT {
                    this_timeout = MAX_TIMEOUT;
                }
                continue;
            }
            Err(e) => {
                return Err(Error::Tftp(e));
            }
        }
    }
}

fn retrying_upload(
    filename: &str,
    data: &[u8],
    socket: &UdpSocket,
    timeout: Duration,
    max_timeout: Duration,
    retries: usize,
) -> Result<(), Error> {
    let mut local_retries = 0;
    let mut this_timeout = timeout;
    loop {
        if local_retries == retries {
            return Err(Error::Tftp(tftp_client::Error::Timeout));
        }
        let res = upload(filename, data, socket, timeout, max_timeout, retries);
        match res {
            Ok(()) => return Ok(()),
            Err(tftp_client::Error::Protocol { code, msg }) => {
                debug!("Protocol error: {:?} {msg}", code);
                local_retries += 1;
                std::thread::sleep(this_timeout);
                local_retries += 1;
                this_timeout += this_timeout / 2;
                if this_timeout > MAX_TIMEOUT {
                    this_timeout = MAX_TIMEOUT;
                }
                continue;
            }
            Err(e) => {
                return Err(Error::Tftp(e));
            }
        }
    }
}

/// Gets the temperature of the remote device in Celsius
/// # Errors
/// Returns an error on TFTP errors
/// # Panics
/// Panics if we did not get back enough bytes
pub fn temp(socket: &UdpSocket, retries: usize) -> Result<f32, Error> {
    let bytes = retrying_download("/temp", socket, DEFAULT_TIMEOUT, MAX_TIMEOUT, retries)?;
    Ok(f32::from_be_bytes(
        bytes[..4].try_into().map_err(|_| Error::Incomplete)?,
    ))
}

/// Gets the list of top level commands (as a string)
/// # Errors
/// Returns an error on TFTP errors
pub fn help(socket: &UdpSocket, retries: usize) -> Result<String, Error> {
    let bytes = retrying_download("/help", socket, DEFAULT_TIMEOUT, MAX_TIMEOUT, retries)?;
    Ok(std::str::from_utf8(&bytes)?.to_string())
}

/// Gets the list of all devices supported by the currently running gateware
/// Returns a hash map from device name to (addr,length)
/// # Errors
/// Returns an error on TFTP errors
pub fn listdev(socket: &UdpSocket, retries: usize) -> Result<HashMap<String, (u32, u32)>, Error> {
    // Grab CSL bytes
    let bytes = retrying_download("/listdev", socket, DEFAULT_TIMEOUT, MAX_TIMEOUT, retries)?;
    // Unpack CSL
    let csl = csl::from_bytes(&bytes)?;
    // Translate into our device map
    csl.into_iter()
        .map(|(k, v)| {
            // Value should be exactly 8 bytes
            // First 4 is offset, second is length
            let addr = u32::from_be_bytes(v[..4].try_into().map_err(|_| Error::Incomplete)?);
            let length = u32::from_be_bytes(v[4..].try_into().map_err(|_| Error::Incomplete)?);
            Ok((k, (addr, length)))
        })
        .collect()
}

/// Read memory associated with the gateware device `device`
/// We can read `offset` words (4 bytes) into a given device in multiples on `n` words
/// The special case of `n` = 0 will read all the bytes at that location
/// # Errors
/// Returns an error on TFTP errors
pub fn read_device(
    device: &str,
    offset: usize,
    n: usize,
    socket: &UdpSocket,
    retries: usize,
) -> Result<Vec<u8>, Error> {
    // To start the request, we need to form the filename string, defined by the TAPCP
    // spec as - `/dev/DEV_NAME[.WORD_OFFSET[.NWORDS]]` with WORD_OFFSET and NWORDs in hexadecimal
    let filename = format!("/dev/{device}.{offset:x}.{n:x}");
    let bytes = retrying_download(&filename, socket, DEFAULT_TIMEOUT, MAX_TIMEOUT, retries)?;
    if n != 0 && bytes.len() != n * 4 {
        Err(Error::Incomplete)
    } else {
        Ok(bytes)
    }
}

/// Write bytes to the device named `device`
/// # Errors
/// Returns an error on TFTP errors
pub fn write_device(
    device: &str,
    offset: usize,
    data: &[u8],
    socket: &UdpSocket,
    retries: usize,
) -> Result<(), Error> {
    // To start the request, we need to form the filename string, defined by the TAPCP
    // spec as - `/dev/DEV_NAME[.WORD_OFFSET]` with WORD_OFFSET and NWORDs in hexadecimal
    let filename = format!("/dev/{device}.{offset:x}");
    // Then do it
    retrying_upload(
        &filename,
        data,
        socket,
        DEFAULT_TIMEOUT,
        MAX_TIMEOUT,
        retries,
    )
}

/// Read memory from the onboard flash
/// `offset` and `n` are in increments of 4 byte words, just like `read_device`
/// # Errors
/// Returns an error on TFTP errors
pub fn read_flash(
    offset: usize,
    n: usize,
    socket: &UdpSocket,
    retries: usize,
) -> Result<Vec<u8>, Error> {
    // spec as - `/flash.WORD_OFFSET[.NWORDS]` with WORD_OFFSET and NWORDs in hexadecimal
    let filename = format!("/flash.{offset:x}.{n:x}");
    let bytes = retrying_download(&filename, socket, DEFAULT_TIMEOUT, MAX_TIMEOUT, retries)?;
    Ok(bytes)
}

/// Writes data to the onboard flash
/// `offset` are in increments of 4 byte words, just like `read_device`
/// # Errors
/// Returns an error on TFTP errors
pub fn write_flash(
    offset: usize,
    data: &[u8],
    socket: &UdpSocket,
    retries: usize,
) -> Result<(), Error> {
    let filename = format!("/flash.{offset:x}");
    retrying_upload(
        &filename,
        data,
        socket,
        DEFAULT_TIMEOUT,
        MAX_TIMEOUT,
        retries,
    )
}

/// Reboot the FPGA from the bitstream program at the 32-bit address `addr`.
/// No validation is performed to ensure a program actually exists there
/// # Errors
/// Returns an error on TFTP errors
pub fn progdev(addr: u32, socket: &UdpSocket) -> Result<(), Error> {
    match upload(
        "/progdev",
        &addr.to_be_bytes(),
        socket,
        DEFAULT_TIMEOUT,
        MAX_TIMEOUT,
        0,
    ) {
        Ok(()) | Err(_) => (),
    }
    // Then wait as the FPGA takes a while to reboot
    std::thread::sleep(Duration::from_secs(10));
    Ok(())
}

/// Retrieves the most recent metadata (stored at the 32-bit `user_flash_loc` address)
/// # Errors
/// Returns an error on TFTP errors or if the metadata couldn't be found
pub fn get_metadata(
    socket: &UdpSocket,
    user_flash_loc: u32,
    retries: usize,
) -> Result<HashMap<KString, String>, Error> {
    let mut dict_str = String::new();
    let mut chunks = 0;
    let chunk_size = 1024 / 4;
    loop {
        if chunks > 128 {
            return Err(Error::MissingMetadata);
        }
        let raw = read_flash(
            (user_flash_loc / 4 + chunks * chunk_size) as usize,
            chunk_size as usize,
            socket,
            retries,
        )?;
        dict_str.push_str(std::str::from_utf8(&raw)?);
        match dict_str.find("?end") {
            Some(idx) => {
                dict_str = dict_str.split_at(idx).0.to_string();
                break;
            }
            None => chunks += 1,
        }
    }
    Ok(dict_str
        .split('?')
        .filter_map(|kv| kv.split_once('\t'))
        .map(|(k, v)| (k.to_string().into(), v.to_string()))
        .collect())
}

/// Program arbitrary metadata (stored at the 32-bit `user_flash_loc` address)
/// # Errors
/// Returns an error on TFTP errors or if the metadata couldn't be found
#[allow(clippy::implicit_hasher)]
pub fn set_metadata(
    data: &HashMap<KString, String>,
    socket: &UdpSocket,
    user_flash_loc: u32,
    retries: usize,
) -> Result<(), Error> {
    // Dict is written as ?<key>\t<value> pairs followed by ?end
    // It must be padded with zeros to be a multiple of 1024
    let mut dict_str = data.iter().fold(String::new(), |mut output, (k, v)| {
        let _ = write!(output, "?{k}\t{v}");
        output
    });
    dict_str.push_str("?end");
    let mut bytes = dict_str.as_bytes().to_vec();
    // Padding
    if bytes.len() % 1024 != 0 {
        bytes.append(&mut vec![b'0'; 1024 - bytes.len() % 1024]);
    }
    // Write
    write_flash((user_flash_loc / 4) as usize, &bytes, socket, retries)
}
