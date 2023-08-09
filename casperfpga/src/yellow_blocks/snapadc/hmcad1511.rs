//! Register map for the HMCAD1511 ADC from Analog Devices
//! As far as KS can tell, this is *only* used for the SNAP platform, so many features may go
//! unimplemented.

use crate::yellow_blocks::Address;
use casperfpga_derive::address;
use packed_struct::prelude::*;

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x00)]
pub struct Reset {
    #[packed_field(bits = "0")]
    /// Self-clearing software reset
    pub(crate) reset: bool,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x0F)]
pub struct SleepPd {
    #[packed_field(bits = "0..=3")]
    pub(crate) sleep4: [bool; 4],
    #[packed_field(bits = "4..=5")]
    pub(crate) sleep2: [bool; 2],
    #[packed_field(bits = "6")]
    pub(crate) sleep1: bool,
    #[packed_field(bits = "8")]
    /// Go to sleep mode
    pub(crate) sleep: bool,
    #[packed_field(bits = "9")]
    /// Go to power down
    pub(crate) pd: bool,
    #[packed_field(bits = "10..=11", ty = "enum")]
    /// Configures the PD pin function
    pub(crate) pd_pin_cfg: PdPinCfg,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
pub enum PdPinCfg {
    SleepChannel = 1,
    DeepSleep = 2,
    #[default]
    PowerDown = 0,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x12)]
pub struct LvdsDrives {
    #[packed_field(bits = "0..=2", ty = "enum")]
    /// LVDS current drive for LCLK
    pub(crate) ilvds_lclk: LvdsDriveStrength,
    #[packed_field(bits = "4..=6", ty = "enum")]
    /// LVDS current drive for FCLK
    pub(crate) ilvds_frame: LvdsDriveStrength,
    #[packed_field(bits = "8..=10", ty = "enum")]
    /// LVDS current drive for output data
    pub(crate) ilvds_dat: LvdsDriveStrength,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// LVDS Current drive strength in mA
pub enum LvdsDriveStrength {
    #[default]
    _3_5 = 0,
    _2_5 = 1,
    _1_5 = 2,
    _0_5 = 3,
    _7_5 = 4,
    _6_5 = 5,
    _5_5 = 6,
    _4_5 = 7,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x11)]
pub struct LvdsTerminations {
    #[packed_field(bits = "14")]
    /// Enabled internal termination for LVDS buffers
    pub(crate) en_lvds_term: bool,
    #[packed_field(bits = "0..=2", ty = "enum")]
    /// LVDS termination for LCLK
    pub(crate) term_lclk: LvdsTermination,
    #[packed_field(bits = "4..=6", ty = "enum")]
    /// LVDS termination for FCLK
    pub(crate) term_frame: LvdsTermination,
    #[packed_field(bits = "8..=10", ty = "enum")]
    /// LVDS termination for output data
    pub(crate) term_dat: LvdsTermination,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// LVDS termination in ohms
pub enum LvdsTermination {
    #[default]
    Disabled = 0,
    _260 = 1,
    _150 = 2,
    _94 = 3,
    _125 = 4,
    _80 = 5,
    _66 = 6,
    _55 = 7,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x24)]
/// Specifies channel specific swapping of th analog input signal (positive and negative)
/// Defaults to non-swapped
pub struct InvertCtl {
    #[packed_field(bits = "0..=3")]
    pub(crate) invert4: [bool; 4],
    #[packed_field(bits = "4..=5")]
    pub(crate) invert2: [bool; 2],
    #[packed_field(bits = "6")]
    pub(crate) intert1: bool,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x25)]
/// Specifies channel specific swapping of th analog input signal (positive and negative)
/// Defaults to non-swapped
pub struct PatternCtl {
    #[packed_field(bits = "4..=6", ty = "enum")]
    pub(crate) pattern: Pattern,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// Output pattern type
pub enum Pattern {
    #[default]
    Disabled = 0,
    Ramp = 0b100,
    DualCustom = 0b010,
    SingleCustom = 0b001,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x26)]
/// Bits for custom pattern 1
pub struct CustomPattern1 {
    #[packed_field(bits = "8..=15")]
    pub(crate) bits_custom1: [bool; 8],
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x27)]
/// Bits for custom pattern 2
pub struct CustomPattern2 {
    #[packed_field(bits = "8..=15")]
    pub(crate) bits_custom2: [bool; 8],
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x2A)]
/// Programmable coarse gain in quad channel setup
pub struct QuadCoarseGains {
    #[packed_field(bits = "0..=3", ty = "enum")]
    pub(crate) cgain4_ch1: CoarseGain,
    #[packed_field(bits = "4..=7", ty = "enum")]
    pub(crate) cgain4_ch2: CoarseGain,
    #[packed_field(bits = "8..=11", ty = "enum")]
    pub(crate) cgain4_ch3: CoarseGain,
    #[packed_field(bits = "12..=15", ty = "enum")]
    pub(crate) cgain4_ch4: CoarseGain,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x2B)]
/// Programmable coarse gain in dual and single channel setup
pub struct DualCoarseGains {
    #[packed_field(bits = "0..=3", ty = "enum")]
    pub(crate) cgain2_ch1: CoarseGain,
    #[packed_field(bits = "4..=7", ty = "enum")]
    pub(crate) cgain2_ch2: CoarseGain,
    #[packed_field(bits = "8..=11", ty = "enum")]
    pub(crate) cgain1_ch1: CoarseGain,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// Coarse gain settings in dB and "gain factor" (not 1 to 1)
pub enum CoarseGain {
    #[default]
    /// 0dB - 1x
    _0 = 0,
    /// 1dB - 1.25x
    _1 = 1,
    /// 2dB - 2x
    _2 = 2,
    /// 3dB - 2.5x
    _3 = 3,
    /// 4dB - 4x
    _4 = 4,
    /// 5dB - 5x
    _5 = 5,
    /// 6dB - 8x
    _6 = 6,
    /// 7dB - 10x
    _7 = 7,
    /// 8dB - 12.5x
    _8 = 8,
    /// 9dB - 16x
    _9 = 9,
    /// 10dB - 20x
    _10 = 10,
    /// 11dB - 25x
    _11 = 11,
    /// 12dB - 32x
    _12 = 12,
    /// 50x
    X50 = 13,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x30)]
/// Clock jutter adjustment
pub struct JitterCtl {
    #[packed_field(bits = "0..=7", ty = "enum")]
    pub(crate) jitter_ctrl: Jitter,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// Jitter control that allows for a trade off between power consumption and clock jitter
pub enum Jitter {
    /// Clock stopped
    _0 = 0,
    #[default]
    /// 160 fsrms - 1 mA
    _1 = 0b0000_0001,
    /// 150 fsrms - 2 mA
    _2 = 0b0000_0011,
    /// 136 fsrms - 3 mA
    _3 = 0b0000_0111,
    /// 130 fsrms - 4 mA
    _4 = 0b0000_1111,
    /// 126 fsrms - 5 mA
    _5 = 0b0001_1111,
    /// 124 fsrms - 6 mA
    _6 = 0b0011_1111,
    /// 122 fsrms - 7 mA
    _7 = 0b0111_1111,
    /// 120 fsrms - 8 mA
    _8 = 0b1111_1111,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x31)]
/// Control for the number of channels and clock division factor
pub struct ChanNumClkDiv {
    #[packed_field(bits = "0..=2", ty = "enum")]
    pub(crate) channel_num: ChannelNum,
    #[packed_field(bits = "8..=9", ty = "enum")]
    pub(crate) clk_divide: ClockDivide,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// Number of channels
pub enum ChannelNum {
    /// Single channel by interleaving ADC1 to ADC4
    Single = 0b001,
    /// Dual channel where channel 1 is made by interleaving ADC1 and ADC2, channel 2 by
    /// interleaving ADC3 and ADC4
    Dual = 0b010,
    #[default]
    /// Quad channel where channel 1 corresponds to ADC1, channel2 to ADC2, channel3 to ADC3 and
    /// channel 4 to ADC4
    Quad = 0b100,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// Clock division factor
pub enum ClockDivide {
    #[default]
    /// Input clock / 1
    _1 = 0,
    /// Input clock / 2
    _2 = 1,
    /// Input clock / 4
    _4 = 2,
    /// Input clock / 8
    _8 = 3,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x33)]
/// Coarse and fine gain settings
pub struct GainCtl {
    #[packed_field(bits = "0")]
    pub(crate) coarse_gain_cfg: bool,
    #[packed_field(bits = "1")]
    pub(crate) fine_gain_en: bool,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x34)]
/// Fine gain control for branch 1 and 2
pub struct FineGain12 {
    #[packed_field(bits = "0..=6")]
    pub(crate) fgain_branch1: Integer<i8, packed_bits::Bits<7>>,
    #[packed_field(bits = "8..=14")]
    pub(crate) fgain_branch2: Integer<i8, packed_bits::Bits<7>>,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x35)]
/// Fine gain control for branch 3 and 4
pub struct FineGain34 {
    #[packed_field(bits = "0..=6")]
    pub(crate) fgain_branch3: Integer<i8, packed_bits::Bits<7>>,
    #[packed_field(bits = "8..=14")]
    pub(crate) fgain_branch4: Integer<i8, packed_bits::Bits<7>>,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x36)]
/// Fine gain control for branch 5 and 6
pub struct FineGain56 {
    #[packed_field(bits = "0..=6")]
    pub(crate) fgain_branch5: Integer<i8, packed_bits::Bits<7>>,
    #[packed_field(bits = "8..=14")]
    pub(crate) fgain_branch6: Integer<i8, packed_bits::Bits<7>>,
}

#[derive(Debug, PackedStruct, Default, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x37)]
/// Fine gain control for branch 7 and 8
pub struct FineGain78 {
    #[packed_field(bits = "0..=6")]
    pub(crate) fgain_branch7: Integer<i8, packed_bits::Bits<7>>,
    #[packed_field(bits = "8..=14")]
    pub(crate) fgain_branch8: Integer<i8, packed_bits::Bits<7>>,
}

#[derive(Debug, PackedStruct, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x3A)]
/// Input select for adc 1 and 2
pub struct InputSelect12 {
    #[packed_field(bits = "0..=4", ty = "enum")]
    pub(crate) inp_sel_adc1: InputSelect,
    #[packed_field(bits = "8..=12", ty = "enum")]
    pub(crate) inp_sel_adc2: InputSelect,
}

impl Default for InputSelect12 {
    fn default() -> Self {
        Self {
            inp_sel_adc1: InputSelect::_1,
            inp_sel_adc2: InputSelect::_2,
        }
    }
}

#[derive(Debug, PackedStruct, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x3B)]
/// Input select for adc 3 and 4
pub struct InputSelect34 {
    #[packed_field(bits = "0..=4", ty = "enum")]
    pub(crate) inp_sel_adc3: InputSelect,
    #[packed_field(bits = "8..=12", ty = "enum")]
    pub(crate) inp_sel_adc4: InputSelect,
}

impl Default for InputSelect34 {
    fn default() -> Self {
        Self {
            inp_sel_adc3: InputSelect::_3,
            inp_sel_adc4: InputSelect::_4,
        }
    }
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// Input select via cross point switch
pub enum InputSelect {
    #[default]
    /// IP1/IN1
    _1 = 0b00010,
    /// IP2/IN2
    _2 = 0b00100,
    /// IP3/IN3
    _3 = 0b01000,
    /// IP4/IN4
    _4 = 0b10000,
}

#[derive(Debug, PackedStruct, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x42)]
pub struct LclkPhase {
    #[packed_field(bits = "5..=6", ty = "enum")]
    pub(crate) phase_ddr: PhaseDdr,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// Control for the phase of LCLK relative to the output from clock and data bits
pub enum PhaseDdr {
    #[default]
    _90 = 2,
    _270 = 0,
    _180 = 1,
    _0 = 3,
}

#[derive(Debug, PackedStruct, Copy, Clone, Default)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x45)]
pub struct DeskewSyncPattern {
    #[packed_field(bits = "0..=1", ty = "enum")]
    pub(crate) pat_deskew_sync: DeskewSyncMode,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
pub enum DeskewSyncMode {
    #[default]
    Disabled = 0,
    Deskew = 1,
    Sync = 2,
}

#[derive(Debug, PackedStruct, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x46)]
pub struct OutputMode {
    #[packed_field(bits = "2")]
    pub(crate) btc_mode: bool,
    #[packed_field(bits = "3")]
    pub(crate) msb_first: bool,
}

#[derive(Debug, PackedStruct, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x50)]
pub struct AdcCurrentVcmDrive {
    #[packed_field(bits = "0..=2", ty = "enum")]
    pub(crate) adc_curr: AdcCurrentControl,
    #[packed_field(bits = "5..=6", ty = "enum")]
    pub(crate) ext_vcm_bc: VcmBufferDrive,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// Scales the current consumption down by the set percentage
pub enum AdcCurrentControl {
    _40 = 0b100,
    _30 = 0b101,
    _20 = 0b110,
    _10 = 0b111,
    #[default]
    _0 = 0b000,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// Sets the driving strength of the buffer supply voltage on the VCM pin
pub enum VcmBufferDrive {
    /// VCM Floating
    Off = 0b00,
    #[default]
    Pm20 = 0b01,
    Pm400 = 0b10,
    Pm700 = 0b11,
}

#[derive(Debug, PackedStruct, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x52)]
pub struct LvdsPowerDown {
    #[packed_field(bits = "0")]
    pub(crate) lvds_pd_mode: bool,
}

#[derive(Debug, PackedStruct, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x53)]
pub struct LvdsOutputControl {
    #[packed_field(bits = "3")]
    pub(crate) low_clk_freq: bool,
    #[packed_field(bits = "4..=5", ty = "enum")]
    pub(crate) lvds_shift: LvdsShift,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// Shift the propagation delay of the LVDS data forwards or backwards
pub enum LvdsShift {
    #[default]
    Disabled = 0,
    Delay = 0b10,
    Advance = 0b01,
}

#[derive(Debug, PackedStruct, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x55)]
pub struct FullScaleRangeControl {
    #[packed_field(bits = "0..=5")]
    /// "Signed" 5 bit number with each increment changing by 0.3%
    pub(crate) fs_cntrl: Integer<i8, packed_bits::Bits<6>>,
}

#[derive(Debug, PackedStruct, Copy, Clone)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "2")]
#[address(0x56)]
pub struct StartupControl {
    #[packed_field(bits = "0..=2", ty = "enum")]
    pub(crate) startup_ctrl: StartupTiming,
}

#[derive(Debug, PrimitiveEnum, Default, Copy, Clone)]
/// Startup timimng - channel count dependent, just read the data sheet here
pub enum StartupTiming {
    #[default]
    Default = 0,
    _100 = 0b100,
    _001 = 0b001,
    _101 = 0b101,
    _011 = 0b011,
}
