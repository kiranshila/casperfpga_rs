//! Routines for interacting with the CASPER 10GbE Core
use crate::transport::{
    Deserialize,
    Serialize,
    Transport,
};
use casperfpga_derive::{
    address,
    CasperSerde,
};
use packed_struct::{
    prelude::*,
    PackedStruct,
    PackingResult,
};
use std::{
    cell::RefCell,
    net::Ipv4Addr,
    sync::Weak,
};

#[derive(Debug)]
pub struct TenGbE<T> {
    transport: Weak<RefCell<T>>,
    name: String,
}

impl<T> TenGbE<T>
where
    T: Transport,
{
    pub fn from_fpg(transport: Weak<RefCell<T>>, reg_name: &str) -> anyhow::Result<Self> {
        Ok(Self {
            transport,
            name: reg_name.to_string(),
        })
    }
}

// The details of the memory map here are magical and come from Jack H

// The 10 GbE Core itself exists as a big register that we can query over the transports
// So, we need to read/write to the register of that name (the name of the block from Simulink)
// at an offset of the address of the thing we care about. We will always read 4 bytes and then
// pass to the packed_struct methods to serde from the rust types

#[derive(PrimitiveEnum_u8, Debug, Copy, Clone)]
pub enum EthernetType {
    OneGbE = 1,
    TenGbE = 2,
    TwentyFiveGbE = 3,
    FortyGbE = 4,
    HundredGbE = 5,
}

#[derive(PackedStruct, CasperSerde, Debug)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "4")]
#[address(0x0)]
pub struct CoreType {
    #[packed_field(bits = "24")]
    pub cpu_tx_enable: bool,
    #[packed_field(bits = "16")]
    pub cpu_rx_enable: bool,
    #[packed_field(bytes = "1")]
    pub revision: u8,
    #[packed_field(bytes = "0", ty = "enum")]
    pub core_type: EthernetType,
}

#[derive(PackedStruct, CasperSerde, Debug)]
#[address(0x4)]
pub struct BufferSizes {
    #[packed_field(endian = "msb")]
    pub tx_buf_max: u16,
    #[packed_field(endian = "msb")]
    pub rx_buf_max: u16,
}

#[derive(PackedStruct, CasperSerde, Debug)]
#[address(0x8)]
pub struct WordLengths {
    #[packed_field(endian = "msb")]
    pub tx_word_size: u16,
    #[packed_field(endian = "msb")]
    pub rx_word_size: u16,
}

// Implement the packing traits for network objects

#[derive(CasperSerde, Debug)]
#[address(0xC)]
pub struct MacAddress([u8; 6]);

impl PackedStruct for MacAddress {
    type ByteArray = [u8; 6];

    fn pack(&self) -> PackingResult<Self::ByteArray> {
        Ok(self.0)
    }

    fn unpack(src: &Self::ByteArray) -> packed_struct::PackingResult<Self> {
        Ok(MacAddress(*src))
    }
}

macro_rules! ip_register {
    ($name:ident, $addr:literal) => {
        #[derive(Debug)]
        #[address($addr)]
        pub struct $name(pub Ipv4Addr);

        impl PackedStruct for $name {
            type ByteArray = [u8; 4];

            fn pack(&self) -> PackingResult<Self::ByteArray> {
                Ok(self.0.octets())
            }

            fn unpack(src: &Self::ByteArray) -> packed_struct::PackingResult<Self> {
                Ok($name(Ipv4Addr::new(src[0], src[1], src[2], src[3])))
            }
        }
    };
}

ip_register!(IpAddress, 0x14);
ip_register!(GatewayAddress, 0x18);
ip_register!(Netmask, 0x1C);
ip_register!(MulticastIp, 0x20);
ip_register!(MulticastMask, 0x24);

#[derive(PackedStruct, CasperSerde, Debug)]
#[address(0x28)]
pub struct BytesAvailable {
    #[packed_field(endian = "msb")]
    pub tx_size: u16,
    #[packed_field(endian = "msb")]
    pub rx_size: u16,
}

#[derive(PackedStruct, CasperSerde, Debug)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "4")]
#[address(0x2C)]
pub struct PromiscRstEn {
    #[packed_field(bits = "4")]
    pub soft_rst: bool,
    #[packed_field(bits = "2")]
    pub promisc: bool,
    #[packed_field(bits = "0")]
    pub enable: bool,
}

#[derive(PackedStruct, CasperSerde, Debug)]
#[address(0x30)]
pub struct Port {
    #[packed_field(endian = "msb")]
    pub port_mask: u16,
    #[packed_field(endian = "msb")]
    pub port: u16,
}

#[derive(PackedStruct, CasperSerde, Debug)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "8")]
#[address(0x34)]
pub struct Status {
    // There's other (undocumented) stuff in here
    #[packed_field(bits = "0")]
    pub link_up: bool,
}
