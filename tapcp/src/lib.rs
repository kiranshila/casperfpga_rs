#![deny(clippy::all)]
#![warn(clippy::pedantic)]

mod csl;
pub mod tftp;

use anyhow::bail;
use kstring::KString;
use std::{
    collections::HashMap,
    ffi::CStr,
    net::UdpSocket,
};
use tftp::Mode;

pub const FLASH_SECTOR_SIZE: u32 = 0x10000;

/// Gets the temperature of the remote device in Celsius
/// # Errors
/// Returns an error on TFTP errors
/// # Panics
/// Panics if we did not get back enough bytes
pub fn temp(socket: &mut UdpSocket, retries: usize) -> anyhow::Result<f32> {
    let bytes = tftp::read("/temp", socket, Mode::Octet, retries)?;
    Ok(f32::from_be_bytes(bytes[..4].try_into()?))
}

/// Gets the list of top level commands (as a string)
/// # Errors
/// Returns an error on TFTP errors
pub fn help(socket: &mut UdpSocket, retries: usize) -> anyhow::Result<String> {
    let bytes = tftp::read("/help", socket, Mode::NetASCII, retries)?;
    Ok(std::str::from_utf8(&bytes)?.to_string())
}

/// Gets the list of all devices supported by the currently running gateware
/// Returns a hash map from device name to (addr,length)
/// # Errors
/// Returns an error on TFTP errors
pub fn listdev(
    socket: &mut UdpSocket,
    retries: usize,
) -> anyhow::Result<HashMap<String, (u32, u32)>> {
    // Create the hash map we'll be constructing to hold the device list
    let mut dev_map = HashMap::new();

    let bytes = tftp::read("/listdev", socket, Mode::Octet, retries)?;
    // Bytes back from this are stored as CSL, so we'll use Dave's C program to uncompress it
    // The CSL lib has internal state for some reason

    // The first two bytes are the length, but we don't care because that's part of the UDP payload
    // Safety: bytes is valid at this point because it's rust memory
    unsafe { csl::csl_iter_init(bytes[2..].as_ptr()) }

    // Now, we have to use the CSL iterator to traverse the list
    // Create a ptr to null that will be updated by `csl_iter_next`
    let mut key_ptr = std::ptr::null();

    loop {
        // Safety: key_ptr is valid because it's rust memory
        let value_ptr = unsafe { csl::csl_iter_next(&mut key_ptr) };

        if value_ptr.is_null() {
            break;
        }

        // Now key *should* be valid
        // Safety: We're trusting Dave gives us ptrs to valid ASCII
        // and we can safely reinterpret the *const u8 and *const i8 because they share a size
        let key = unsafe { CStr::from_ptr(key_ptr.cast::<i8>()) }
            .to_str()?
            .into();

        // Safety: The "spec" says this will be 8 bytes
        let value = unsafe { std::slice::from_raw_parts(value_ptr, 8) };

        // The first 4 byte word is the offset (address) and the second is the length
        let addr = u32::from_be_bytes(value[..4].try_into()?);
        let length = u32::from_be_bytes(value[4..].try_into()?);

        // Finally, push this all to our hash map
        dev_map.insert(key, (addr, length));
    }
    Ok(dev_map)
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
    socket: &mut UdpSocket,
    retries: usize,
) -> anyhow::Result<Vec<u8>> {
    // To start the request, we need to form the filename string, defined by the TAPCP
    // spec as - `/dev/DEV_NAME[.WORD_OFFSET[.NWORDS]]` with WORD_OFFSET and NWORDs in hexadecimal
    let filename = format!("/dev/{device}.{offset:x}.{n:x}");
    let bytes = tftp::read(&filename, socket, Mode::Octet, retries)?;
    if n != 0 && bytes.len() != n * 4 {
        bail!("We did not receive the number of bytes we expected");
    }
    Ok(bytes)
}

/// Write bytes to the device named `device`
/// # Errors
/// Returns an error on TFTP errors
pub fn write_device(
    device: &str,
    offset: usize,
    data: &[u8],
    socket: &mut UdpSocket,
    retries: usize,
) -> anyhow::Result<()> {
    // To start the request, we need to form the filename string, defined by the TAPCP
    // spec as - `/dev/DEV_NAME[.WORD_OFFSET]` with WORD_OFFSET and NWORDs in hexadecimal
    let filename = format!("/dev/{device}.{offset:x}");
    // Then do it
    tftp::write(&filename, data, socket, retries)
}

/// Read memory from the onboard flash
/// `offset` and `n` are in increments of 4 byte words, just like `read_device`
/// # Errors
/// Returns an error on TFTP errors
pub fn read_flash(
    offset: usize,
    n: usize,
    socket: &mut UdpSocket,
    retries: usize,
) -> anyhow::Result<Vec<u8>> {
    // spec as - `/flash.WORD_OFFSET[.NWORDS]` with WORD_OFFSET and NWORDs in hexadecimal
    let filename = format!("/flash.{offset:x}.{n:x}");
    let bytes = tftp::read(&filename, socket, Mode::Octet, retries)?;
    Ok(bytes)
}

/// Writes data to the onboard flash
/// `offset` are in increments of 4 byte words, just like `read_device`
/// # Errors
/// Returns an error on TFTP errors
pub fn write_flash(
    offset: usize,
    data: &[u8],
    socket: &mut UdpSocket,
    retries: usize,
) -> anyhow::Result<()> {
    let filename = format!("/flash.{offset:x}");
    tftp::write(&filename, data, socket, retries)
}

/// Reboot the FPGA from the bitstream program at the 32-bit address `addr`.
/// No validation is performed to ensure a program actually exists there
/// # Errors
/// Returns an error on TFTP errors
pub fn progdev(addr: u32, socket: &mut UdpSocket, retries: usize) -> anyhow::Result<()> {
    tftp::write("/progdev", &addr.to_be_bytes(), socket, retries)?;
    Ok(())
}

/// Retrieves the most recent metadata (stored at the 32-bit `user_flash_loc` address)
/// # Errors
/// Returns an error on TFTP errors or if the metadata couldn't be found
pub fn get_metadata(
    socket: &mut UdpSocket,
    user_flash_loc: u32,
    retries: usize,
) -> anyhow::Result<HashMap<KString, String>> {
    let mut dict_str = String::new();
    let mut chunks = 0;
    let chunk_size = 1024 / 4;
    loop {
        if chunks > 128 {
            bail!("No metadata found");
        }
        let raw = read_flash(
            (user_flash_loc / 4 + chunks * chunk_size) as usize,
            chunk_size as usize,
            socket,
            retries,
        )?;
        dict_str.push_str(&String::from_utf8(raw)?);
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
    socket: &mut UdpSocket,
    user_flash_loc: u32,
    retries: usize,
) -> anyhow::Result<()> {
    // Dict is written as ?<key>\t<value> pairs followed by ?end
    // It must be padded with zeros to be a multiple of 1024
    let mut dict_str = data
        .iter()
        .map(|(k, v)| format!("?{k}\t{v}"))
        .collect::<String>();
    dict_str.push_str("?end");
    let mut bytes = dict_str.as_bytes().to_vec();
    // Padding
    if bytes.len() % 1024 != 0 {
        bytes.append(&mut vec![b'0'; 1024 - bytes.len() % 1024]);
    }
    // Write
    write_flash((user_flash_loc / 4) as usize, &bytes, socket, retries)
}
