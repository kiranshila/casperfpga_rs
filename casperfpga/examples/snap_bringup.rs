//! In this example, we will connect to a SNAP over TAPCP, program a file, calibrate the ADCs, and
//! setup the 10 GbE core.

use std::collections::HashMap;

use casperfpga::transport::mock::Mock;
use casperfpga_derive::fpga_from_fpg;

fpga_from_fpg!(
    GrexFpga,
    "casperfpga/examples/grex_gateware_2022-10-18_1631.fpg"
);

fn main() {
    let transport = Mock::new(HashMap::new());
    let fpga = GrexFpga::new(transport);
    dbg!(fpga);
}
