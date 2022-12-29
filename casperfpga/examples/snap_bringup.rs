//! In this example, we will connect to a SNAP over TAPCP, program a file, calibrate the ADCs, and
//! setup the 10 GbE core.

use casperfpga::{
    core::estimate_fpga_clock,
    transport::{
        tapcp::Tapcp,
        Transport,
    },
};
use casperfpga_derive::fpga_from_fpg;

fpga_from_fpg!(GrexFpga, "casperfpga/examples/grex_gateware.fpg");

fn main() {
    let mut transport = Tapcp::connect("192.168.0.3:69".parse().unwrap()).unwrap();

    dbg!(estimate_fpga_clock(&mut transport).unwrap());
    //let fpga = GrexFpga::new(transport).unwrap();
    //dbg!(fpga);
}
