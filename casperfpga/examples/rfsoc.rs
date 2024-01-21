use casperfpga::{
    prelude::*,
    transport::local::Local,
};

fn main() -> anyhow::Result<()> {
    // Read FPG file (this should be macro-ified)
    let fpg = read_fpg_file("rfsoc4x2_tut_platform.fpg")?;
    // Construct a transport
    let mut transport = Local::new(fpg.devices)?;
    // Write a register
    let reg_name = "sys_scratchpad".to_string();
    let reg_val = 0xdeadu32;
    transport.write(&reg_name, 0, &reg_val).unwrap();
    // Readback
    let read_val: u32 = transport.read(&reg_name, 0).unwrap();
    // Verify
    assert_eq!(read_val, reg_val);
    Ok(())
}
