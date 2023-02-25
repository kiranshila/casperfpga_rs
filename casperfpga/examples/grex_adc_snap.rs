use anyhow::bail;
use casperfpga::prelude::*;

fpga_from_fpg!(
    GrexFpga,
    "/home/kiran/Projects/Rust/casperfpga/casperfpga/examples/grex_gateware.fpg"
);

fn main() -> anyhow::Result<()> {
    let fpga = GrexFpga::new(Tapcp::connect(
        "192.168.0.5:69".parse()?,
        tapcp::Platform::SNAP,
    )?)?;
    if !fpga.transport.lock().unwrap().is_running()? {
        bail!("FPGA isn't runnning");
    }
    fpga.adc_snap.arm()?;
    fpga.adc_snap.trigger()?;
    let bytes = fpga.adc_snap.read()?;
    dbg!(bytes);
    Ok(())
}
