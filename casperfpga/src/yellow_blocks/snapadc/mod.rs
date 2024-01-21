//! Interface to the HMCAD1511 ADCs on the SNAP board and their associated components like
//! synthesizer and clock switch
pub mod clockswitch;
pub mod controller;
pub mod hmcad1511;
pub mod lmx;

use self::{
    clockswitch::{
        ClockSwitch,
        Source,
    },
    controller::{
        Adc16,
        ChannelInput,
        ChipSelect,
    },
    hmcad1511::{
        LvdsDriveStrength,
        LvdsTermination,
    },
    lmx::Synth,
};
use crate::transport::Transport;
use std::sync::{
    Mutex,
    Weak,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Transport(#[from] crate::transport::Error),
    #[error(transparent)]
    Controller(#[from] controller::Error),
    #[error(transparent)]
    Clockswitch(#[from] clockswitch::Error),
    #[error("Invalid number of SNAP inputs from the fpg file")]
    BadSnapInputs,
    #[error("Only the  8 bit resolution HMCAD1511 is supported - PRs welcome :)")]
    BadAdcResolution,
    #[error("Bad sample rate from the fpg file")]
    BadSampleRate,
}

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
    pub sample_rate: f64,
    /// Channel mode for each chip
    pub mode: AdcMode,
    /// Clock source,
    pub source: Source,
    /// Clock Switch
    pub clksw: ClockSwitch<T>,
    /// LMX Synthesizer,
    pub synth: Synth<T>,
    /// ADC Controller
    pub controller: Adc16<T>,
    /// Register name
    _name: String,
}

impl<T> SnapAdc<T>
where
    T: Transport,
{
    const RAM0_NAME: &'static str = "adc16_wb_ram0";
    const RAM1_NAME: &'static str = "adc16_wb_ram1";
    const RAM2_NAME: &'static str = "adc16_wb_ram2";

    /// Builds a [`SnapAdc`] from FPG description strings
    /// # Errors
    /// Returns an error on bad string arguments
    pub fn from_fpg(
        transport: Weak<Mutex<T>>,
        reg_name: &str,
        adc_resolution: &str,
        sample_rate: &str,
        snap_inputs: &str,
        clock_src: &str,
    ) -> Result<Self, Error> {
        let mode = match snap_inputs {
            "12" => AdcMode::Quad,
            "6" => AdcMode::Dual,
            "3" => AdcMode::Single,
            _ => return Err(Error::BadSnapInputs),
        };
        if adc_resolution != "8" {
            return Err(Error::BadAdcResolution);
        }
        let clksw = ClockSwitch::new(transport.clone());
        let synth = Synth::new(transport.clone());
        let controller = Adc16::new(transport.clone());
        let source = match clock_src {
            "sys_clk" => Source::Internal,
            _ => Source::External,
        };
        Ok(Self {
            transport,
            sample_rate: sample_rate.parse().map_err(|_| Error::BadSampleRate)?,
            mode,
            clksw,
            synth,
            controller,
            _name: reg_name.to_string(),
            source,
        })
    }

    /// Request a snapshot of `chip`
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn snapshot(&self, chip: SnapAdcChip) -> Result<[u8; 1024], Error> {
        // Request the snapshot
        self.controller.snap_req()?;
        // Then read the BRAM
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        Ok(transport.read_bytes(
            match chip {
                SnapAdcChip::A => Self::RAM0_NAME,
                SnapAdcChip::B => Self::RAM1_NAME,
                SnapAdcChip::C => Self::RAM2_NAME,
            },
            0,
        )?)
    }

    /// Initializes the ADCs - follow this up by setting the controller crossbar and calibrating
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn initialize(&mut self) -> Result<(), Error> {
        // Start off with a reset
        self.controller.reset()?;
        // Chip select all the ADCs in the SNAP
        self.controller.chip_select(&ChipSelect::select_all());
        // Set the clock switch based on the source
        self.clksw.set_source(self.source)?;
        // If we're using the LMX synthesizer (Internal source), set that up
        if self.source == Source::Internal {
            todo!()
        }
        // Initialize the ADCs (this does a reset, power cycles, and sets the modes)
        self.controller.init(self.mode, self.sample_rate)?;
        // Set the termination and drive strength on two out of the three ADCs as the clock is only
        // sourced from adc0
        self.controller.chip_select(&ChipSelect {
            b: true,
            c: true,
            ..Default::default()
        });
        // LCLK and Frame to 94 Ohms
        self.controller.set_terminations(
            LvdsTermination::_94,
            LvdsTermination::_94,
            LvdsTermination::default(),
        )?;
        // LCLK and Frame to 0.5 mA
        self.controller.set_drive_strength(
            LvdsDriveStrength::_0_5,
            LvdsDriveStrength::_0_5,
            LvdsDriveStrength::default(),
        )?;
        // And back to select all
        self.controller.chip_select(&ChipSelect::select_all());

        // Calibrate here maybe?

        // Setup the FPGA-side demux
        self.controller.set_demux(match self.mode {
            AdcMode::Single => controller::DemuxMode::SingleChannel,
            AdcMode::Dual => controller::DemuxMode::DualChannel,
            AdcMode::Quad => controller::DemuxMode::QuadChannel,
        })?;
        Ok(())
    }

    /// Set the crossbars - ensures we match the number of channels
    /// # Errors
    /// Returns an error on bad transport
    /// # Panics
    /// Panics if the given input selection does not match the current mode
    pub fn select_inputs(&self, inputs: ChannelInput) -> Result<(), Error> {
        // Extract channel mode and assert
        match self.mode {
            AdcMode::Single => assert!(matches!(inputs, ChannelInput::Single(_))),
            AdcMode::Dual => assert!(matches!(inputs, ChannelInput::Dual(_, _))),
            AdcMode::Quad => assert!(matches!(inputs, ChannelInput::Quad(_, _, _, _))),
        };
        // Then set
        Ok(self.controller.input_select(inputs)?)
    }
}

#[derive(Debug, Copy, Clone)]
/// Enumerates the three ADC chips on the SNAP platform
pub enum SnapAdcChip {
    A = 0,
    B = 1,
    C = 2,
}
