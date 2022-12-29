//! Implementations of the "ADC16" controller, as specified
//! by Dave MacMahon's [Ruby Implementation](https://github.com/david-macmahon/casper_adc16/blob/master/ruby/lib/adc16.rb).
//!
//! This device controls and manages multiple HMCAD1511 ADCs

use super::hmcad1511::{
    Reset,
    SleepPd,
};
use crate::{
    transport::{
        Deserialize,
        Serialize,
        Transport,
    },
    yellow_blocks::Address,
};
use anyhow::bail;
use casperfpga_derive::{
    address,
    CasperSerde,
};
use packed_struct::prelude::*;
use std::sync::{
    Mutex,
    Weak,
};

/// Controller for the ADC chips themselves
#[derive(Debug)]
pub struct Adc16Controller<T> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
    /// Holds the current chip select state,
    cs: ChipSelect,
}

impl<T> Adc16Controller<T>
where
    T: Transport,
{
    const NAME: &'static str = "adc16_controller";

    pub fn new(transport: Weak<Mutex<T>>) -> Self {
        Self {
            transport,
            cs: Default::default(),
        }
    }

    /// Gets the number of ADC chips this controller supports
    pub fn supported_chips(&self) -> anyhow::Result<u8> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let word: Adc3Wire = transport.read_addr(Self::NAME)?;
        Ok(word.supported_chips.into())
    }

    /// Gets the controller revision
    pub fn revision(&self) -> anyhow::Result<u8> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        let word: Adc3Wire = transport.read_addr(Self::NAME)?;
        Ok(word.revision.into())
    }

    /// Checks to see if the ADCs are locked
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

    /// Cursed bit-banging to send a bit to the current chip select
    fn send_3wire_bit(&self, bit: bool) -> anyhow::Result<()> {
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
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

    /// Cursed bit-banging to send an ADC register over the 3 wire to the current chip select
    fn send_reg<R>(&self, reg: R) -> anyhow::Result<()>
    where
        R: Address + PackedStruct,
    {
        // Convert the register into an address and data to bitbang
        let addr = R::addr();
        let mut packed = [0u8; 2];
        reg.pack_to_slice(&mut packed)?;
        let value = u16::from_be_bytes(packed);
        // Grab the transport context
        let tarc = self.transport.upgrade().unwrap();
        let mut transport = (*tarc).lock().unwrap();
        // Idle
        transport.write_addr(Self::NAME, &Adc3Wire::idle())?;
        // Write the address
        for i in (0..=7).rev() {
            self.send_3wire_bit(((addr >> i) & 1) == 1)?;
        }
        // And the data
        for i in (0..=15).rev() {
            self.send_3wire_bit(((value >> i) & 1) == 1)?;
        }
        // Then back to idle
        transport.write_addr(Self::NAME, &Adc3Wire::idle())?;
        Ok(())
    }

    /// Checks if the gateware supports demultiplexing modes
    /// Demultiplexing modes are used when running the ADC16 in dual and quad
    /// channel configurations
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
    ///
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
    pub fn reset(&self) -> anyhow::Result<()> {
        self.send_reg(Reset { reset: true })
    }

    /// Power cycles all the ADCs selected by the current chip select
    pub fn power_cycle(&mut self) -> anyhow::Result<()> {
        // Powerdown
        self.send_reg(SleepPd {
            pd: true,
            ..Default::default()
        })?;
        // Store old chip select
        let old_cs = self.cs;
        // Power up one chip at a time
        for i in 0..=7 {
            self.cs = ChipSelect::by_number(i);
            // Powerup
            self.send_reg(SleepPd::default())?;
        }
        // Restore old cs
        self.cs = old_cs;
        Ok(())
    }
}

#[derive(PackedStruct, Default, Debug, PartialEq, Copy, Clone)]
#[packed_struct(bit_numbering = "msb0")]
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

#[derive(PrimitiveEnum, Clone, Copy, PartialEq, Debug, Default)]
pub enum DemuxMode {
    #[default]
    SingleChannel = 0,
    DualChannel = 1,
    QuadChannel = 2,
}

#[derive(PackedStruct, Default, Debug, PartialEq)]
#[packed_struct(bit_numbering = "msb0")]
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
