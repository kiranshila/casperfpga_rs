//! This module contains the logic for parsing and interpreting the CASPER-Specific FPG files
use anyhow::anyhow;
use kstring::KString;
use nom::{
    bytes::complete::{
        tag,
        take_till,
    },
    character::{
        complete::{
            hex_digit1,
            line_ending,
            not_line_ending,
            space1,
        },
        is_space,
    },
    combinator::map_res,
    multi::many0,
    sequence::{
        preceded,
        terminated,
    },
    IResult,
};
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::Path,
    str::from_utf8,
};

#[derive(Debug, PartialEq, Eq)]
pub struct FpgRegister {
    pub addr: u32,
    pub size: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FpgDevice {
    pub kind: String,
    pub register: Option<FpgRegister>,
    pub metadata: HashMap<KString, String>,
}

impl FpgDevice {
    fn add_meta(&mut self, k: KString, v: String) {
        self.metadata.insert(k, v);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FpgFile {
    pub devices: HashMap<KString, FpgDevice>,
    pub bitstream: Vec<u8>,
}

fn shebang(input: &[u8]) -> IResult<&[u8], &[u8]> {
    terminated(tag("#!/bin/kcpfpg"), line_ending)(input)
}

fn uploadbin(input: &[u8]) -> IResult<&[u8], &[u8]> {
    terminated(tag("?uploadbin"), line_ending)(input)
}

fn from_hex(input: &[u8]) -> anyhow::Result<u32> {
    let in_str = from_utf8(input)?;
    let num = u32::from_str_radix(in_str, 16)?;
    Ok(num)
}

fn hex_number(input: &[u8]) -> IResult<&[u8], u32> {
    map_res(preceded(tag("0x"), hex_digit1), from_hex)(input)
}

fn utf8_string(input: &[u8]) -> anyhow::Result<&str> {
    let in_str = from_utf8(input)?;
    Ok(in_str)
}

fn register(input: &[u8]) -> IResult<&[u8], (&str, u32, u32)> {
    let (remaining, _) = tag("?register")(input)?;
    let (remaining, name) = map_res(preceded(space1, take_till(is_space)), utf8_string)(remaining)?;
    let (remaining, addr) = preceded(space1, hex_number)(remaining)?;
    let (remaining, size) = terminated(preceded(space1, hex_number), line_ending)(remaining)?;
    Ok((remaining, (name, addr, size)))
}

type Metadata<'a> = (KString, &'a str, &'a str, &'a str);

fn meta(input: &[u8]) -> IResult<&[u8], Metadata> {
    let (remaining, _) = tag("?meta")(input)?;
    let (remaining, device) =
        map_res(preceded(space1, take_till(is_space)), utf8_string)(remaining)?;
    let (remaining, kind) = map_res(preceded(space1, take_till(is_space)), utf8_string)(remaining)?;
    let (remaining, meta_key) =
        map_res(preceded(space1, take_till(is_space)), utf8_string)(remaining)?;
    let (remaining, meta_value) = map_res(
        preceded(space1, terminated(not_line_ending, line_ending)),
        utf8_string,
    )(remaining)?;
    // For some (unknown) reason, the metadata object path uses '/' for nested context, instead of
    // '_' like the registers list To make them match (for later lookup), we'll replace them.
    Ok((
        remaining,
        (device.replace('/', "_").into(), kind, meta_key, meta_value),
    ))
}

fn quit(input: &[u8]) -> IResult<&[u8], &[u8]> {
    terminated(tag("?quit"), line_ending)(input)
}

pub(crate) fn fpg_file(input: &[u8]) -> IResult<&[u8], FpgFile> {
    let (remaining, _) = shebang(input)?;
    let (remaining, _) = uploadbin(remaining)?;
    let (remaining, registers) = many0(register)(remaining)?;
    let (remaining, metas) = many0(meta)(remaining)?;
    let (bitstream, _) = quit(remaining)?;

    let mut registers: HashMap<KString, FpgRegister> = registers
        .into_iter()
        .map(|(name, addr, size)| (name.to_owned().into(), FpgRegister { addr, size }))
        .collect();

    let mut devices: HashMap<KString, FpgDevice> = HashMap::new();

    for (name, kind, k, v) in metas {
        match devices.get_mut(&name) {
            Some(d) => {
                d.add_meta(k.to_owned().into(), v.to_owned());
            }
            None => {
                devices.insert(
                    name.clone(),
                    FpgDevice {
                        kind: kind.to_owned(),
                        metadata: HashMap::from_iter([(k.to_owned().into(), v.to_owned())]),
                        register: registers.remove(&name),
                    },
                );
            }
        }
    }

    Ok((
        bitstream,
        FpgFile {
            devices,
            bitstream: bitstream.into(),
        },
    ))
}

/// Reads a CASPER-specific FPG file
pub fn read_fpg_file<T>(filename: T) -> anyhow::Result<FpgFile>
where
    T: AsRef<Path>,
{
    let mut file = File::open(filename)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    let (_, file) = fpg_file(&contents)
        .map_err(|_| anyhow!("Error parsing fpg file, are you sure it's valid?"))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shebang() {
        let test_str = "#!/bin/kcpfpg\n".as_bytes();
        let (remaining, tag) = shebang(test_str).unwrap();
        assert_eq!(remaining, []);
        assert_eq!(tag, "#!/bin/kcpfpg".as_bytes());
    }

    #[test]
    fn test_uploadbin() {
        let test_str = "?uploadbin\n".as_bytes();
        let (remaining, tag) = uploadbin(test_str).unwrap();
        assert_eq!(remaining, []);
        assert_eq!(tag, "?uploadbin".as_bytes());
    }

    #[test]
    fn test_register() {
        let test_str = "?register	fft_overflow_cnt	0x3510c	0x4\n".as_bytes();
        let (remaining, (name, addr, size)) = register(test_str).unwrap();
        assert_eq!(remaining, []);
        assert_eq!(name, "fft_overflow_cnt");
        assert_eq!(addr, 0x3510C);
        assert_eq!(size, 0x4);
    }

    #[test]
    fn test_meta() {
        let test_str = "?meta	gbe0/txs/ss/bram	xps:bram	init_vals	[0:2^13-1]\n".as_bytes();
        let (remaining, (device, kind, key, value)) = meta(test_str).unwrap();
        assert_eq!(remaining, []);
        assert_eq!(device, "gbe0_txs_ss_bram");
        assert_eq!(kind, "xps:bram");
        assert_eq!(key, "init_vals");
        assert_eq!(value, "[0:2^13-1]");
    }

    #[test]
    fn test_fpg_file() {
        let mut input = "#!/bin/kcpfpg
?uploadbin
?register	tx_en	0x3513c	0x4
?meta	SNAP	xps:xsg	clk_rate	250
?meta	tx_en	xps:sw_reg	bitwidths	32
?quit
"
        .as_bytes()
        .to_vec();

        input.append(&mut vec![0xDE, 0xAD, 0xBE, 0xEF]);

        let (_, file) = fpg_file(&input).unwrap();
        assert_eq!(
            *file.devices.get("SNAP").unwrap(),
            FpgDevice {
                kind: "xps:xsg".to_owned(),
                register: None,
                metadata: HashMap::from_iter([("clk_rate".into(), "250".to_owned())])
            }
        );
        assert_eq!(
            *file.devices.get("tx_en").unwrap(),
            FpgDevice {
                kind: "xps:sw_reg".to_owned(),
                register: Some(FpgRegister {
                    addr: 217404,
                    size: 4
                }),
                metadata: HashMap::from_iter([("bitwidths".into(), "32".to_owned())])
            }
        );
        assert_eq!(file.bitstream, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }
}
