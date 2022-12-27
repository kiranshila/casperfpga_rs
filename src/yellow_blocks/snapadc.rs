//! Interface to the ADCs on the SNAP board

use super::YellowBlock;
use crate::bitstream::fpg::FpgDevice;

pub struct SnapAdc {}

impl YellowBlock for SnapAdc {
    fn from_fpg(device: &FpgDevice) -> anyhow::Result<Self> {
        todo!()
    }
}
