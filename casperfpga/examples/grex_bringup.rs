//! In this example, we will connect to a SNAP over TAPCP, program a file, calibrate the ADCs, and
//! setup the 10 GbE core.

use casperfpga::{
    prelude::*,
    yellow_blocks::snapadc::{
        controller::ChannelInput,
        hmcad1511::InputSelect,
    },
};
use std::net::Ipv4Addr;

fpga_from_fpg!(GrexFpga, "casperfpga/examples/grex_gateware.fpg");

fn main() -> anyhow::Result<()> {
    // Create the transport and connect
    let mut fpga = GrexFpga::new(Tapcp::connect(
        "192.168.0.3:69".parse()?,
        tapcp::Platform::SNAP,
    )?)?;

    // Program the design
    let design = read_fpg_file("casperfpga/examples/grex_gateware.fpg")?;
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
    fpga.master_rst.write(false)?;
    fpga.master_rst.write(true)?;
    fpga.master_rst.write(false)?;

    fpga.gbe1.set_ip("192.168.0.20".parse()?)?;
    fpga.gbe1.set_gateway(dest_ip)?;
    fpga.gbe1.set_netmask("255.255.255.0".parse()?)?;
    fpga.gbe1.set_port(dest_port)?;
    fpga.gbe1.set_mac(&[0x02, 0x2E, 0x46, 0xE0, 0x64, 0xA1])?;
    fpga.gbe1.set_enable(true)?;
    fpga.gbe1.toggle_reset()?;

    // Set destination registers
    fpga.dest_port.write(dest_port.into())?;
    fpga.dest_ip.write(u32::from(dest_ip).into())?;
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
