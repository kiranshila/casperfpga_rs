use std::{
    collections::HashMap,
    net::{
        SocketAddr,
        UdpSocket,
    },
    time::Duration,
};

const RETRIES: usize = 7;
const SNAP_FLASH_LOC: u32 = 0x800000;

fn main() -> anyhow::Result<()> {
    // Setup the socket
    let mut socket = UdpSocket::bind("0.0.0.0:0")?;
    // Set a default timeout
    let timeout = Duration::from_secs_f32(0.5);
    socket.set_write_timeout(Some(timeout))?;
    socket.set_read_timeout(Some(timeout))?;
    // Connect
    let host_addr: SocketAddr = "192.168.0.3:69".parse()?;
    socket.connect(host_addr)?;
    let sample_meta = HashMap::from([
        ("flash".into(), "1234".to_string()),
        ("foo".into(), "bar".to_string()),
    ]);
    tapcp::set_metadata(&sample_meta, &mut socket, SNAP_FLASH_LOC, RETRIES)?;
    std::thread::sleep(Duration::from_secs_f32(0.5));
    assert_eq!(
        sample_meta,
        tapcp::get_metadata(&mut socket, SNAP_FLASH_LOC, RETRIES)?
    );
    Ok(())
}
