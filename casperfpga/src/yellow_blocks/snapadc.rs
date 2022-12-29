//! Interface to the HMCAD1511 ADCs on the SNAP board and their associated components like
//! synthesizer and clock switch

use anyhow::bail;

use crate::transport::Transport;
use std::sync::{
    Mutex,
    Weak,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// Valid modes for each HMCAD1511 ADC
pub enum AdcMode {
    /// Single channel mode - Fsmax = 1000 Msps
    Single,
    /// Dual channel mode - Fsmax = 500 Msps
    Dual,
    /// Quad channel mode - Fsmax = 250 Msps
    Quad,
}

/// The HMCAD1511 ADCs on the SNAP platform
#[derive(Debug)]
pub struct SnapAdc<T> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
    /// Sample rate in MHz
    sample_rate: usize,
    /// Channel mode for each chip
    mode: AdcMode,
    /// Clock Switch
    clksw: ClockSwitch<T>,
    /// LMX Synthesizer,
    synth: LmxSynth<T>,
    /// ADC Controller
    controller: SnapAdcController<T>,
    /// Register name
    name: String,
}

impl<T> SnapAdc<T>
where
    T: Transport,
{
    pub fn from_fpg(
        transport: Weak<Mutex<T>>,
        reg_name: &str,
        adc_resolution: &str,
        sample_rate: &str,
        snap_inputs: &str,
    ) -> anyhow::Result<Self> {
        let mode = match snap_inputs {
            "12" => AdcMode::Quad,
            "6" => AdcMode::Dual,
            "3" => AdcMode::Single,
            _ => bail!("Invalid number of snap_inputs"),
        };
        if adc_resolution != "8" {
            bail!("Only the  8 bit resolution HMCAD1511 is supported - PRs welcome :)");
        }
        let clksw = ClockSwitch::new(transport.clone());
        let synth = LmxSynth::new(transport.clone());
        let controller = SnapAdcController::new(transport.clone());
        Ok(Self {
            transport,
            sample_rate: sample_rate.parse()?,
            mode,
            clksw,
            synth,
            controller,
            name: reg_name.to_string(),
        })
    }
}

/// Internal SNAP clock synthesizer - LMX2581
#[derive(Debug)]
pub struct LmxSynth<T> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
}

impl<T> LmxSynth<T>
where
    T: Transport,
{
    const NAME: &'static str = "lmx_ctrl";

    pub fn new(transport: Weak<Mutex<T>>) -> Self {
        Self { transport }
    }
}

/// Controller for the ADC chips themselves
#[derive(Debug)]
pub struct SnapAdcController<T> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
}

impl<T> SnapAdcController<T>
where
    T: Transport,
{
    const NAME: &'static str = "adc16_controller";

    pub fn new(transport: Weak<Mutex<T>>) -> Self {
        Self { transport }
    }
}

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

    pub fn new(transport: Weak<Mutex<T>>) -> Self {
        Self { transport }
    }

    pub fn set_source(&self, source: Source) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        match source {
            Source::Internal => transport.write(Self::NAME, 0, &1u32),
            Source::External => transport.write(Self::NAME, 0, &0u32),
        }
    }

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
