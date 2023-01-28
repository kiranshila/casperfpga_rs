//! Logic and implementations for CASPER "Yellow Block" devices.
pub mod snapadc;
pub mod snapshot;
pub mod swreg;
pub mod ten_gbe;

/// Certain Yellow Block struct types will implement this trait to allow for auto offsets in
/// transport read methods
pub trait Address {
    fn addr() -> u16;
}
