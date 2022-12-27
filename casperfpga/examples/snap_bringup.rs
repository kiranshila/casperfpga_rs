//! In this example, we will connect to a SNAP over TAPCP, program a file, calibrate the ADCs, and
//! setup the 10 GbE core.

use casperfpga::{
    core::CasperFpga,
    transport::tapcp::Tapcp,
};
use casperfpga_derive::fpga_from_fpg;

fpga_from_fpg!(GrexFpga, "snap160t_golden.fpg");

fn main() {}
