//! Analog to digital converter configuration.
//!
//! # Status
//! Most options relating to regular conversions are implemented. One-shot and sequences of conversions
//! have been tested and work as expected.
//!
//! GPIO to channel mapping should be correct for all supported F4 devices. The mappings were taken from
//! CubeMX. The mappings are feature gated per 4xx device but there are actually sub variants for some
//! devices and some pins may be missing on some variants. The implementation has been split up and commented
//! to show which pins are available on certain device variants but currently the library doesn't enforce this.
//! To fully support the right pins would require 10+ more features for the various variants.
//! ## Todo
//! * Injected conversions
//! * Analog watchdog config
//! * Discontinuous mode
//! # Examples
//! ## One-shot conversion
//! ```
//! use stm32f4xx_hal::{
//!   gpio::gpioa,
//!   adc::{
//!     Adc,
//!     config::{AdcConfig, SampleTime},
//!   },
//! };
//!
//! let mut adc = Adc::adc1(device.ADC1, true, AdcConfig::default());
//! let pa3 = gpioa.pa3.into_analog();
//! let sample = adc.convert(&pa3, SampleTime::Cycles_480);
//! let millivolts = adc.sample_to_millivolts(sample);
//! info!("pa3: {}mV", millivolts);
//! ```
//!
//! ## Sequence conversion
//! ```
//! use stm32f4xx_hal::{
//!   gpio::gpioa,
//!   adc::{
//!     Adc,
//!     config::{AdcConfig, SampleTime, Sequence, Eoc, Scan, Clock},
//!   },
//! };
//!
//! let config = AdcConfig::default()
//!     //We'll either need DMA or an interrupt per conversion to convert
//!     //multiple values in a sequence
//!     .end_of_conversion_interrupt(Eoc::Conversion)
//!     //Scan mode is also required to convert a sequence
//!     .scan(Scan::Enabled)
//!     //And since we're looking for one interrupt per conversion the
//!     //clock will need to be fairly slow to avoid overruns breaking
//!     //the sequence. If you are running in debug mode and logging in
//!     //the interrupt, good luck... try setting pclk2 really low.
//!     //(Better yet use DMA)
//!     .clock(Clock::Pclk2_div_8);
//! let mut adc = Adc::adc1(device.ADC1, true, config);
//! let pa0 = gpioa.pa0.into_analog();
//! let pa3 = gpioa.pa3.into_analog();
//! adc.configure_channel(&pa0, Sequence::One, SampleTime::Cycles_112);
//! adc.configure_channel(&pa3, Sequence::Two, SampleTime::Cycles_480);
//! adc.configure_channel(&pa0, Sequence::Three, SampleTime::Cycles_112);
//! adc.start_conversion();
//! ```
//!
//! ## External trigger
//!
//! A common mistake on STM forums is enabling continuous mode but that causes it to start
//! capturing on the first trigger and capture as fast as possible forever, regardless of
//! future triggers. Continuous mode is disabled by default but I thought it was worth
//! highlighting.
//!
//! Getting the timer config right to make sure it's sending the event the ADC is listening
//! to can be a bit of a pain but the key fields are highlighted below. Try hooking a timer
//! channel up to an external pin with an LED or oscilloscope attached to check it's really
//! generating pulses if the ADC doesn't seem to be triggering.
//! ```
//! use stm32f4xx_hal::{
//!   gpio::gpioa,
//!   adc::{
//!     Adc,
//!     config::{AdcConfig, SampleTime, Sequence, Eoc, Scan, Clock},
//!   },
//! };
//!
//!  let config = AdcConfig::default()
//!      //Set the trigger you want
//!      .external_trigger(TriggerMode::RisingEdge, ExternalTrigger::Tim_1_cc_1);
//!  let mut adc = Adc::adc1(device.ADC1, true, config);
//!  let pa0 = gpioa.pa0.into_analog();
//!  adc.configure_channel(&pa0, Sequence::One, SampleTime::Cycles_112);
//!  //Make sure it's enabled but don't start the conversion
//!  adc.enable();
//!
//! //Configure the timer
//! let mut tim = Timer::tim1(device.TIM1, 1.hz(), clocks);
//! unsafe {
//!     let tim = &(*TIM1::ptr());
//!
//!     //Channel 1
//!     //Disable the channel before configuring it
//!     tim.ccer.modify(|_, w| w.cc1e().clear_bit());
//!
//!     tim.ccmr1_output().modify(|_, w| w
//!       //Preload enable for channel
//!       .oc1pe().set_bit()
//!
//!       //Set mode for channel, the default mode is "frozen" which won't work
//!       .oc1m().pwm_mode1()
//!     );
//!
//!     //Set the duty cycle, 0 won't work in pwm mode but might be ok in
//!     //toggle mode or match mode
//!     let max_duty = tim.arr.read().arr().bits() as u16;
//!     tim.ccr1.modify(|_, w| w.ccr().bits(max_duty / 2));
//!
//!     //Enable the channel
//!     tim.ccer.modify(|_, w| w.cc1e().set_bit());
//!
//!     //Enable the TIM main Output
//!     tim.bdtr.modify(|_, w| w.moe().set_bit());
//! }
//! ```

#![deny(missing_docs)]

/*
    Currently unused but this is the formula for using temperature calibration:
    Temperature in °C = (110-30) * (adc_sample - VtempCal30::get().read()) / (VtempCal110::get().read()-VtempCal30::get().read()) + 30
*/


use crate::rcc::{Enable, Reset};
use crate::{
    pac};
use core::fmt;

/// Vref internal signal, used for calibration
pub struct Vref;

/// Vbat internal signal, used for monitoring the battery (if used)
pub struct Vbat;

/// Core temperature internal signal
pub struct Temperature;

/// Contains types related to ADC configuration
pub mod config {
    /// The place in the sequence a given channel should be captured
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
    #[repr(u8)]
    pub enum RegularSequence {
        /// 1
        One = 0,
        /// 2
        Two = 1,
        /// 3
        Three = 2,
        /// 4
        Four = 3,
        /// 5
        Five = 4,
        /// 6
        Six = 5,
        /// 7
        Seven = 6,
        /// 8
        Eight = 7,
        /// 9
        Nine = 8,
        /// 10
        Ten = 9,
        /// 11
        Eleven = 10,
        /// 12
        Twelve = 11,
        /// 13
        Thirteen = 12,
        /// 14
        Fourteen = 13,
        /// 15
        Fifteen = 14,
        /// 16
        Sixteen = 15,
    }

    impl From<RegularSequence> for u8 {
        fn from(s: RegularSequence) -> u8 {
            s as _
        }
    }

    impl From<u8> for RegularSequence {
        fn from(bits: u8) -> Self {
            match bits {
                0 => RegularSequence::One,
                1 => RegularSequence::Two,
                2 => RegularSequence::Three,
                3 => RegularSequence::Four,
                4 => RegularSequence::Five,
                5 => RegularSequence::Six,
                6 => RegularSequence::Seven,
                7 => RegularSequence::Eight,
                8 => RegularSequence::Nine,
                9 => RegularSequence::Ten,
                10 => RegularSequence::Eleven,
                11 => RegularSequence::Twelve,
                12 => RegularSequence::Thirteen,
                13 => RegularSequence::Fourteen,
                14 => RegularSequence::Fifteen,
                15 => RegularSequence::Sixteen,
                _ => unimplemented!(),
            }
        }
    }

    /// The place in the sequence a given channel should be captured
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
    #[repr(u8)]
    pub enum InjectedSequence {
        /// 1
        One = 0,
        /// 2
        Two = 1,
        /// 3
        Three = 2,
        /// 4
        Four = 3,
    }

    impl From<InjectedSequence> for u8 {
        fn from(s: InjectedSequence) -> u8 {
            s as _
        }
    }

    impl From<u8> for InjectedSequence {
        fn from(bits: u8) -> Self {
            match bits {
                0 => InjectedSequence::One,
                1 => InjectedSequence::Two,
                2 => InjectedSequence::Three,
                3 => InjectedSequence::Four,
                _ => unimplemented!(),
            }
        }
    }


    /// The number of cycles to sample a given channel for
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    #[repr(u8)]
    pub enum SampleTime {
        /// 3 cycles
        Cycles_1p5 = 0,
        /// 15 cycles
        Cycles_7p5 = 1,
        /// 28 cycles
        Cycles_13p5 = 2,
        /// 56 cycles
        Cycles_28p5 = 3,
        /// 84 cycles
        Cycles_41p5 = 4,
        /// 112 cycles
        Cycles_55p5 = 5,
        /// 144 cycles
        Cycles_71p5 = 6,
        /// 480 cycles
        Cycles_239p5 = 7,
    }

    impl From<u8> for SampleTime {
        fn from(f: u8) -> SampleTime {
            match f {
                0 => SampleTime::Cycles_1p5,
                1 => SampleTime::Cycles_7p5,
                2 => SampleTime::Cycles_13p5,
                3 => SampleTime::Cycles_28p5,
                4 => SampleTime::Cycles_41p5,
                5 => SampleTime::Cycles_55p5,
                6 => SampleTime::Cycles_71p5,
                7 => SampleTime::Cycles_239p5,
                _ => unimplemented!(),
            }
        }
    }

    impl From<SampleTime> for u8 {
        fn from(l: SampleTime) -> u8 {
            l as _
        }
    }

    /// Clock config for the ADC
    /// Check the datasheet for the maximum speed the ADC supports
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    #[repr(u8)]
    pub enum Clock {
        /// HCLK referenced Adc Clock
        Hclk = 0, 
        /// Pll referenced Adc Clock
        PllClk = 1, 
    }


    /// Resolution to sample at
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    #[repr(u8)]
    pub enum Resolution {
        /// 12-bit
        Twelve = 3,
        /// 10-bit
        Ten = 2,
        /// 8-bit
        Eight = 1,
        /// 6-bit
        Six = 0,
    }
    impl From<Resolution> for u8 {
        fn from(r: Resolution) -> u8 {
            r as _
        }
    }

    /// Possible external triggers the ADC can listen to
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    #[repr(u8)]
    pub enum ExternalTrigger {
        /// TIM1 compare channel 1
        Tim_1_cc_1 = 0b0000,
        /// TIM1 compare channel 2
        Tim_1_cc_2 = 0b0001,
        /// TIM1 compare channel 3
        Tim_1_cc_3 = 0b0010,
        /// TIM2 compare channel 2
        Tim_2_cc_2 = 0b0011,
        /// TIM2 compare channel 3
        Tim_2_cc_3 = 0b0100,
        /// TIM2 compare channel 4
        Tim_2_cc_4 = 0b0101,
        /// TIM2 trigger out
        Tim_2_trgo = 0b0110,
        /// TIM3 compare channel 1
        Tim_3_cc_1 = 0b0111,
        /// TIM3 trigger out
        Tim_3_trgo = 0b1000,
        /// TIM4 compare channel 4
        Tim_4_cc_4 = 0b1001,
        /// TIM5 compare channel 1
        Tim_5_cc_1 = 0b1010,
        /// TIM5 compare channel 2
        Tim_5_cc_2 = 0b1011,
        /// TIM5 compare channel 3
        Tim_5_cc_3 = 0b1100,
        /// External interrupt line 11
        Exti_11 = 0b1111,
    }
    impl From<ExternalTrigger> for u8 {
        fn from(et: ExternalTrigger) -> u8 {
            et as _
        }
    }

    /// Possible trigger modes
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    #[repr(u8)]
    pub enum TriggerMode {
        /// Don't listen to external trigger
        Disabled = 0,
        /// Listen for rising edges of external trigger
        RisingEdge = 1,
    }
    impl From<TriggerMode> for bool {
        fn from(tm: TriggerMode) -> bool {
            tm == TriggerMode::RisingEdge
        }
    }

    /// Data register alignment
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum Align {
        /// Right align output data
        Right,
        /// Left align output data
        Left,
    }
    impl From<Align> for bool {
        fn from(a: Align) -> bool {
            match a {
                Align::Right => false,
                Align::Left => true,
            }
        }
    }

    /// Scan enable/disable
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum Scan {
        /// Scan mode disabled
        Disabled,
        /// Scan mode enabled
        Enabled,
    }
    impl From<Scan> for bool {
        fn from(s: Scan) -> bool {
            match s {
                Scan::Disabled => false,
                Scan::Enabled => true,
            }
        }
    }

    /// Continuous mode enable/disable
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum Continuous {
        /// Single mode, continuous disabled
        Single,
        /// Continuous mode enabled
        Continuous,
    }
    impl From<Continuous> for bool {
        fn from(c: Continuous) -> bool {
            match c {
                Continuous::Single => false,
                Continuous::Continuous => true,
            }
        }
    }

    /// DMA mode
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum Dma {
        /// No DMA, disabled
        Disabled,
        /// Single DMA, DMA will be disabled after each conversion sequence
        Single,
    }

    /// End-of-conversion interrupt enabled/disabled
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum Eoc {
        /// End-of-conversion interrupt disabled
        Disabled,
        /// End-of-conversion interrupt enabled per conversion
        Conversion,
        /// End-of-conversion interrupt enabled per sequence
        Sequence,
    }

    /// Configuration for the adc.
    /// There are some additional parameters on the adc peripheral that can be
    /// added here when needed but this covers several basic usecases.
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub struct AdcConfig {
        pub(crate) clock: Clock,
        pub(crate) resolution: Resolution,
        pub(crate) align: Align,
        pub(crate) scan: Scan,
        pub(crate) external_trigger: (TriggerMode, ExternalTrigger),
        pub(crate) continuous: Continuous,
        pub(crate) dma: Dma,
        pub(crate) end_of_conversion_interrupt: Eoc,
        pub(crate) default_sample_time: SampleTime,
        pub(crate) vdda: Option<u32>,
    }

    impl AdcConfig {
        /// change the clock field
        pub fn clock(mut self, clock: Clock) -> Self {
            self.clock = clock;
            self
        }
        /// change the resolution field
        pub fn resolution(mut self, resolution: Resolution) -> Self {
            self.resolution = resolution;
            self
        }
        /// change the align field
        pub fn align(mut self, align: Align) -> Self {
            self.align = align;
            self
        }
        /// change the scan field
        pub fn scan(mut self, scan: Scan) -> Self {
            self.scan = scan;
            self
        }
        /// change the external_trigger field
        pub fn external_trigger(
            mut self,
            trigger_mode: TriggerMode,
            trigger: ExternalTrigger,
        ) -> Self {
            self.external_trigger = (trigger_mode, trigger);
            self
        }
        /// change the continuous field
        pub fn continuous(mut self, continuous: Continuous) -> Self {
            self.continuous = continuous;
            self
        }
        /// change the dma field
        pub fn dma(mut self, dma: Dma) -> Self {
            self.dma = dma;
            self
        }
        /// change the end_of_conversion_interrupt field
        pub fn end_of_conversion_interrupt(mut self, end_of_conversion_interrupt: Eoc) -> Self {
            self.end_of_conversion_interrupt = end_of_conversion_interrupt;
            self
        }
        /// change the default_sample_time field
        pub fn default_sample_time(mut self, default_sample_time: SampleTime) -> Self {
            self.default_sample_time = default_sample_time;
            self
        }

        /// Specify the reference voltage for the ADC.
        ///
        /// # Args
        /// * `vdda_mv` - The ADC reference voltage in millivolts.
        pub fn reference_voltage(mut self, vdda_mv: u32) -> Self {
            self.vdda = Some(vdda_mv);
            self
        }
    }

    impl Default for AdcConfig {
        fn default() -> Self {
            Self {
                clock: Clock::Hclk,
                resolution: Resolution::Twelve,
                align: Align::Right,
                scan: Scan::Disabled,
                external_trigger: (TriggerMode::Disabled, ExternalTrigger::Tim_1_cc_1),
                continuous: Continuous::Single,
                dma: Dma::Disabled,
                end_of_conversion_interrupt: Eoc::Disabled,
                default_sample_time: SampleTime::Cycles_239p5,
                vdda: None,
            }
        }
    }
}

/// Analog to Digital Converter
#[derive(Clone, Copy)]
pub struct Adc<ADC> {
    /// Current config of the ADC, kept up to date by the various set methods
    config: config::AdcConfig,
    /// The adc peripheral
    adc_reg: ADC,
    /// Exclusive limit for the sample value possible for the configured resolution.
    max_sample: u32,
}
impl<ADC> fmt::Debug for Adc<ADC> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Adc: {{ max_sample: {:?}, config: {:?}, ... }}",
            self.max_sample, self.config
        )
    }
}

macro_rules! adc {
    ($($adc_type:ident => ($constructor_fn_name:ident)),+ $(,)*) => {
        $(

            impl Adc<pac::$adc_type> {

                /// Enables the ADC clock, resets the peripheral (optionally), runs calibration and applies the supplied config
                /// # Arguments
                /// * `reset` - should a reset be performed. This is provided because on some devices multiple ADCs share the same common reset
                pub fn $constructor_fn_name(adc: pac::$adc_type, reset: bool, config: config::AdcConfig) -> Adc<pac::$adc_type> {
                    unsafe {
                        // All ADCs share the same reset interface.
                        // NOTE(unsafe) this reference will only be used for atomic writes with no side effects.
                        let rcc = &(*pac::Rcc::ptr());

                        //Enable the clock
                        pac::$adc_type::enable(rcc);

                        if reset {
                            //Reset the peripheral(s)
                            pac::$adc_type::reset(rcc);
                        }
                    }

                    let mut s = Self {
                        config,
                        adc_reg: adc,
                        max_sample: 0,
                    };

                    //Probably unnecessary to disable the ADC in most cases but it shouldn't do any harm either
                    s.disable();
                    s.apply_config(config);

                    s.enable();
                    s
                }

                /// Applies all fields in AdcConfig
                pub fn apply_config(&mut self, config: config::AdcConfig) {
                    self.set_resolution(config.resolution);
                    self.set_align(config.align);
                    self.set_scan(config.scan);
                    self.set_regular_channel_external_trigger(config.external_trigger);

                    self.set_continuous(config.continuous);
                    self.set_dma(config.dma);
                    self.set_end_of_regular_conversion_interrupt(config.end_of_conversion_interrupt);
                    self.set_default_sample_time(config.default_sample_time);
                }

                /// Returns if the adc is enabled
                pub fn is_enabled(&self) -> bool {
                    self.adc_reg.ctrl2().read().on().bit_is_set()
                }

                /// Enables the adc
                pub fn enable(&mut self) {
                    self.adc_reg.ctrl2().modify(|_, w| w.on().set_bit());
                }
                
                /// Calibrates the adc
                pub fn calibrate(&mut self) {
                    self.adc_reg.ctrl2().modify(|_,w| w.encal().set_bit());
                    while self.adc_reg.ctrl2().read().encal().bit_is_set() {}
                }

                /// Enable Vref/Temp channels in the adc
                pub fn enable_vref_temp(&mut self) {
                    self.adc_reg.ctrl2().modify(|_,w| w.tempen().set_bit());
                }

                /// Enable Vref/Temp channels in the adc
                pub fn set_synchronous_injection_mode(&mut self) {
                    unsafe { self.adc_reg.ctrl1().modify(|_,w| w.dusel().bits(0b0101)) };
                }

                /// Disables the adc
                /// # Note
                /// The ADC in the f4 has few restrictions on what can be configured while the ADC
                /// is enabled. If any bugs are found where some settings aren't "sticking" try disabling
                /// the ADC before changing them. The reference manual for the chip I'm using only states
                /// that the sequence registers are locked when they are being converted.
                pub fn disable(&mut self) {
                    self.adc_reg.ctrl2().modify(|_, w| w.on().clear_bit());
                }

                /// Starts conversion sequence. Waits for the hardware to indicate it's actually started.
                pub fn start_conversion(&mut self) {
                    self.enable();
                    self.clear_end_of_regular_conversion_flag();
                    //Start conversion
                    self.adc_reg.ctrl2().modify(|_, w| w.swstrrch().set_bit());

                    while !self.adc_reg.sts().read().str().bit_is_set() {}
                }

                /// Sets the sampling resolution
                pub fn set_resolution(&mut self, resolution: config::Resolution) {
                    self.max_sample = match resolution {
                        config::Resolution::Twelve => (1 << 12),
                        config::Resolution::Ten => (1 << 10),
                        config::Resolution::Eight => (1 << 8),
                        config::Resolution::Six => (1 << 6),
                    };
                    self.config.resolution = resolution;
                    self.adc_reg.ctrl3().modify(|_, w| unsafe { w.res().bits(resolution as _) });
                }

                /// Sets the DR register alignment to left or right
                pub fn set_align(&mut self, align: config::Align) {
                    self.config.align = align;
                    self.adc_reg.ctrl2().modify(|_, w| w.alig().bit(align.into()));
                }

                /// Enables and disables scan mode
                pub fn set_scan(&mut self, scan: config::Scan) {
                    self.config.scan = scan;
                    self.adc_reg.ctrl1().modify(|_, w| w.scanmd().bit(scan.into()));
                }

                /// Sets which external trigger to use and if it is disabled, rising, falling or both
                pub fn set_regular_channel_external_trigger(&mut self, (edge, extsel): (config::TriggerMode, config::ExternalTrigger)) {
                    self.config.external_trigger = (edge, extsel);
                    self.adc_reg.ctrl2().modify(|_, w| unsafe { w
                        .extrsel().bits(extsel as _)
                        .extrtrig().bit(edge.into()) }
                    );
                }
                /// Sets which external trigger to use and if it is disabled, rising, falling or both
                pub fn set_injected_channel_external_trigger(&mut self, (edge, extsel): (config::TriggerMode, config::ExternalTrigger)) {
                    self.config.external_trigger = (edge, extsel);
                    self.adc_reg.ctrl2().modify(|_, w| unsafe { w
                        .extjsel().bits(extsel as _)
                        .extjtrig().bit(edge.into()) }
                    );
                }

                /// Enables and disables continuous mode
                pub fn set_continuous(&mut self, continuous: config::Continuous) {
                    self.config.continuous = continuous;
                    self.adc_reg.ctrl2().modify(|_, w| w.ctu().bit(continuous.into()));
                }

                /// Sets DMA to disabled, single or continuous
                pub fn set_dma(&mut self, dma: config::Dma) {
                    self.config.dma = dma;
                    let endma = match dma {
                        config::Dma::Disabled => false,
                        config::Dma::Single => true,
                    };
                    self.adc_reg.ctrl2().modify(|_, w| w
                        .endma().bit(endma)
                    );
                }

                /// Sets if the end-of-conversion behaviour.
                /// The end-of-conversion interrupt occur either per conversion or for the whole sequence.
                pub fn set_end_of_regular_conversion_interrupt(&mut self, eoc: config::Eoc) {
                    self.config.end_of_conversion_interrupt = eoc;
                    let (en_ch, en_seq) = match eoc {
                        config::Eoc::Disabled => (false, false),
                        config::Eoc::Conversion => (true, true),
                        config::Eoc::Sequence => (true, false),
                    };
                    self.adc_reg.ctrl1().modify(|_, w| w.endien().bit(en_seq));
                    self.adc_reg.ctrl3().modify(|_, w| w.endcaien().bit(en_ch));
                }

                /// Sets if the end-of-conversion behaviour.
                /// The end-of-conversion interrupt occur either per conversion or for the whole sequence.
                pub fn set_end_of_injected_conversion_interrupt(&mut self, eoc: config::Eoc) {
                    self.config.end_of_conversion_interrupt = eoc;
                    let (en_ch, en_seq) = match eoc {
                        config::Eoc::Disabled => (false, false),
                        config::Eoc::Conversion => (true, false),
                        config::Eoc::Sequence => (false, true),
                    };
                    self.adc_reg.ctrl1().modify(|_, w| w.jendcien().bit(en_seq));
                    self.adc_reg.ctrl3().modify(|_, w| w.jendcaien().bit(en_ch));
                }

                /// Resets the end-of-conversion flag
                pub fn clear_end_of_regular_conversion_flag(&mut self) {
                    self.adc_reg.sts().modify(|_, w| w.endca().clear_bit().endc().clear_bit());
                }

                /// Resets the end-of-conversion flag
                pub fn clear_end_of_injected_conversion_flag(&mut self) {
                    self.adc_reg.sts().modify(|_, w| w.jendca().clear_bit().jendc().clear_bit());
                }

                /// Sets the default sample time that is used for one-shot conversions.
                /// [configure_channel](#method.configure_channel) and [start_conversion](#method.start_conversion) can be \
                /// used for configurations where different sampling times are required per channel.
                pub fn set_default_sample_time(&mut self, sample_time: config::SampleTime) {
                    self.config.default_sample_time = sample_time;
                }

                /// Returns the current sequence length. Primarily useful for configuring DMA.
                pub fn sequence_length(&mut self) -> u8 {
                    self.adc_reg.rseq1().read().len().bits() + 1
                }

                /// Reset the regular sequence
                pub fn reset_regular_sequence(&mut self) {
                    //The reset state is One conversion selected
                    self.adc_reg.rseq1().modify(|_, w| unsafe { w.len().bits(config::RegularSequence::One.into())});
                }
                
                /// Reset the injected sequence
                pub fn reset_injected_sequence(&mut self) {
                    //The reset state is One conversion selected
                    self.adc_reg.jseq().modify(|_, w| unsafe { w.jlen().bits(config::InjectedSequence::One.into())});
                }


                /// Returns the address of the ADC data register. Primarily useful for configuring DMA.
                pub fn data_register_address(&mut self) -> u32 {
                    self.adc_reg.dat().as_ptr() as u32
                }

                /// Configure a channel for sampling.
                /// It will make sure the sequence is at least as long as the `sequence` provided.
                /// # Arguments
                /// * `channel` - channel to configure
                /// * `sequence` - where in the sequence to sample the channel. Also called rank in some STM docs/code
                /// * `sample_time` - how long to sample for. See datasheet and ref manual to work out how long you need\
                /// to sample for at a given ADC clock frequency
                pub fn configure_regular_channel<CHANNEL>(&mut self, _channel: &CHANNEL, sequence: config::RegularSequence, sample_time: config::SampleTime)
                where
                    CHANNEL: embedded_hal_02::adc::Channel<pac::$adc_type, ID=u8>
                {
                    //Check the sequence is long enough
                    self.adc_reg.rseq1().modify(|r, w| {
                        let prev: config::RegularSequence = r.len().bits().into();
                        if prev < sequence {
                            unsafe { w.len().bits(sequence.into()) }
                        } else {
                            w
                        }
                    });

                    let channel = CHANNEL::channel();

                    //Set the channel in the right sequence field
                    match sequence {
                        config::RegularSequence::One      => self.adc_reg.rseq3().modify(|_, w| unsafe {w.seq1().bits(channel) }),
                        config::RegularSequence::Two      => self.adc_reg.rseq3().modify(|_, w| unsafe {w.seq2().bits(channel) }),
                        config::RegularSequence::Three    => self.adc_reg.rseq3().modify(|_, w| unsafe {w.seq3().bits(channel) }),
                        config::RegularSequence::Four     => self.adc_reg.rseq3().modify(|_, w| unsafe {w.seq4().bits(channel) }),
                        config::RegularSequence::Five     => self.adc_reg.rseq3().modify(|_, w| unsafe {w.seq5().bits(channel) }),
                        config::RegularSequence::Six      => self.adc_reg.rseq3().modify(|_, w| unsafe {w.seq6().bits(channel) }),
                        config::RegularSequence::Seven    => self.adc_reg.rseq2().modify(|_, w| unsafe {w.seq7().bits(channel) }),
                        config::RegularSequence::Eight    => self.adc_reg.rseq2().modify(|_, w| unsafe {w.seq8().bits(channel) }),
                        config::RegularSequence::Nine     => self.adc_reg.rseq2().modify(|_, w| unsafe {w.seq9().bits(channel) }),
                        config::RegularSequence::Ten      => self.adc_reg.rseq2().modify(|_, w| unsafe {w.seq10().bits(channel) }),
                        config::RegularSequence::Eleven   => self.adc_reg.rseq2().modify(|_, w| unsafe {w.seq11().bits(channel) }),
                        config::RegularSequence::Twelve   => self.adc_reg.rseq2().modify(|_, w| unsafe {w.seq12().bits(channel) }),
                        config::RegularSequence::Thirteen => self.adc_reg.rseq1().modify(|_, w| unsafe {w.seq13().bits(channel) }),
                        config::RegularSequence::Fourteen => self.adc_reg.rseq1().modify(|_, w| unsafe {w.seq14().bits(channel) }),
                        config::RegularSequence::Fifteen  => self.adc_reg.rseq1().modify(|_, w| unsafe {w.seq15().bits(channel) }),
                        config::RegularSequence::Sixteen  => self.adc_reg.rseq1().modify(|_, w| unsafe {w.seq16().bits(channel) }),
                    }

                    //Set the sample time for the channel
                    let st = sample_time as u8;
                    match channel {
                        0 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp0().bits(st)}),
                        1 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp1().bits(st)}),
                        2 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp2().bits(st)}),
                        3 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp3().bits(st)}),
                        4 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp4().bits(st)}),
                        5 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp5().bits(st)}),
                        6 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp6().bits(st)}),
                        7 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp7().bits(st)}),
                        8 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp8().bits(st)}),
                        9 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp9().bits(st)}),
                        10 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp10().bits(st)}),
                        11 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp11().bits(st)}),
                        12 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp12().bits(st)}),
                        13 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp13().bits(st)}),
                        14 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp14().bits(st)}),
                        15 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp15().bits(st)}),
                        16 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp16().bits(st)}),
                        17 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp17().bits(st)}),
                        18 => self.adc_reg.sampt3().modify(|_, w| unsafe {w.samp().bits(st)}),
                        _ => unimplemented!(),
                    }
                }

                /// Configure a channel for sampling.
                /// It will make sure the sequence is at least as long as the `sequence` provided.
                /// # Arguments
                /// * `channel` - channel to configure
                /// * `sequence` - where in the sequence to sample the channel. Also called rank in some STM docs/code
                /// * `sample_time` - how long to sample for. See datasheet and ref manual to work out how long you need\
                /// to sample for at a given ADC clock frequency
                pub fn configure_injected_channel<CHANNEL>(&mut self, _channel: &CHANNEL, sequence: config::InjectedSequence, sample_time: config::SampleTime)
                where
                    CHANNEL: embedded_hal_02::adc::Channel<pac::$adc_type, ID=u8>
                {
                    let mut jlen = self.adc_reg.jseq().read().jlen().bits();
                    //Check the sequence is long enough
                    self.adc_reg.jseq().modify(|r, w| {
                        let init_reg = r.bits();
                        let prev: config::InjectedSequence = r.jlen().bits().into();
                        if prev < sequence {
                            let shift_cnt = (sequence as u8 - prev as u8) * 5u8;
                            unsafe { w.bits(init_reg >> shift_cnt); }
                            jlen = sequence as u8;
                            unsafe { w.jlen().bits(sequence as u8) }
                        } else {
                            jlen = sequence as u8;
                            w
                        }
                    });

                    let channel = CHANNEL::channel();
                    let target_jseq : config::InjectedSequence = (3 - jlen + sequence as u8).into();
                    //Set the channel in the right sequence field
                    match target_jseq {
                        config::InjectedSequence::One      => self.adc_reg.jseq().modify(|_, w| unsafe {w.jseq1().bits(channel) }),
                        config::InjectedSequence::Two      => self.adc_reg.jseq().modify(|_, w| unsafe {w.jseq2().bits(channel) }),
                        config::InjectedSequence::Three    => self.adc_reg.jseq().modify(|_, w| unsafe {w.jseq3().bits(channel) }),
                        config::InjectedSequence::Four     => self.adc_reg.jseq().modify(|_, w| unsafe {w.jseq4().bits(channel) }),
                    }

                    //Set the sample time for the channel
                    let st = sample_time as u8;
                    match channel {
                        0 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp0().bits(st)}),
                        1 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp1().bits(st)}),
                        2 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp2().bits(st)}),
                        3 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp3().bits(st)}),
                        4 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp4().bits(st)}),
                        5 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp5().bits(st)}),
                        6 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp6().bits(st)}),
                        7 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp7().bits(st)}),
                        8 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp8().bits(st)}),
                        9 => self.adc_reg.smpr2().modify(|_, w| unsafe {w.samp9().bits(st)}),
                        10 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp10().bits(st)}),
                        11 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp11().bits(st)}),
                        12 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp12().bits(st)}),
                        13 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp13().bits(st)}),
                        14 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp14().bits(st)}),
                        15 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp15().bits(st)}),
                        16 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp16().bits(st)}),
                        17 => self.adc_reg.smpr1().modify(|_, w| unsafe {w.samp17().bits(st)}),
                        18 => self.adc_reg.sampt3().modify(|_, w| unsafe {w.samp().bits(st)}),
                        _ => unimplemented!(),
                    }
                }


                /// Returns the current sample stored in the ADC data register
                pub fn current_sample(&self) -> u16 {
                    self.adc_reg.dat().read().jdat().bits()
                }


                /// Returns the current injected sample stored in the ADC data register
                pub fn injected_sample(&self, seq : config::InjectedSequence) -> i16 {
                    match seq {
                        config::InjectedSequence::One      => self.adc_reg.jdat1().read().jdat1().bits() as i16,
                        config::InjectedSequence::Two      => self.adc_reg.jdat2().read().jdat2().bits() as i16,
                        config::InjectedSequence::Three    => self.adc_reg.jdat3().read().jdat3().bits() as i16,
                        config::InjectedSequence::Four     => self.adc_reg.jdat4().read().jdat4().bits() as i16,
                    }
                }

                /// Returns the current injected sample stored in the ADC data register
                pub fn get_injected_offset(&self, seq : config::InjectedSequence) -> u16 {
                    match seq {
                        config::InjectedSequence::One      => self.adc_reg.joffset1().read().offsetjch1().bits(),
                        config::InjectedSequence::Two      => self.adc_reg.joffset2().read().offsetjch2().bits(),
                        config::InjectedSequence::Three    => self.adc_reg.joffset3().read().offsetjch3().bits(),
                        config::InjectedSequence::Four     => self.adc_reg.joffset4().read().offsetjch4().bits(),
                    }
                }

                /// Returns the current injected sample stored in the ADC data register
                pub fn set_injected_offset(&self, seq : config::InjectedSequence, offset : u16) {
                    match seq {
                        config::InjectedSequence::One      => self.adc_reg.joffset1().modify(|_,w| unsafe { w.offsetjch1().bits(offset) }),
                        config::InjectedSequence::Two      => self.adc_reg.joffset2().modify(|_,w| unsafe { w.offsetjch2().bits(offset) }),
                        config::InjectedSequence::Three    => self.adc_reg.joffset3().modify(|_,w| unsafe { w.offsetjch3().bits(offset) }),
                        config::InjectedSequence::Four     => self.adc_reg.joffset4().modify(|_,w| unsafe { w.offsetjch4().bits(offset) }),
                    }
                }

                /// Returns the current injected sample stored in the ADC data register
                pub fn shift_injected_offset(&self, seq : config::InjectedSequence, offset : i16) {
                    match seq {
                        config::InjectedSequence::One => self.adc_reg.joffset1().modify(|r,w| unsafe {  
                            w.offsetjch1().bits(i16::wrapping_add(r.offsetjch1().bits() as i16, offset) as u16) 
                        }),
                        config::InjectedSequence::Two => self.adc_reg.joffset2().modify(|r,w| unsafe {  
                            w.offsetjch2().bits(i16::wrapping_add(r.offsetjch2().bits() as i16 , offset) as u16) 
                        }),
                        config::InjectedSequence::Three => self.adc_reg.joffset3().modify(|r,w| unsafe {  
                            w.offsetjch3().bits(i16::wrapping_add(r.offsetjch3().bits() as i16 , offset) as u16) 
                        }),
                        config::InjectedSequence::Four => self.adc_reg.joffset4().modify(|r,w| unsafe {  
                            w.offsetjch4().bits(i16::wrapping_add(r.offsetjch4().bits() as i16 , offset) as u16) 
                        }),
                    }
                }

                /// Block until the conversion is completed
                /// # Panics
                /// Will panic if there is no conversion started and the end-of-conversion bit is not set
                pub fn wait_for_regular_conversion_sequence(&self) {
                    if !self.adc_reg.sts().read().str().bit_is_set() && !self.adc_reg.sts().read().endc().bit_is_set() {
                        panic!("Waiting for end-of-conversion but no conversion started");
                    }
                    while !self.adc_reg.sts().read().endc().bit_is_set() {}
                    //Clear the conversion started flag
                    self.adc_reg.sts().modify(|_, w| w.str().clear_bit());
                }

                /// Block until the conversion is completed
                /// # Panics
                /// Will panic if there is no conversion started and the end-of-conversion bit is not set
                pub fn wait_for_injected_conversion_sequence(&self) {
                    if !self.adc_reg.sts().read().jstr().bit_is_set() && !self.adc_reg.sts().read().jendc().bit_is_set() {
                        panic!("Waiting for end-of-conversion but no conversion started");
                    }
                    while !self.adc_reg.sts().read().jendc().bit_is_set() {}
                    //Clear the conversion started flag
                    self.adc_reg.sts().modify(|_, w| w.jstr().clear_bit());
                }


                /// Synchronously convert a single sample
                /// Note that it reconfigures the adc sequence and doesn't restore it
                pub fn convert<PIN>(&mut self, pin: &PIN, sample_time: config::SampleTime) -> u16
                where
                    PIN: embedded_hal_02::adc::Channel<pac::$adc_type, ID=u8>
                {
                    self.adc_reg.ctrl2().modify(|_, w| w
                        .endma().clear_bit() //Disable dma
                        .ctu().clear_bit() //Disable continuous mode
                        .extrtrig().bit(config::TriggerMode::Disabled.into()) //Disable trigger
                    );
                    self.adc_reg.ctrl1().modify(|_, w| w
                        .scanmd().clear_bit() //Disable scan mode
                        .endien().clear_bit() //Disable end of conversion interrupt
                    );
                    self.adc_reg.ctrl3().modify(|_, w| w
                        .endcaien().clear_bit() //Disable scan mode
                    );
                    self.reset_regular_sequence();
                    self.configure_regular_channel(pin, config::RegularSequence::One, sample_time);
                    self.enable();
                    self.clear_end_of_regular_conversion_flag();
                    self.start_conversion();

                    //Wait for the sequence to complete
                    self.wait_for_regular_conversion_sequence();

                    let result = self.current_sample();

                    //Reset the config
                    self.apply_config(self.config);

                    result
                }
            }

            impl Adc<pac::$adc_type> {
                fn read<PIN>(&mut self, pin: &mut PIN) -> nb::Result<u16, ()>
                    where PIN: embedded_hal_02::adc::Channel<pac::$adc_type, ID=u8>,
                {
                    let enabled = self.is_enabled();
                    if !enabled {
                        self.enable();
                    }

                    let sample = self.convert(pin, self.config.default_sample_time);

                    if !enabled {
                        self.disable();
                    }

                    Ok(sample)
                }
            }

            impl<PIN> embedded_hal_02::adc::OneShot<pac::$adc_type, u16, PIN> for Adc<pac::$adc_type>
            where
                PIN: embedded_hal_02::adc::Channel<pac::$adc_type, ID=u8>,
            {
                type Error = ();

                fn read(&mut self, pin: &mut PIN) -> nb::Result<u16, Self::Error> {
                    self.read::<PIN>(pin)
                }
            }
        )+
    };
}



adc!(Adc1 => (adc1));

adc!(Adc2 => (adc2));

adc!(Adc3 => (adc3));

adc!(Adc4 => (adc4));


macro_rules! adc_map {
    ($adc_type:ident => { $(($channel_type:ty , $channel_id:tt)),+ $(,)* }) => {
        $(
            impl embedded_hal_02::adc::Channel<crate::pac::$adc_type> for $channel_type {
                type ID = u8;

                fn channel() -> Self::ID {
                    $channel_id
                }
            }
        )*
    };
}
mod mappings {
    use crate::gpio::*;
    use super::*;
    adc_map! {
        Adc1 => {
            (PA0<crate::gpio::Analog>, 1),
            (PA1<crate::gpio::Analog>, 2),
            (PA6<crate::gpio::Analog>, 3),
            (PA3<crate::gpio::Analog>, 4),
            (PF4<crate::gpio::Analog>, 5),
            (PC0<crate::gpio::Analog>, 6),
            (PC1<crate::gpio::Analog>, 7),
            (PC2<crate::gpio::Analog>, 8),
            (PC3<crate::gpio::Analog>, 9),
            (PF2<crate::gpio::Analog>, 10),
            (PA2<crate::gpio::Analog>, 11),
            (Temperature, 16),
            (Vbat, 17),
            (Vref, 18),
        }
    }
    adc_map! {
        Adc2 => {
            (PA4<crate::gpio::Analog>, 1),
            (PA5<crate::gpio::Analog>, 2),
            (PB1<crate::gpio::Analog>, 3),
            (PA7<crate::gpio::Analog>, 4),
            (PC4<crate::gpio::Analog>, 5),
            (PC0<crate::gpio::Analog>, 6),
            (PC1<crate::gpio::Analog>, 7),
            (PC2<crate::gpio::Analog>, 8),
            (PC3<crate::gpio::Analog>, 9),
            (PF2<crate::gpio::Analog>, 10),
            (PA2<crate::gpio::Analog>, 11),
            (PC5<crate::gpio::Analog>, 12),
            (PB2<crate::gpio::Analog>, 13),

            (Vref, 18),
        }
    }
    adc_map! {
        Adc3 => {
            (PB11<crate::gpio::Analog>, 1),
            (PE9<crate::gpio::Analog>, 2),
            (PE13<crate::gpio::Analog>, 3),
            (PE12<crate::gpio::Analog>, 4),
            (PB13<crate::gpio::Analog>, 5),
            (PE8<crate::gpio::Analog>, 6),
            (PD10<crate::gpio::Analog>, 7),
            (PD11<crate::gpio::Analog>, 8),
            (PD12<crate::gpio::Analog>, 9),
            (PD13<crate::gpio::Analog>, 10),
            (PD14<crate::gpio::Analog>, 11),
            (PB0<crate::gpio::Analog>, 12),
            (PE7<crate::gpio::Analog>, 13),
            (PE10<crate::gpio::Analog>, 14),
            (PE11<crate::gpio::Analog>, 15),

            (Vref, 18),
        }
    }
    adc_map! {
        Adc4 => {
            (PE14<crate::gpio::Analog>, 1),
            (PE15<crate::gpio::Analog>, 2),
            (PB12<crate::gpio::Analog>, 3),
            (PB14<crate::gpio::Analog>, 4),
            (PB15<crate::gpio::Analog>, 5),
            (PE8<crate::gpio::Analog>, 6),
            (PD10<crate::gpio::Analog>, 7),
            (PD11<crate::gpio::Analog>, 8),
            (PD12<crate::gpio::Analog>, 9),
            (PD13<crate::gpio::Analog>, 10),
            (PD14<crate::gpio::Analog>, 11),
            (PD8<crate::gpio::Analog>, 12),
            (PD9<crate::gpio::Analog>, 13),
            
            (Vref, 18),

        }
    }

}


