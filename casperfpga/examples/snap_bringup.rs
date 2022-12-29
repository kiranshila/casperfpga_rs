//! In this example, we will connect to a SNAP over TAPCP, program a file, calibrate the ADCs, and
//! setup the 10 GbE core.

use casperfpga::transport::tapcp::Tapcp;
use casperfpga_derive::fpga_from_fpg;

fpga_from_fpg!(
    GrexFpga,
    "/home/kiran/Dropbox/Projects/Rust/casperfpga/casperfpga/examples/grex_gateware.fpg"
);

fn main() {
    let transport = Tapcp::connect("192.168.0.3:69".parse().unwrap()).unwrap();
    let fpga = GrexFpga::new(transport).unwrap();
    dbg!(fpga.snap_adc.controller.get_demux().unwrap());
    //dbg!(fpga);
}
