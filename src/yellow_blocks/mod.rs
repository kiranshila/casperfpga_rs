pub mod snapadc;
pub mod ten_gbe;

/// A trait to assign an FPGA address to a struct (presumably one we will serde)
pub trait RegisterAddress {
    /// Returns the address of this particular struct
    fn address() -> u8;
}

/// Auto generates the trait impl from an enum of addresses
#[macro_export]
macro_rules! register_address {
    ($addrs:ident, $reg:ident) => {
        impl RegisterAddress for $reg {
            fn address() -> u8 {
                $addrs::$reg as u8
            }
        }
    };
}

// Auto implement our transport serde for packed_struct things
#[macro_export]
macro_rules! serde_packed {
    ($type:ty) => {
        impl Serialize for $type {
            type Chunk = [u8; std::mem::size_of::<Self>()];

            fn serialize(&self) -> Self::Chunk {
                self.pack().expect("Packing failed, this shouldn't happen")
            }
        }

        impl Deserialize for $type {
            type Chunk = [u8; std::mem::size_of::<Self>()];

            fn deserialize(chunk: Self::Chunk) -> anyhow::Result<Self> {
                Ok(Self::unpack(&chunk)?)
            }
        }
    };
}