//! Routines for interacting with CASPER software register yellow blocks. This uses the `fixed` crate to interact with fixed point numbers.

use super::YellowBlock;
use crate::bitstream::fpg::FpgDevice;
use anyhow::bail;

/// The IO direction of this register
#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    /// Client applications can read registers of this kind
    ToProcessor,
    /// Client applications can write registers of this kind
    FromProcessor,
}

/// The kind of software register
#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    /// This register contains boolean data
    Bool,
    /// This register contains fixed point data
    Fixed { bin_pts: usize, signed: bool },
}

/// The unidirectional 32-bit fixed point software register yellow block
#[derive(Debug, PartialEq, Eq)]
pub struct SoftwareRegister {
    /// IO direction of this register
    direction: Direction,
    /// The kind of software register
    kind: Kind,
}

impl YellowBlock for SoftwareRegister {
    fn from_fpg(device: &FpgDevice) -> anyhow::Result<Self> {
        if device.kind != "xps:sw_reg" {
            bail!("Provided FpgDevice is not of the right kind");
        }
        let direction = match device.metadata.get("io_dir") {
            Some(s) => match s.as_str() {
                "To\\_Processor" => Direction::ToProcessor,
                "From\\_Processor" => Direction::FromProcessor,
                _ => bail!("Malformed FpgDevice metadata entry"),
            },
            None => bail!("Missing FpgDevice metadata entry"),
        };

        let bin_pts = match device.metadata.get("bin_pts") {
            Some(s) => s.as_str().parse()?,
            None => bail!("Missing FpgDevice metadata entry"),
        };

        let kind = if let Some(s) = device.metadata.get("arith_types") {
            match s.as_str() {
                "0" => Kind::Fixed {
                    bin_pts,
                    signed: false,
                },
                "1" => Kind::Fixed {
                    bin_pts,
                    signed: true,
                },
                "2" => Kind::Bool,
                _ => bail!("Missing FpgDevice metadata entry"),
            }
        } else {
            bail!("Missing FpgDevice metadata entry")
        };

        Ok(SoftwareRegister { direction, kind })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_from_fpg() {
        let device = FpgDevice {
            kind: "xps:sw_reg".to_owned(),
            metadata: HashMap::from_iter([
                ("io_dir".into(), "From\\_Processor".to_owned()),
                ("io_delay".into(), "0".to_owned()),
                ("bin_pts".into(), "0".to_owned()),
                ("arith_types".into(), "0".to_owned()),
            ]),
        };
        let swreg = SoftwareRegister::from_fpg(&device).unwrap();
        assert_eq!(swreg.direction, Direction::FromProcessor);
        assert_eq!(
            swreg.kind,
            Kind::Fixed {
                bin_pts: 0,
                signed: false
            }
        );
    }
}
