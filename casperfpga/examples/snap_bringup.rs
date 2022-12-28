//! In this example, we will connect to a SNAP over TAPCP, program a file, calibrate the ADCs, and
//! setup the 10 GbE core.

use casperfpga::transport::{tapcp::Tapcp, Transport};
use casperfpga_derive::fpga_from_fpg;
use fixed::types::U27F5;

fpga_from_fpg!(
    GrexFpga,
    "/home/kiran/Dropbox/Projects/Rust/casperfpga/casperfpga/examples/grex_gateware.fpg"
);

fn main() {
    let mut transport = Tapcp::connect("192.168.0.3:69".parse().unwrap()).unwrap();
    dbg!(transport.listdev());
    //let fpga = GrexFpga::new(transport).unwrap();
    //let gain = U27F5::from_num(2.33);
    //fpga.requant_gain.write(&gain).unwrap();
    //dbg!(fpga.requant_gain.read().unwrap());
}
