//! Implementations of the "ADC16" controller, as specified
//! by Dave's [Ruby Implementation](https://github.com/david-macmahon/casper_adc16/blob/master/ruby/lib/adc16.rb).
//!
//! This device controls and manages multiple HMCAD1511 ADCs

#[allow(clippy::wildcard_imports)]
use super::{hmcad1511::*, AdcMode};
use crate::{
    transport::{Deserialize, Serialize, Transport},
    yellow_blocks::Address,
};
use anyhow::bail;
use casperfpga_derive::{address, CasperSerde};
use packed_struct::prelude::*;
use std::sync::{Mutex, Weak};

/// Controller for the ADC chips themselves
#[derive(Debug)]
pub struct Adc16<T> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
    /// Holds the current chip select state,
    cs: ChipSelect,
}

impl<T> Adc16<T>
where
    T: Transport,
{
    const NAME: &'static str = "adc16_controller";

    #[must_use]
    pub fn new(transport: Weak<Mutex<T>>) -> Self {
        Self {
            transport,
            cs: ChipSelect::default(),
        }
    }

    /// Gets the number of ADC chips this controller supports
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn supported_chips(&self) -> anyhow::Result<u8> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let word: Adc3Wire = transport.read_addr(Self::NAME)?;
        Ok(word.supported_chips.into())
    }

    /// Gets the controller revision
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn revision(&self) -> anyhow::Result<u8> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let word: Adc3Wire = transport.read_addr(Self::NAME)?;
        Ok(word.revision.into())
    }

    /// Checks to see if the ADCs are locked
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn locked(&self) -> anyhow::Result<bool> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let word: Adc3Wire = transport.read_addr(Self::NAME)?;
        let ll: u8 = word.line_lock.into();
        let num_adcs: u8 = word.supported_chips.into();
        Ok(match ll {
            0 | 2 => false,
            1 => num_adcs <= 4,
            3 => true,
            _ => unreachable!(),
        })
    }

    /// Sets the chip select state of the controller
    pub fn chip_select(&mut self, cs: &ChipSelect) {
        self.cs = *cs;
    }

    /// Cursed bit-banging to send a bit to the current chip select taking a mutable transport ref
    /// # Errors
    /// Returns an error on bad transport
    fn send_3wire_bit(&self, transport: &mut T, bit: bool) -> anyhow::Result<()> {
        // Clock low, data and chip select set accordingly
        transport.write_addr(
            Self::NAME,
            &Adc3Wire {
                sclk: false,
                sdata: bit,
                chip_select: self.cs,
                ..Default::default()
            },
        )?;
        // Clock high, data and chip selects set accordingly
        transport.write_addr(
            Self::NAME,
            &Adc3Wire {
                sclk: true,
                sdata: bit,
                chip_select: self.cs,
                ..Default::default()
            },
        )?;
        Ok(())
    }

    fn send_reg_raw(&self, transport: &mut T, addr: u8, val: u16) -> anyhow::Result<()> {
        // Idle
        transport.write_addr(Self::NAME, &Adc3Wire::idle())?;
        // Write the address
        for i in (0..=7).rev() {
            self.send_3wire_bit(transport, ((addr >> i) & 1) == 1)?;
        }
        // And the data
        for i in (0..=15).rev() {
            self.send_3wire_bit(transport, ((val >> i) & 1) == 1)?;
        }
        // Idle
        transport.write_addr(Self::NAME, &Adc3Wire::idle())?;
        Ok(())
    }

    /// Cursed bit-banging to send an ADC register over the 3 wire to the current chip select
    fn send_reg<R>(&self, transport: &mut T, reg: &R) -> anyhow::Result<()>
    where
        R: Address + PackedStruct,
    {
        // Convert the register into an address and data to bitbang
        let addr = R::addr();
        let mut packed = [0u8; 2];
        reg.pack_to_slice(&mut packed)?;
        let value = u16::from_be_bytes(packed);
        self.send_reg_raw(
            transport,
            addr.try_into().expect("Address didn't fit in a u8"),
            value,
        )
    }

    /// Checks if the gateware supports demultiplexing modes
    /// Demultiplexing modes are used when running the ADC16 in dual and quad
    /// channel configurations
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn supports_demux(&self) -> anyhow::Result<bool> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Check to see if we support demux by testing the demux write enable bit
        // If we /can't/ set it, we /do/ support demux
        transport.write_addr(
            Self::NAME,
            &AdcControl {
                demux_write_enable: true,
                ..Default::default()
            },
        )?;
        let demux_test: AdcControl = transport.read_addr(Self::NAME)?;
        // If we were able to set the bit, we do not support demux modes
        Ok(!demux_test.demux_write_enable)
    }

    /// Gets the current demux mode if the gateware supports it, otherwise returns None
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn get_demux(&self) -> anyhow::Result<Option<DemuxMode>> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        Ok(if self.supports_demux()? {
            let demux_test: AdcControl = transport.read_addr(Self::NAME)?;
            Some(demux_test.demux_mode)
        } else {
            None
        })
    }

    /// Sets the current demux mode
    /// Returns an error if the gateware doesn't support demux modes.
    /// # Errors
    /// Returns an error on bad transport
    /// ### Words of wisdom from Dave
    /// Note that setting the demux mode here only affects the demultiplexing of
    /// the data from the ADC before presenting it to the FPGA fabric.  The
    /// demultiplexing mode set does NOT set the "mode of operation" of the ADC
    /// chips.  That must be done by the user when initializing the ADC16 chips
    /// because it requires a software power down of the ADC chip.  The user is
    /// responsible for ensuring that the "mode of operation" set in the ADC chips
    /// at initialization time is consistent with the demux mode set using this
    /// method.  Mismatches will result in improper interpretation of the data. method.
    /// Mismatches will result in improper interpretation of the data.
    #[allow(clippy::missing_panics_doc)]
    pub fn set_demux(&self, mode: DemuxMode) -> anyhow::Result<()> {
        if self.supports_demux()? {
            let tarc = self.transport.upgrade().unwrap();
            let mut transport = (*tarc).lock().unwrap();
            // Grab the current state of the control register
            let mut ctl: AdcControl = transport.read_addr(Self::NAME)?;
            ctl.demux_mode = mode;
            // Write the update
            transport.write_addr(Self::NAME, &ctl)?;
            Ok(())
        } else {
            bail!("Current gateware doesn't support demux modes");
        }
    }

    /// Resets all the chips selected by the current chip select
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn reset(&self) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        self.send_reg(&mut transport, &Reset { reset: true })
    }

    /// Power down the ADCs by setting the pd bit
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn power_down(&self) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Powerdown
        self.send_reg(
            &mut transport,
            &SleepPd {
                pd: true,
                ..Default::default()
            },
        )
    }

    /// Power up the ADCs by setting the pd bit
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn power_up(&self) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Powerdown
        self.send_reg(
            &mut transport,
            &SleepPd {
                pd: false,
                ..Default::default()
            },
        )
    }

    /// Power cycles all the ADCs selected by the current chip select
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn power_cycle(&mut self) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Powerdown
        self.send_reg(
            &mut transport,
            &SleepPd {
                pd: true,
                ..Default::default()
            },
        )?;
        // Store old chip select
        let old_cs = self.cs;
        // Power up one chip at a time
        for i in 0..=7 {
            self.cs = ChipSelect::by_number(i);
            // Powerup
            self.send_reg(&mut transport, &SleepPd::default())?;
        }
        // Restore old cs
        self.cs = old_cs;
        Ok(())
    }

    /// Selects a test pattern or sampled data for all the adc currently selected
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn enable_pattern(&self, pat: TestPattern) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        self.send_reg(&mut transport, &PatternCtl::default())?;
        self.send_reg(&mut transport, &DeskewSyncPattern::default())?;
        match pat {
            TestPattern::Ramp => self.send_reg(
                &mut transport,
                &PatternCtl {
                    pattern: Pattern::Ramp,
                },
            ),
            TestPattern::Deskew => self.send_reg(
                &mut transport,
                &DeskewSyncPattern {
                    pat_deskew_sync: DeskewSyncMode::Deskew,
                },
            ),
            TestPattern::Sync => self.send_reg(
                &mut transport,
                &DeskewSyncPattern {
                    pat_deskew_sync: DeskewSyncMode::Sync,
                },
            ),
            TestPattern::Custom1 | TestPattern::Custom2 => self.send_reg(
                &mut transport,
                &PatternCtl {
                    pattern: Pattern::SingleCustom,
                },
            ),
            TestPattern::Dual => self.send_reg(
                &mut transport,
                &PatternCtl {
                    pattern: Pattern::DualCustom,
                },
            ),
            TestPattern::None => Ok(()),
        }
    }

    /// Set the "Custom 1 " pattern
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn custom_1(&self, bits: [bool; 8]) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        self.send_reg(&mut transport, &CustomPattern1 { bits_custom1: bits })
    }

    /// Set the "Custom 2 " pattern
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn custom_2(&self, bits: [bool; 8]) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        self.send_reg(&mut transport, &CustomPattern2 { bits_custom2: bits })
    }

    /// Perform a bitslip operation on specified chips
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn bitslip(&self, bitslips: Bitslip) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let slip = AdcControl {
            bitslip: bitslips,
            ..Default::default()
        };
        transport.write_addr(Self::NAME, &AdcControl::default())?;
        transport.write_addr(Self::NAME, &slip)?;
        transport.write_addr(Self::NAME, &AdcControl::default())?;
        Ok(())
    }

    /// Request a snapshot - reads from the corresponding BRAM happen elsewhere
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn snap_req(&self) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Request the snapshot
        let snap_req = AdcControl {
            snap_request: true,
            ..Default::default()
        };
        transport.write_addr(Self::NAME, &AdcControl::default())?;
        transport.write_addr(Self::NAME, &snap_req)?;
        transport.write_addr(Self::NAME, &AdcControl::default())?;
        Ok(())
    }

    /// Set the operating mode along with the clock frequency in megahertz
    /// We will *always* set the clock divide to 1, as is done in the python. Wouldn't be that bad
    /// to change, but would need manual intervention at init time.
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn set_operating_mode(&self, mode: AdcMode, freq: f64) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();

        // Determine if we need to set the low frequency bit
        let low_clk = LvdsOutputControl {
            low_clk_freq: ((mode == AdcMode::Single) && (freq < 240.))
                || ((mode == AdcMode::Dual) && (freq < 120.))
                || ((mode == AdcMode::Quad) && (freq < 60.)),
            lvds_shift: LvdsShift::Disabled,
        };

        // Match mode with the corresponding register
        let chan_cfg = ChanNumClkDiv {
            channel_num: match mode {
                AdcMode::Single => ChannelNum::Single,
                AdcMode::Dual => ChannelNum::Dual,
                AdcMode::Quad => ChannelNum::Quad,
            },
            clk_divide: ClockDivide::_1,
        };

        // Send all the bits
        self.send_reg(&mut transport, &chan_cfg)?;
        self.send_reg(&mut transport, &low_clk)?;

        Ok(())
    }

    /// Startup the ADCs into a clean slate
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn init(&mut self, mode: AdcMode, freq: f64) -> anyhow::Result<()> {
        self.reset()?;
        self.power_down()?;
        self.set_operating_mode(mode, freq)?;
        self.power_up()?;
        Ok(())
    }

    /// Set the crossbars in the chip selected adc
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn input_select(&self, inputs: ChannelInput) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Make the selections
        let mut selections = [InputSelect::default(); 4];
        match inputs {
            ChannelInput::Single(a) => {
                selections[0] = a;
                selections[1] = a;
                selections[2] = a;
                selections[3] = a;
            }
            ChannelInput::Dual(a, b) => {
                selections[0] = a;
                selections[1] = a;
                selections[2] = b;
                selections[3] = b;
            }
            ChannelInput::Quad(a, b, c, d) => {
                selections[0] = a;
                selections[1] = b;
                selections[2] = c;
                selections[3] = d;
            }
        }
        // Write the inputs
        self.send_reg(
            &mut transport,
            &InputSelect12 {
                inp_sel_adc1: selections[0],
                inp_sel_adc2: selections[1],
            },
        )?;
        self.send_reg(
            &mut transport,
            &InputSelect34 {
                inp_sel_adc3: selections[2],
                inp_sel_adc4: selections[3],
            },
        )?;
        Ok(())
    }

    /// Disable LVDS terminations
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn disable_termination(&self) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        self.send_reg(
            &mut transport,
            &LvdsTerminations {
                en_lvds_term: false,
                ..Default::default()
            },
        )
    }

    /// Set the three LVDS terminations
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn set_terminations(
        &self,
        lclk: LvdsTermination,
        frame: LvdsTermination,
        data: LvdsTermination,
    ) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        self.send_reg(
            &mut transport,
            &LvdsTerminations {
                en_lvds_term: true,
                term_lclk: lclk,
                term_frame: frame,
                term_dat: data,
            },
        )
    }

    /// Set the LVDS drive strengths
    /// # Errors
    /// Returns an error on bad transport
    #[allow(clippy::missing_panics_doc)]
    pub fn set_drive_strength(
        &self,
        lclk: LvdsDriveStrength,
        frame: LvdsDriveStrength,
        data: LvdsDriveStrength,
    ) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        self.send_reg(
            &mut transport,
            &LvdsDrives {
                ilvds_lclk: lclk,
                ilvds_frame: frame,
                ilvds_dat: data,
            },
        )
    }
}

#[derive(Debug, Copy, Clone)]
/// Crossbar selections for given input modes
pub enum ChannelInput {
    /// All interleaved - one input for all cores
    Single(InputSelect),
    //// ADCs 1 and 2 then 3 and 4 interleaved, each pair shares input
    Dual(InputSelect, InputSelect),
    /// ADCs 1 through 4 have independent inputs
    Quad(InputSelect, InputSelect, InputSelect, InputSelect),
}

#[derive(Debug, Copy, Clone, Default)]
/// Test patterns to enable
pub enum TestPattern {
    #[default]
    /// Ramp from 0 to 255
    Ramp,
    /// Deskew (10101010)
    Deskew,
    /// Sync (11110000)
    Sync,
    Custom1,
    Custom2,
    Dual,
    /// Sampled data
    None,
}

#[derive(PackedStruct, Default, Debug, PartialEq, Eq, Copy, Clone)]
#[packed_struct(bit_numbering = "msb0")]
#[allow(clippy::struct_excessive_bools)]
pub struct ChipSelect {
    #[packed_field(bits = "7")]
    pub a: bool,
    #[packed_field(bits = "6")]
    pub b: bool,
    #[packed_field(bits = "5")]
    pub c: bool,
    #[packed_field(bits = "4")]
    pub d: bool,
    #[packed_field(bits = "3")]
    pub e: bool,
    #[packed_field(bits = "2")]
    pub f: bool,
    #[packed_field(bits = "1")]
    pub g: bool,
    #[packed_field(bits = "0")]
    pub h: bool,
}

impl ChipSelect {
    /// Select every ADC this controller supports
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn select_all() -> Self {
        Self::unpack_from_slice(&[0b1111_1111]).unwrap()
    }

    fn by_number(v: u8) -> Self {
        match v {
            0 => Self {
                a: true,
                ..Default::default()
            },
            1 => Self {
                b: true,
                ..Default::default()
            },
            2 => Self {
                c: true,
                ..Default::default()
            },
            3 => Self {
                d: true,
                ..Default::default()
            },
            4 => Self {
                e: true,
                ..Default::default()
            },
            5 => Self {
                f: true,
                ..Default::default()
            },
            6 => Self {
                g: true,
                ..Default::default()
            },
            7 => Self {
                h: true,
                ..Default::default()
            },
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PackedStruct, CasperSerde, Default)]
#[address(0x0)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "4")]
pub struct Adc3Wire {
    #[packed_field(bits = "6..=7")]
    line_lock: Integer<u8, packed_bits::Bits<2>>,
    #[packed_field(bits = "8..=11")]
    supported_chips: Integer<u8, packed_bits::Bits<4>>,
    #[packed_field(bits = "14..=15")]
    revision: Integer<u8, packed_bits::Bits<2>>,
    #[packed_field(bits = "22")]
    sclk: bool,
    #[packed_field(bits = "23")]
    sdata: bool,
    #[packed_field(bits = "24..=31")]
    chip_select: ChipSelect,
}

impl Adc3Wire {
    /// Returns an IDLE 3-wire state
    fn idle() -> Self {
        Self {
            sclk: true,
            ..Default::default()
        }
    }
}

#[derive(PrimitiveEnum, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DemuxMode {
    #[default]
    SingleChannel = 0,
    DualChannel = 1,
    QuadChannel = 2,
}

#[derive(PackedStruct, Default, Debug, PartialEq, Eq)]
#[packed_struct(bit_numbering = "msb0")]
#[allow(clippy::struct_excessive_bools)]
pub struct Bitslip {
    #[packed_field(bits = "7")]
    a: bool,
    #[packed_field(bits = "6")]
    b: bool,
    #[packed_field(bits = "5")]
    c: bool,
    #[packed_field(bits = "4")]
    d: bool,
    #[packed_field(bits = "3")]
    e: bool,
    #[packed_field(bits = "2")]
    f: bool,
    #[packed_field(bits = "1")]
    g: bool,
    #[packed_field(bits = "0")]
    h: bool,
}

#[derive(Debug, PackedStruct, CasperSerde, Default)]
#[address(0x4)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "4")]
pub struct AdcControl {
    #[packed_field(bits = "5")]
    demux_write_enable: bool,
    #[packed_field(bits = "6..=7", ty = "enum")]
    demux_mode: DemuxMode,
    #[packed_field(bits = "11")]
    reset: bool,
    #[packed_field(bits = "15")]
    snap_request: bool,
    #[packed_field(bits = "16..=23")]
    bitslip: Bitslip,
    #[packed_field(bits = "27..=31")]
    delay_taps: [bool; 5],
}

#[derive(Debug, PackedStruct, CasperSerde)]
#[address(0x8)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "4")]
pub struct AdcDelayAStrobe {
    #[packed_field(bits = "0..=3")]
    h: [bool; 4],
    #[packed_field(bits = "4..=7")]
    g: [bool; 4],
    #[packed_field(bits = "8..=11")]
    f: [bool; 4],
    #[packed_field(bits = "12..=15")]
    e: [bool; 4],
    #[packed_field(bits = "16..=19")]
    d: [bool; 4],
    #[packed_field(bits = "20..=23")]
    c: [bool; 4],
    #[packed_field(bits = "24..=27")]
    b: [bool; 4],
    #[packed_field(bits = "28..=31")]
    a: [bool; 4],
}

#[derive(Debug, PackedStruct, CasperSerde)]
#[address(0xC)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "4")]
pub struct AdcDelayBStrobe {
    #[packed_field(bits = "0..=3")]
    h: [bool; 4],
    #[packed_field(bits = "4..=7")]
    g: [bool; 4],
    #[packed_field(bits = "8..=11")]
    f: [bool; 4],
    #[packed_field(bits = "12..=15")]
    e: [bool; 4],
    #[packed_field(bits = "16..=19")]
    d: [bool; 4],
    #[packed_field(bits = "20..=23")]
    c: [bool; 4],
    #[packed_field(bits = "24..=27")]
    b: [bool; 4],
    #[packed_field(bits = "28..=31")]
    a: [bool; 4],
}
