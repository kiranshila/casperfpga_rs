use crate::transport::Transport;
use std::sync::{
    Mutex,
    Weak,
};

/// Clock source for SNAP ADCs
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Source {
    /// Internal (using the LMX synth)
    Internal,
    /// External
    External,
}

#[derive(Debug)]
pub struct ClockSwitch<T> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
}

impl<T> ClockSwitch<T>
where
    T: Transport,
{
    const NAME: &'static str = "adc16_use_synth";

    #[must_use]
    pub fn new(transport: Weak<Mutex<T>>) -> Self {
        Self { transport }
    }

    /// Sets the source of the clock switch
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn set_source(&self, source: Source) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        match source {
            Source::Internal => transport.write(Self::NAME, 0, &1u32),
            Source::External => transport.write(Self::NAME, 0, &0u32),
        }
    }

    /// Gets the source of the clock switch
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn get_source(&self) -> anyhow::Result<Source> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let raw: u32 = transport.read(Self::NAME, 0)?;
        Ok(match raw {
            1 => Source::Internal,
            0 => Source::External,
            _ => unreachable!(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core::Register,
        transport::mock::Mock,
    };
    use std::{
        collections::HashMap,
        sync::Arc,
    };

    #[test]
    fn test_clock_switch() {
        let transport = Mock::new(HashMap::from([(
            "adc16_use_synth".into(),
            Register { addr: 0, length: 4 },
        )]));
        let transport = Arc::new(Mutex::new(transport));
        let cksw = ClockSwitch::new(Arc::downgrade(&transport));
        cksw.set_source(Source::External).unwrap();
        assert_eq!(cksw.get_source().unwrap(), Source::External);
        cksw.set_source(Source::Internal).unwrap();
        assert_eq!(cksw.get_source().unwrap(), Source::Internal);
    }
}
