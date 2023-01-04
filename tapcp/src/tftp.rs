//! An IO-agnostic implementation of the TFTP standard (Revision 2), as defined by RFC 1350.
//! Only the client is implemented here, because that's all we care about.
//! The TFTP servers that TAPCP clients are running do not support the RFC 2348
//! Blocksize Option, so all data blocks must be 512 bytes or fewer.

use std::{
    fmt::Display,
    net::UdpSocket,
    str::FromStr,
    time::Duration,
};

use anyhow::bail;
use num_derive::{
    FromPrimitive,
    ToPrimitive,
};
use num_traits::{
    FromPrimitive,
    ToPrimitive,
};

const MAX_DATA: usize = 512;

#[derive(Debug, Copy, Clone)]
pub(crate) enum Mode {
    NetASCII,
    Octet,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Mode::NetASCII => "netascii",
                Mode::Octet => "octet",
            }
        )
    }
}

impl FromStr for Mode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "netascii" => Mode::NetASCII,
            "octet" => Mode::Octet,
            _ => return Err(Error::BadMode(s.to_owned())),
        })
    }
}

#[derive(thiserror::Error, Debug, FromPrimitive, ToPrimitive)]
pub enum ErrorCode {
    #[error("Not defined, see error message (if any)")]
    NotDefined = 0,
    #[error("File not found")]
    NotFound = 1,
    #[error("Access violation")]
    AccessViolation = 2,
    #[error("Disk full or allocation exceeded")]
    Full = 3,
    #[error("Illegal TFTP operation")]
    IllegalOp = 4,
    #[error("Unknown transfer ID")]
    UnknownID = 5,
    #[error("File already exists")]
    FileExists = 6,
    #[error("No such user")]
    NoUser = 7,
}

/// Errors that can be thrown from TFTP interactions
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("We recieved an error response back: {0} - {1}")]
    ErrorResponse(ErrorCode, String),
    #[error("We expected a mode string, but got back something invalid: {0}")]
    BadMode(String),
    #[error("Not enough bytes in the payload")]
    Incomplete,
    #[error("We didn't get a valid op code")]
    BadOpcode,
    #[error("The error code wasn't one we know about")]
    BadErrorCode,
    #[error("We didn't get back a block number we expected: {0}")]
    BadBlock(u16),
    #[error("Retry count exceeded")]
    Timeout,
}

#[derive(Debug)]
pub(crate) enum Payload {
    Read {
        filename: String,
        mode: Mode,
    },
    Write {
        filename: String,
        mode: Mode,
    },
    Data {
        block: u16,
        data: Vec<u8>,
    },
    Ack {
        block: u16,
    },
    Error {
        error_code: ErrorCode,
        error_msg: String,
    },
}

#[allow(clippy::cast_precision_loss)]
fn backoff_read(socket: &mut UdpSocket, buf: &mut [u8], retries: usize) -> anyhow::Result<usize> {
    let mut retry_count = 0usize;
    let nbytes;
    loop {
        if retry_count == retries {
            bail!(Error::Timeout);
        }
        std::thread::sleep(Duration::from_secs_f64(
            (2f64.powf(retry_count as f64) - 1f64) / 1000f64,
        ));
        match socket.recv(buf) {
            Ok(v) => {
                nbytes = v;
                break;
            }
            Err(e) => match e.kind() {
                // Compat for both windows and *nix
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => retry_count += 1,
                _ => return Err(e.into()),
            },
        }
    }
    Ok(nbytes)
}

#[allow(clippy::cast_precision_loss)]
fn backoff_write(socket: &mut UdpSocket, data: &[u8], retries: usize) -> anyhow::Result<()> {
    let mut retry_count = 0usize;
    loop {
        if retry_count == retries {
            bail!(Error::Timeout);
        }
        std::thread::sleep(Duration::from_secs_f64(
            (2f64.powf(retry_count as f64) - 1f64) / 1000f64,
        ));
        match socket.send(data) {
            Ok(_) => break,
            Err(e) => match e.kind() {
                // Compat for both windows and *nix
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => retry_count += 1,
                _ => return Err(e.into()),
            },
        }
    }
    Ok(())
}

impl Payload {
    /// Take an instance of a TFTP payload, and construct the byte payload to send over UDP
    fn pack(&self) -> Vec<u8> {
        let mut bytes = vec![];
        if let Payload::Read { filename, mode } = self {
            bytes.extend_from_slice(&1u16.to_be_bytes());
            bytes.extend_from_slice(filename.as_bytes());
            bytes.push(0u8);
            bytes.extend_from_slice(mode.to_string().as_bytes());
            bytes.push(0u8);
        } else if let Payload::Write { filename, mode } = self {
            bytes.extend_from_slice(&2u16.to_be_bytes());
            bytes.extend_from_slice(filename.as_bytes());
            bytes.push(0u8);
            bytes.extend_from_slice(mode.to_string().as_bytes());
            bytes.push(0u8);
        } else if let Payload::Data { block, data } = self {
            bytes.extend_from_slice(&3u16.to_be_bytes());
            bytes.extend_from_slice(&block.to_be_bytes());
            bytes.extend_from_slice(data);
        } else if let Payload::Ack { block } = self {
            bytes.extend_from_slice(&4u16.to_be_bytes());
            bytes.extend_from_slice(&block.to_be_bytes());
        } else if let Payload::Error {
            error_code,
            error_msg,
        } = self
        {
            bytes.extend_from_slice(&5u16.to_be_bytes());
            bytes.extend_from_slice(
                &(error_code.to_u16().expect("This will always fit in a u16")).to_be_bytes(),
            );
            bytes.extend_from_slice(error_msg.as_bytes());
            bytes.push(0);
        }

        bytes
    }

    /// Given bytes from UDP, construct an instance of a TFTP payload
    fn unpack(bytes: &[u8]) -> anyhow::Result<Self> {
        // The smallest this can be is 4 bytes (ACK), so if it's less than that, bail
        if bytes.len() < 4 {
            bail!(Error::Incomplete);
        }
        // First two bytes determine the op code
        let opcode = u16::from_be_bytes(
            bytes[0..2]
                .try_into()
                .expect("We've already checked that it will have these"),
        );
        // "Consume" the bytes we've used by shadowing the slice
        let bytes = &bytes[2..];
        Ok(match opcode {
            // Read
            1 | 2 => {
                let filename_null_idx = bytes
                    .iter()
                    .position(|&c| c == b'\0')
                    .ok_or(Error::Incomplete)?;
                let filename = std::str::from_utf8(&bytes[..filename_null_idx])?.to_string();
                // Consume more bytes, skipping the null
                let bytes = &bytes[(filename_null_idx + 1)..];
                // One more C string for the mode
                let mode_null_idx = bytes
                    .iter()
                    .position(|&c| c == b'\0')
                    .ok_or(Error::Incomplete)?;
                let mode_str = std::str::from_utf8(&bytes[..mode_null_idx])?;
                let mode = Mode::from_str(mode_str)?;
                if opcode == 1 {
                    Payload::Read { filename, mode }
                } else {
                    Payload::Write { filename, mode }
                }
            }
            // Data
            3 => {
                let block = u16::from_be_bytes(bytes[..2].try_into()?);
                let data = bytes[2..].to_vec();
                Payload::Data { block, data }
            }
            4 => {
                let block = u16::from_be_bytes(bytes[..2].try_into()?);
                Payload::Ack { block }
            }
            5 => {
                let raw_err_code = u16::from_be_bytes(bytes[..2].try_into()?);
                let error_code = ErrorCode::from_u16(raw_err_code).ok_or(Error::BadErrorCode)?;
                // Consume more bytes, skipping the null
                let bytes = &bytes[2..];
                let err_null_idx = bytes
                    .iter()
                    .position(|&c| c == b'\0')
                    .ok_or(Error::Incomplete)?;
                let error_msg = std::str::from_utf8(&bytes[..err_null_idx])?.to_string();
                Payload::Error {
                    error_code,
                    error_msg,
                }
            }
            _ => bail!(Error::BadOpcode),
        })
    }
}

/// Read from a filename and get back the bytes
pub(crate) fn read(
    filename: &str,
    socket: &mut UdpSocket,
    mode: Mode,
    retries: usize,
) -> anyhow::Result<Vec<u8>> {
    // Create the buffer we will use to read into. The biggest this can be is 512 bytes of data,
    // plus 4 bytes of header
    let mut buf = [0u8; 516];
    // And the output vector we'll store bytes in
    let mut output = vec![];
    // We start the transfer with the RRQ (read request) and will recieve the first data packet
    let rrq = Payload::Read {
        filename: filename.to_string(),
        mode,
    };
    // Write out this payload with retries
    backoff_write(socket, &rrq.pack(), retries)?;

    loop {
        // Read and deserialize the responses with retries and backoff
        let nbytes = backoff_read(socket, &mut buf, retries)?;
        let resp = Payload::unpack(&buf[..nbytes])?;
        match resp {
            Payload::Data { block, ref data } => {
                // Copy out the bytes
                output.extend_from_slice(data);
                // Send the ACK
                let ack = Payload::Ack { block };
                socket.send(&ack.pack())?;
                // Check end of data condition
                if data.len() < MAX_DATA {
                    break;
                }
            }
            Payload::Error {
                error_code,
                error_msg,
            } => bail!(Error::ErrorResponse(error_code, error_msg)),
            _ => unreachable!(),
        }
    }
    Ok(output)
}

/// Write the bytes from `data` to the TFTP server at `filename`
pub(crate) fn write(
    filename: &str,
    data: &[u8],
    socket: &mut UdpSocket,
    retries: usize,
) -> anyhow::Result<()> {
    // We start the transfer with the WRQ (write request)
    let wrq = Payload::Write {
        filename: filename.to_string(),
        mode: Mode::Octet,
    };
    // Create the buffer we will use to read into.
    // We will only ever get back ACK or error messages.
    // In the case of error messages, we will limit the string payload to 512 bytes, like the data
    let mut buf = [0u8; 516];
    // Write out this payload
    backoff_write(socket, &wrq.pack(), retries)?;
    // We should receive either an ACK of block 0 or ERROR
    let nbytes = backoff_read(socket, &mut buf, retries)?;
    match Payload::unpack(&buf[..nbytes])? {
        Payload::Ack { block } => {
            if block != 0 {
                bail!(Error::BadBlock(block))
            }
        }
        Payload::Error {
            error_code,
            error_msg,
        } => bail!(Error::ErrorResponse(error_code, error_msg)),
        _ => unreachable!(),
    }
    // Assuming we survived this, we can start actually sending the data
    for (i, chunk) in data.chunks(MAX_DATA).enumerate() {
        // Send the (i+1)th chunk (because chunks are 1-indexed)
        // Prepare the data payload
        let data_payload = Payload::Data {
            block: (i + 1).try_into().expect("i+1 didn't fit in a u16"),
            data: chunk.to_vec(),
        };
        // Send
        backoff_write(socket, &data_payload.pack(), retries)?;
        // Wait for the ACK
        let nbytes = backoff_read(socket, &mut buf, retries)?;
        // Which should match the index we just sent
        match Payload::unpack(&buf[..nbytes])? {
            Payload::Ack { block } => {
                if block as usize != i + 1 {
                    bail!(Error::BadBlock(block))
                }
            }
            Payload::Error {
                error_code,
                error_msg,
            } => bail!(Error::ErrorResponse(error_code, error_msg)),
            _ => unreachable!(),
        }
    }
    // If we survived this, then we've sent everything we needed to!
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_read() {
        let payload = Payload::Read {
            filename: "/foo".to_owned(),
            mode: Mode::Octet,
        };
        let packed = payload.pack();
        assert_eq!(
            packed,
            vec![0, 1, b'/', b'f', b'o', b'o', 0, b'o', b'c', b't', b'e', b't', 0]
        );
    }

    #[test]
    fn test_pack_write() {
        let payload = Payload::Write {
            filename: "/foo".to_owned(),
            mode: Mode::Octet,
        };
        let packed = payload.pack();
        assert_eq!(
            packed,
            vec![0, 2, b'/', b'f', b'o', b'o', 0, b'o', b'c', b't', b'e', b't', 0]
        );
    }

    #[test]
    fn test_pack_data() {
        let payload = Payload::Data {
            block: 1,
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
        let packed = payload.pack();
        assert_eq!(packed, vec![0, 3, 0, 1, 0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_pack_ack() {
        let payload = Payload::Ack { block: 1 };
        let packed = payload.pack();
        assert_eq!(packed, vec![0, 4, 0, 1]);
    }

    #[test]
    fn test_pack_error() {
        let payload = Payload::Error {
            error_code: ErrorCode::Full,
            error_msg: "Full".to_owned(),
        };
        let packed = payload.pack();
        assert_eq!(packed, vec![0, 5, 0, 3, b'F', b'u', b'l', b'l', 0]);
    }

    #[test]
    fn test_roundtrip_read() {
        let payload = vec![
            0, 1, b'/', b'f', b'o', b'o', 0, b'o', b'c', b't', b'e', b't', 0,
        ];
        assert_eq!(payload, Payload::unpack(&payload).unwrap().pack());
    }

    #[test]
    fn test_roundtrip_write() {
        let payload = vec![
            0, 2, b'/', b'f', b'o', b'o', 0, b'o', b'c', b't', b'e', b't', 0,
        ];
        assert_eq!(payload, Payload::unpack(&payload).unwrap().pack());
    }

    #[test]
    fn test_roundtrip_data() {
        let payload = vec![0, 3, 0, 1, 0xDE, 0xAD, 0xBE, 0xEF];
        assert_eq!(payload, Payload::unpack(&payload).unwrap().pack());
    }

    #[test]
    fn test_roundtrip_ack() {
        let payload = vec![0, 4, 0, 1];
        assert_eq!(payload, Payload::unpack(&payload).unwrap().pack());
    }

    #[test]
    fn test_roundtrip_error() {
        let payload = vec![0, 5, 0, 3, b'F', b'u', b'l', b'l', 0];
        assert_eq!(payload, Payload::unpack(&payload).unwrap().pack());
    }
}
