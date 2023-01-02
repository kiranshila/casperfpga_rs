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
        Adc16Controller,
        ChannelInput,
        ChipSelect,
    },
    hmcad1511::{
        LvdsDriveStrength,
        LvdsTermination,
    },
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
    pub sample_rate: f64,
    /// Channel mode for each chip
    pub mode: AdcMode,
    /// Clock source,
    pub source: Source,
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
    const RAM0_NAME: &str = "adc16_wb_ram0";
    const RAM1_NAME: &str = "adc16_wb_ram1";
    const RAM2_NAME: &str = "adc16_wb_ram2";

    pub fn from_fpg(
        transport: Weak<Mutex<T>>,
        reg_name: &str,
        adc_resolution: &str,
        sample_rate: &str,
        snap_inputs: &str,
        clock_src: &str,
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
        let source = match clock_src {
            "sys_clk" => Source::Internal,
            _ => Source::External,
        };
        Ok(Self {
            transport,
            sample_rate: sample_rate.parse()?,
            mode,
            clksw,
            synth,
            controller,
            name: reg_name.to_string(),
            source,
        })
    }

    /// Request a snapshot of `chip`
    pub fn snapshot(&self, chip: SnapAdcChip) -> anyhow::Result<[u8; 1024]> {
        // Request the snapshot
        self.controller.snap_req()?;
        // Then read the BRAM
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        transport.read_bytes(
            match chip {
                SnapAdcChip::A => Self::RAM0_NAME,
                SnapAdcChip::B => Self::RAM1_NAME,
                SnapAdcChip::C => Self::RAM2_NAME,
            },
            0,
        )
    }

    /// Initializes the ADCs - follow this up by setting the controller crossbar and calibrating
    pub fn initialize(&mut self) -> anyhow::Result<()> {
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
    pub fn select_inputs(&self, inputs: ChannelInput) -> anyhow::Result<()> {
        // Extract channel mode and assert
        match self.mode {
            AdcMode::Single => assert!(matches!(inputs, ChannelInput::Single(_))),
            AdcMode::Dual => assert!(matches!(inputs, ChannelInput::Dual(_, _))),
            AdcMode::Quad => assert!(matches!(inputs, ChannelInput::Quad(_, _, _, _))),
        };
        // Then set
        self.controller.input_select(inputs)
    }
}

#[derive(Debug, Copy, Clone)]
/// Enumerates the three ADC chips on the SNAP platform
pub enum SnapAdcChip {
    A = 0,
    B = 1,
    C = 2,
}
