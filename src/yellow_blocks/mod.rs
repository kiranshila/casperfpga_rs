pub mod clockswitch;
pub mod snapadc;
pub mod ten_gbe;

use nom::{
    bytes::complete::{tag, take_till},
    character::{
        complete::{line_ending, space1},
        is_space,
    },
    number::complete::hex_u32,
    sequence::{preceded, terminated, tuple},
    IResult,
};
use std::collections::HashMap;

type FpgDevices = HashMap<String, FpgDevice>;

#[derive(Debug)]
struct FpgDevice {
    kind: Option<String>, // FIXME to use enum
    addr: Option<usize>,
    size: Option<usize>,
    metadata: HashMap<String, String>,
}

#[derive(Debug)]
pub struct FpgFile {
    /// The name of the source FPG file
    filename: String,
    /// The extra metadata
    devices: FpgDevices,
}

// Parser

fn shebang(input: &[u8]) -> IResult<&[u8], &[u8]> {
    terminated(tag("#!/bin/kcpfg"), line_ending)(input)
}

fn uploadbin(input: &[u8]) -> IResult<&[u8], &[u8]> {
    terminated(tag("?uploadbin"), line_ending)(input)
}

fn register<'a>(input: &'a [u8], devices: &mut FpgDevices) -> IResult<&'a [u8], ()> {
    let (rest, _) = tag("?register")(input)?;
    let (rest, name) = preceded(space1, take_till(is_space))(rest)?;
    let (rest, addr) = preceded(tuple((space1, tag("0x"))), hex_u32)(rest)?;
    let (rest, size) = preceded(tuple((space1, tag("0x"))), hex_u32)(rest)?;
    let (rest, _) = line_ending(rest)?;

    // These will all be "new" devices
    let name = std::str::from_utf8(name).unwrap(); // FIXME
    let dev = FpgDevice {
        kind: None,
        addr: Some(addr as usize),
        size: Some(size as usize),
        metadata: HashMap::new(),
    };

    // Insert into device map
    devices.insert(name.to_owned(), dev);

    // And return the rest
    Ok((rest, ()))
}
