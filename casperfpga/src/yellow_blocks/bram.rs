use crate::transport::Transport;
use fixed::traits::Fixed;
use std::{
    marker::PhantomData,
    sync::{
        Arc,
        Mutex,
        Weak,
    },
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Transport(#[from] crate::transport::Error),
    #[error("Out of bounds addressing")]
    OutOfBounds,
    #[error("Size of given data doesn't fit the target")]
    BadSize,
    #[error("Failed to parse addr_width from the fpg file")]
    BadAddrWidth,
}

/// The snapshot yellow block to capture a chunk of samples
#[derive(Debug)]
pub struct Bram<T, F> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
    /// The name of the register
    name: String,
    /// Marker for the integer type of the data type
    phantom: PhantomData<F>,
    // Size of the BRAM in number of words
    size: usize,
}

impl<T, F> Bram<T, F>
where
    T: Transport,
    F: Fixed,
{
    #[must_use]
    pub fn new(transport: &Arc<Mutex<T>>, reg_name: &str, size: usize) -> Self {
        let transport = Arc::downgrade(transport);
        Self {
            transport,
            name: reg_name.to_string(),
            phantom: PhantomData,
            size,
        }
    }

    /// Builds a [`Bram`] from fpg details
    /// # Errors
    /// Returns an error on bad string arguments
    pub fn from_fpg(
        transport: Weak<Mutex<T>>,
        reg_name: &str,
        addr_width: &str,
    ) -> Result<Self, Error> {
        Ok(Self {
            transport,
            name: reg_name.to_string(),
            phantom: PhantomData,
            size: 1
                << addr_width
                    .parse::<usize>()
                    .map_err(|_| Error::BadAddrWidth)?,
        })
    }
}

impl<T, F, const N: usize> Bram<T, F>
where
    T: Transport,
    F: Fixed<Bytes = [u8; N]>,
{
    /// Read one fixed point word at `addr` from the BRAM
    /// # Errors
    /// Returns an error on transport errors
    #[allow(clippy::missing_panics_doc)]
    pub fn read_addr(&self, addr: usize) -> Result<F, Error> {
        if addr >= self.size {
            return Err(Error::OutOfBounds);
        }
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        Ok(F::from_be_bytes(transport.read(&self.name, addr)?))
    }

    /// Reads the entire BRAM
    /// # Errors
    /// Returns an error on transport errors
    #[allow(clippy::missing_panics_doc)]
    pub fn read(&self) -> Result<Vec<F>, Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Read all the data
        let total_bytes = self.size * N;
        let v = transport.read_n_bytes(&self.name, 0, total_bytes)?;
        // Transform the vec of bytes to the vec of fixed point words
        Ok(v.chunks(N)
            .map(|c| F::from_be_bytes(c.try_into().unwrap()))
            .collect())
    }

    /// Write the entire BRAM
    /// # Errors
    /// Returns an error on transport errors or if the data is not the correct size
    #[allow(clippy::missing_panics_doc)]
    pub fn write(&self, data: &[F]) -> Result<(), Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Transform the vec of fixed point words to the vec of bytes
        let total_bytes = self.size * N;
        let v = data
            .iter()
            .flat_map(|f| f.to_be_bytes().to_vec())
            .collect::<Vec<_>>();
        if v.len() != total_bytes {
            return Err(Error::BadSize);
        }
        // Write all the data
        transport.write_bytes(&self.name, 0, &v)?;
        Ok(())
    }

    /// Write a fixed point word at `addr` to the BRAM
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn write_addr(&self, addr: usize, val: F) -> Result<(), Error> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Perform the write
        Ok(transport.write(&self.name, addr, &(val.to_be_bytes()))?)
    }
}
