//! Interface to the HMCAD1511 ADCs on the SNAP board and their associated components like
//! synthesizer and clock switch
pub mod clockswitch;
pub mod controller;
pub mod hmcad1511;
pub mod lmx;

use self::{
    clockswitch::ClockSwitch,
    controller::Adc16Controller,
    lmx::LmxSynth,
};
use crate::transport::Transport;
use anyhow::bail;
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
    pub sample_rate: usize,
    /// Channel mode for each chip
    pub mode: AdcMode,
    /// Clock Switch
    pub clksw: ClockSwitch<T>,
    /// LMX Synthesizer,
    pub synth: LmxSynth<T>,
    /// ADC Controller
    pub controller: Adc16Controller<T>,
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
        let controller = Adc16Controller::new(transport.clone());
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
