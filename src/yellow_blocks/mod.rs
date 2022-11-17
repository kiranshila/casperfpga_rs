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