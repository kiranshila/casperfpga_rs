use crate::bitstream::fpg::FpgDevice;

pub mod clockswitch;
pub mod snapadc;
pub mod swreg;
pub mod ten_gbe;

pub trait YellowBlock: Sized {
    fn from_fpg(device: &FpgDevice) -> anyhow::Result<Self>;
}
