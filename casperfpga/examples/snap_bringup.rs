//! In this example, we will connect to a SNAP over TAPCP, program a file, calibrate the ADCs, and
//! setup the 10 GbE core.

use casperfpga::{
    transport::tapcp::Tapcp,
    yellow_blocks::snapadc::{
        controller::ChannelInput,
        hmcad1511::InputSelect,
        SnapAdcChip,
    },
};
use casperfpga_derive::fpga_from_fpg;

fpga_from_fpg!(
    GrexFpga,
    "/home/kiran/Dropbox/Projects/Rust/casperfpga/casperfpga/examples/grex_gateware.fpg"
);

fn main() -> anyhow::Result<()> {
    let transport = Tapcp::connect("192.168.0.3:69".parse()?)?;
    let mut fpga = GrexFpga::new(transport)?;
    fpga.snap_adc.initialize()?;
    fpga.snap_adc
        .select_inputs(ChannelInput::Dual(InputSelect::_1, InputSelect::_3))?;
    dbg!(fpga.snap_adc.snapshot(SnapAdcChip::A)?);
    Ok(())
}
