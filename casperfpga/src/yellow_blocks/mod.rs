//! Logic and implementations for CASPER "Yellow Block" devices.
//! These are at the heart of a casperfpga design and will be the structs you primarily interact
//! with.
//!
//! From a design perspective, all of the yellow block structs contain a `transport` field which is
//! of type `Weak<Mutex<T: Transport>>`, this allows the yellow block to interact with the
//! transport, but not own the transport. This is important as one will almost certainly have
//! many yellow blocks that will all needs to interface to the hardware. Although nothing enforces
//! the convention, it is best practice to put the owned `Arc<Mutex<T:Transport>>` in some top-level
//! struct and then have the yellow blocks as members of that struct.
//!
//! To this end, all yellow block structs follow the constructor convention of `new(transport:
//! &Arc<Mutex<T:Transport>>, reg_name: &str, ..<metadata>)`, where the constructor implicitly calls
//! `Arc::downgrade`.
//!
//! Additionally, from an error handling perspective, every yellow block will have its own error
//! type, usually including a thin wrapper around the transport error.

use thiserror::Error;

pub mod bram;
pub mod snapadc;
pub mod snapshot;
pub mod swreg;
pub mod ten_gbe;

/// Certain Yellow Block struct types will implement this trait to allow for auto offsets in
/// transport read methods
pub trait Address {
    fn addr() -> u16;
}

#[derive(Error, Debug)]
/// Top level error for all yellow blocks (rarely used)
pub enum Error {
    #[error(transparent)]
    Bram(#[from] bram::Error),
    #[error(transparent)]
    SnapAdc(#[from] snapadc::Error),
    #[error(transparent)]
    Snapshot(#[from] snapshot::Error),
    #[error(transparent)]
    Swreg(#[from] swreg::Error),
    #[error(transparent)]
    TenGbE(#[from] ten_gbe::Error),
}
