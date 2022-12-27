//! Logic and implementations for "Yellow Block" devices.

pub mod clockswitch;
pub mod snapadc;
pub mod swreg;
pub mod ten_gbe;

use casper_utils::bitstream::fpg::FpgDevice;

trait YellowBlock: Sized {
    /// The device kind, as read from an FPG file
    const KIND: &'static str;

    /// Create an object instance from an [FpgDevice]
    fn from_fpg(device: &FpgDevice) -> anyhow::Result<Self>;
}
