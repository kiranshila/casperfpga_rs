//! In this example, we will connect to a SNAP over TAPCP, program a file, calibrate the ADCs, and
//! setup the 10 GbE core.

use casper_utils::design_sources::fpg::read_fpg_file;
use casperfpga::{
    transport::{
        tapcp::{Platform, Tapcp},
        Transport,
    },
    yellow_blocks::snapadc::{controller::ChannelInput, hmcad1511::InputSelect},
};
use casperfpga_derive::fpga_from_fpg;
use fixed::types::U32F0;
use std::net::Ipv4Addr;
fpga_from_fpg!(
    GrexFpga,
    "/home/kiran/Projects/Rust/casperfpga/casperfpga/examples/grex_gateware.fpg"
);

fn main() -> anyhow::Result<()> {
    // Create the transport and connect
    let mut fpga = GrexFpga::new(Tapcp::connect("192.168.0.3:69".parse()?, Platform::SNAP)?)?;

    // Program the design
    let design = read_fpg_file(
        "/home/kiran/Projects/Rust/casperfpga/casperfpga/examples/grex_gateware.fpg",
    )?;
    fpga.transport.lock().unwrap().program(&design, true)?;

    // Setup the ADCs
    fpga.snap_adc.initialize()?;
    fpga.snap_adc
        .select_inputs(ChannelInput::Dual(InputSelect::_1, InputSelect::_1))?;

    // Configure the 10 GbE core
    let dest_ip: Ipv4Addr = "192.168.0.1".parse()?;
    let dest_mac = [0x98, 0xb7, 0x85, 0xa7, 0xec, 0x78];
    let dest_port = 60000u16;

    // Disable
    fpga.tx_en.write(false)?;
    // Reset
    fpga.tx_rst.write(false)?;
    fpga.tx_rst.write(true)?;
    fpga.tx_rst.write(false)?;

    fpga.gbe1.set_ip("192.168.0.20".parse()?)?;
    fpga.gbe1.set_gateway(dest_ip)?;
    fpga.gbe1.set_netmask("255.255.255.0".parse()?)?;
    fpga.gbe1.set_port(dest_port)?;
    fpga.gbe1.set_mac(&[0x02, 0x2E, 0x46, 0xE0, 0x64, 0xA1])?;
    fpga.gbe1.set_enable(true)?;
    fpga.gbe1.toggle_reset()?;

    // Set destination registers
    fpga.dest_port.write(&U32F0::from_num(dest_port))?;
    fpga.dest_ip.write(&U32F0::from_num(u32::from(dest_ip)))?;
    fpga.gbe1.set_single_arp_entry(dest_ip, &dest_mac)?;

    // Turn on the core
    fpga.tx_en.write(true)?;

    // Check the link
    assert!(fpga.gbe1_linkup.read()?, "10GbE Link Failed to come up");

    // Toggle the master reset and send a sync pulse
    fpga.master_rst.write(false)?;
    fpga.master_rst.write(true)?;
    fpga.master_rst.write(false)?;
    fpga.pps_trig.write(false)?;
    fpga.pps_trig.write(true)?;
    fpga.pps_trig.write(false)?;

    println!("PPS Count - {}", fpga.pps_cnt.read()?);

    // Read some status
    Ok(())
}
