//! Routines for interacting with the CASPER 10GbE Core
use crate::{
    transport::{
        Deserialize,
        Serialize,
        Transport,
    },
    yellow_blocks::Address,
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
    net::Ipv4Addr,
    sync::{
        Mutex,
        Weak,
    },
};
use thiserror::Error;

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
    type ByteArray = [u8; 8];

    fn pack(&self) -> PackingResult<Self::ByteArray> {
        let mut dest = [0u8; 8];
        dest[2..].copy_from_slice(&self.0);
        Ok(dest)
    }

    fn unpack(src: &Self::ByteArray) -> packed_struct::PackingResult<Self> {
        Ok(MacAddress(src[2..].try_into().unwrap()))
    }
}

macro_rules! ip_register {
    ($name:ident, $addr:literal) => {
        #[derive(Debug, CasperSerde)]
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

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Transport(#[from] crate::transport::Error),
}

#[derive(Debug)]
pub struct TenGbE<T> {
    transport: Weak<Mutex<T>>,
    name: String,
}

impl<T> TenGbE<T>
where
    T: Transport,
{
    /// Builds a [`TenGbE`] from FPG description strings
    /// # Errors
    /// Returns an error on bad string arguments
    pub fn from_fpg(transport: Weak<Mutex<T>>, reg_name: &str) -> Result<Self, Error> {
        Ok(Self {
            transport,
            name: reg_name.to_string(),
        })
    }

    /// Get the IP of the core
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn get_ip(&self) -> Result<Ipv4Addr, Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let ip: IpAddress = transport.read_addr(&self.name)?;
        Ok(ip.0)
    }

    /// Set the IP of the core
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn set_ip(&self, addr: Ipv4Addr) -> Result<(), Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        Ok(transport.write_addr(&self.name, &IpAddress(addr))?)
    }

    /// Get the gateway IP of the core
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn get_gateway(&self) -> Result<Ipv4Addr, Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let ip: GatewayAddress = transport.read_addr(&self.name)?;
        Ok(ip.0)
    }

    /// Set the gateway IP of the core
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn set_gateway(&self, addr: Ipv4Addr) -> Result<(), Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        Ok(transport.write_addr(&self.name, &GatewayAddress(addr))?)
    }

    /// Get the port of the core
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn get_port(&self) -> Result<u16, Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let port: Port = transport.read_addr(&self.name)?;
        Ok(port.port)
    }

    /// Set the port of the core
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn set_port(&self, port: u16) -> Result<(), Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        Ok(transport.write_addr(
            &self.name,
            &Port {
                port_mask: 0xFF,
                port,
            },
        )?)
    }

    /// Get the subnet mask of the core
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn get_netmask(&self) -> Result<Ipv4Addr, Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let ip: Netmask = transport.read_addr(&self.name)?;
        Ok(ip.0)
    }

    /// Set the subnet mask of the core
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn set_netmask(&self, addr: Ipv4Addr) -> Result<(), Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        Ok(transport.write_addr(&self.name, &Netmask(addr))?)
    }

    /// Get the MAC address of the core
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn get_mac(&self) -> Result<[u8; 6], Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let mac: MacAddress = transport.read_addr(&self.name)?;
        Ok(mac.0)
    }

    /// Set the MAC address of the core
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn set_mac(&self, mac: &[u8; 6]) -> Result<(), Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        Ok(transport.write_addr(&self.name, &MacAddress(*mac))?)
    }

    /// Enable or disable the core fabric
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn set_enable(&self, enabled: bool) -> Result<(), Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        Ok(transport.write_addr(
            &self.name,
            &PromiscRstEn {
                soft_rst: false,
                promisc: false,
                enable: enabled,
            },
        )?)
    }

    /// Toggle the software reset of the core
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn toggle_reset(&self) -> Result<(), Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let mut pre: PromiscRstEn = transport.read_addr(&self.name)?;
        pre.soft_rst = false;
        transport.write_addr(&self.name, &pre)?;
        pre.soft_rst = true;
        transport.write_addr(&self.name, &pre)?;
        pre.soft_rst = false;
        transport.write_addr(&self.name, &pre)?;
        Ok(())
    }

    /// Set a single entry in the ARP table
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn set_single_arp_entry(&self, ip: Ipv4Addr, mac: &[u8; 6]) -> Result<(), Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // ARP entries start at 0x1000 and are laid out like MacAddress
        // two bytes of zeros then mac
        let offset = 0x1000 + 8 * (*ip.octets().last().unwrap()) as usize;
        transport.write(&self.name, offset, &MacAddress(*mac))?;
        Ok(())
    }
}
