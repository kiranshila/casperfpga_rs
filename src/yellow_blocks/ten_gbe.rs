//! Routines for interacting with the CASPER 10GbE Core
use crate::{
    register_address,
    transport::{Deserialize, Serialize},
    yellow_blocks::RegisterAddress, serde_packed,
};
use packed_struct::{prelude::*, PackedStruct, PackingResult};
use std::net::Ipv4Addr;
// The details of the memory map here are magical and come from Jack H

// The 10 GbE Core itself exists as a big register that we can query over the transports
// So, we need to read/write to the register of that name (the name of the block from Simulink)
// at an offset of the address of the thing we care about. We will always read 4 bytes and then
// pass to the packed_struct methods to serde from the rust types

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
enum CoreAddress {
    CoreType = 0x0,
    BufferSizes = 0x4,
    WordLengths = 0x8,
    MacAddress = 0xC,
    IpAddress = 0x14,
    GatewayAddress = 0x18,
    Netmask = 0x1C,
    MulticastIp = 0x20,
    MulticastMask = 0x24,
    BytesAvailable = 0x28,
    PromiscRstEn = 0x2C,
    Port = 0x30,
    Status = 0x34,
    // Control = 0x3C,
    // ARPSize = 0x44,
    // TXPacketRate = 0x48,
    // TXPacketCounter = 0x4C,
    // TXValidRate = 0x50,
    // TXValidCounter = 0x54,
    // TXOverflowCounter = 0x58,
    // TXAlmostFullCounter = 0x5C,
    // RXPacketRate = 0x60,
    // RXPacketCounter = 0x64,
    // RXValidRate = 0x68,
    // RXValidCounter = 0x6C,
    // RXOverflowCounter = 0x70,
    // RXBadCounter = 0x74,
    //  CounterReset = 0x78,
}

#[derive(PrimitiveEnum_u8, Debug, Copy, Clone)]
pub enum EthernetType {
    OneGbE = 1,
    TenGbE = 2,
    TwentyFiveGbE = 3,
    FortyGbE = 4,
    HundredGbE = 5,
}

// Address impls
register_address! {CoreAddress,CoreType}
register_address! {CoreAddress,BufferSizes}
register_address! {CoreAddress,WordLengths}
register_address! {CoreAddress,MacAddress}
register_address! {CoreAddress,IpAddress}
register_address! {CoreAddress,GatewayAddress}
register_address! {CoreAddress,Netmask}
register_address! {CoreAddress,MulticastIp}
register_address! {CoreAddress,MulticastMask}
register_address! {CoreAddress,BytesAvailable}
register_address! {CoreAddress,PromiscRstEn}
register_address! {CoreAddress,Port}
register_address! {CoreAddress,Status}

// Serde impls
serde_packed!(CoreType);
serde_packed!(BufferSizes);
serde_packed!(WordLengths);
serde_packed!(MacAddress);
serde_packed!(IpAddress);
serde_packed!(GatewayAddress);
serde_packed!(Netmask);
serde_packed!(MulticastIp);
serde_packed!(MulticastMask);
serde_packed!(BytesAvailable);
serde_packed!(PromiscRstEn);
serde_packed!(Port);
serde_packed!(Status);

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "4")]
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

#[derive(PackedStruct, Debug)]
pub struct BufferSizes {
    #[packed_field(endian = "msb")]
    pub tx_buf_max: u16,
    #[packed_field(endian = "msb")]
    pub rx_buf_max: u16,
}

#[derive(PackedStruct, Debug)]
pub struct WordLengths {
    #[packed_field(endian = "msb")]
    pub tx_word_size: u16,
    #[packed_field(endian = "msb")]
    pub rx_word_size: u16,
}

// Implement the packing traits for network objects

#[derive(Debug)]
pub struct MacAddress([u8; 6]);

macro_rules! ip_register {
    ($name:ident) => {
        #[derive(Debug)]
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

ip_register!(IpAddress);
ip_register!(GatewayAddress);
ip_register!(Netmask);
ip_register!(MulticastIp);
ip_register!(MulticastMask);

#[derive(PackedStruct, Debug)]
pub struct BytesAvailable {
    #[packed_field(endian = "msb")]
    pub tx_size: u16,
    #[packed_field(endian = "msb")]
    pub rx_size: u16,
}

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "4")]
pub struct PromiscRstEn {
    #[packed_field(bits = "4")]
    pub soft_rst: bool,
    #[packed_field(bits = "2")]
    pub promisc: bool,
    #[packed_field(bits = "0")]
    pub enable: bool,
}

#[derive(PackedStruct, Debug)]
pub struct Port {
    #[packed_field(endian = "msb")]
    pub port_mask: u16,
    #[packed_field(endian = "msb")]
    pub port: u16,
}

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "8")]
pub struct Status {
    // There's other (undocumented) stuff in here
    #[packed_field(bits = "0")]
    pub link_up: bool,
}