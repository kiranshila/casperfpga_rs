mod csl;
pub mod tftp;

use anyhow::bail;
use std::{collections::HashMap, ffi::CStr, net::UdpSocket};
use tftp::Mode;

/// Gets the temperature of the remote device in Celsius
pub fn temp(socket: &mut UdpSocket) -> anyhow::Result<f32> {
    let bytes = tftp::read("/temp", socket, Mode::Octet)?;
    Ok(f32::from_be_bytes(bytes[..4].try_into()?))
}

/// Gets the list of top level commands (as a string)
pub fn help(socket: &mut UdpSocket) -> anyhow::Result<String> {
    let bytes = tftp::read("/help", socket, Mode::NetASCII)?;
    Ok(std::str::from_utf8(&bytes)?.to_string())
}

/// Gets the list of all devices supported by the currently running gateware
/// Returns a hash map from device name to (addr,length)
pub fn listdev(socket: &mut UdpSocket) -> anyhow::Result<HashMap<String, (u32, u32)>> {
    // Create the hash map we'll be constructing to hold the device list
    let mut dev_map = HashMap::new();

    let bytes = tftp::read("/listdev", socket, Mode::Octet)?;
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
        let key = unsafe { CStr::from_ptr(key_ptr as *const i8) }
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
pub fn read_device(
    device: &str,
    offset: usize,
    n: usize,
    socket: &mut UdpSocket,
) -> anyhow::Result<Vec<u8>> {
    // To start the request, we need to form the filename string, defined by the TAPCP
    // spec as - `/dev/DEV_NAME[.WORD_OFFSET[.NWORDS]]` with WORD_OFFSET and NWORDs in hexadecimal
    let filename = format!("/dev/{device}.{offset:x}.{n:x}");
    let bytes = tftp::read(&filename, socket, Mode::Octet)?;
    if n != 0 && bytes.len() != n * 4 {
        bail!("We did not receive the number of bytes we expected");
    }
    Ok(bytes)
}

/// Write bytes to the device named `device`
pub fn write_device(
    device: &str,
    offset: usize,
    data: &[u8],
    socket: &mut UdpSocket,
) -> anyhow::Result<()> {
    // To start the request, we need to form the filename string, defined by the TAPCP
    // spec as - `/dev/DEV_NAME[.WORD_OFFSET]` with WORD_OFFSET and NWORDs in hexadecimal
    let filename = format!("/dev/{device}.{offset:x}");
    // Then do it
    tftp::write(&filename, data, socket)
}

/// Read memory from the onboard flash
/// `offset` and `n` are in increments of 4 byte words, just like `read_device`
pub fn read_flash(offset: usize, n: usize, socket: &mut UdpSocket) -> anyhow::Result<Vec<u8>> {
    // spec as - `/flash.WORD_OFFSET[.NWORDS]` with WORD_OFFSET and NWORDs in hexadecimal
    let filename = format!("/flash.{offset:x}.{n:x}");
    let bytes = tftp::read(&filename, socket, Mode::Octet)?;
    Ok(bytes)
}
