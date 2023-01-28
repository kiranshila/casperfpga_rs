use anyhow::bail;
use casperfpga::transport::tapcp::{Platform, Tapcp};
use casperfpga_derive::fpga_from_fpg;
fpga_from_fpg!(
    GrexFpga,
    "/home/kiran/Projects/Rust/casperfpga/casperfpga/examples/grex_gateware.fpg"
);
use casperfpga::transport::Transport;
use hdrhistogram::Histogram;

#[derive(Debug)]
struct TimeSeries {
    a: Vec<i8>,
    b: Vec<i8>,
}

impl TimeSeries {
    fn from_bytes(bytes: &[u8]) -> Self {
        // Bytes are interleaved out of the adc, and we have two channels
        // So, we need to interleave them and split them into channels

        let mut a = vec![];
        let mut b = vec![];

        for chunk in bytes.chunks(4) {
            a.push(chunk[0] as i8);
            a.push(chunk[1] as i8);
            b.push(chunk[2] as i8);
            b.push(chunk[3] as i8);
        }

        Self { a, b }
    }
}

fn main() -> anyhow::Result<()> {
    let fpga = GrexFpga::new(Tapcp::connect("192.168.0.5:69".parse()?, Platform::SNAP)?)?;
    if !fpga.transport.lock().unwrap().is_running()? {
        bail!("FPGA isn't runnning");
    }
    fpga.adc_snap.arm()?;
    fpga.adc_snap.trigger()?;
    let bytes = fpga.adc_snap.read()?;
    let ts = TimeSeries::from_bytes(&bytes);

    Ok(())
}
