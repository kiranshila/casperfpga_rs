//! Interface to the HMC922 Differential SPDT Switch
//!
use super::YellowBlock;
use crate::bitstream::fpg::FpgDevice;

pub struct ClockSwitch {}

impl YellowBlock for ClockSwitch {
    fn from_fpg(device: &FpgDevice) -> anyhow::Result<Self> {
        todo!()
    }
}
