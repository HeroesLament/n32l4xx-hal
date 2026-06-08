//! Timers
//!
//! Pins can be used for PWM output in both push-pull mode (`Alternate`) and open-drain mode
//! (`AlternateOD`).

use crate::delay::CountDown;
use cast::{u16, u32};
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::peripheral::{DCB, DWT, SYST};
use embedded_hal_02::timer::{Cancel, CountDown as _, Periodic};
use void::Void;

use crate::pac::Rcc;

use crate::rcc::{self, Clocks};
use crate::time::{Hertz, MicroSecond};

/// Timer wrapper
pub struct Timer<TIM> {
    pub(crate) tim: TIM,
    pub(crate) clk: Hertz,
}

/// Hardware timers
pub struct CountDownTimer<TIM> {
    tim: TIM,
    clk: Hertz,
}

impl<TIM> Timer<TIM>
where
    CountDownTimer<TIM>: CountDown<Time = MicroSecond>,
{
    /// Starts timer in count down mode at a given frequency
    pub fn start_count_down<T>(self, timeout: T) -> CountDownTimer<TIM>
    where
        T: Into<MicroSecond>,
    {
        let Self { tim, clk } = self;
        let mut timer = CountDownTimer { tim, clk };
        timer.start(timeout);
        timer
    }
}

impl<TIM> Periodic for CountDownTimer<TIM> {}

/// Interrupt events
pub enum Event {
    /// CountDownTimer timed out / count down ended
    TimeOut,
}

/// Trigger output source
pub enum TriggerSource {
    /// Timer reset - UG as trigger output
    Reset,
    /// Timer enable - CNT_EN as trigger output
    Enable = 0b001,
    /// Update event - Update event as trigger output
    Update = 0b010,
    /// Compare Pulse - Positive pulse if CC1IF is setted
    ComparePulse = 0b011,
    /// Compare1 - OC1REFC as trigger output
    Compare1 = 0b100,
    /// Compare2 - OC2REFC as trigger output
    Compare2 = 0b101,
    /// Compare3 - OC3REFC as trigger output
    Compare3 = 0b110,
    /// Compare4 - OC4REFC as trigger output
    Compare4 = 0b111,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Error {
    /// CountDownTimer is disabled
    Disabled,
}

impl Timer<SYST> {
    /// Initialize timer
    pub fn syst(mut syst: SYST, clocks: &Clocks) -> Self {
        syst.set_clock_source(SystClkSource::Core);
        Self {
            tim: syst,
            clk: clocks.hclk,
        }
    }

    pub fn release(self) -> SYST {
        self.tim
    }
}

impl CountDownTimer<SYST> {
    /// Starts listening for an `event`
    pub fn listen(&mut self, event: Event) {
        match event {
            Event::TimeOut => self.tim.enable_interrupt(),
        }
    }

    /// Stops listening for an `event`
    pub fn unlisten(&mut self, event: Event) {
        match event {
            Event::TimeOut => self.tim.disable_interrupt(),
        }
    }
}

impl embedded_hal_02::timer::CountDown for CountDownTimer<SYST> {
    type Time = MicroSecond;

    fn start<T>(&mut self, timeout: T)
    where
        T: Into<MicroSecond>,
    {
        let rvr = crate::time::cycles(timeout.into(), self.clk) - 1;

        assert!(rvr < (1 << 24));

        self.tim.set_reload(rvr);
        self.tim.clear_current();
        self.tim.enable_counter();
    }

    fn wait(&mut self) -> nb::Result<(), Void> {
        if self.tim.has_wrapped() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl CountDown for CountDownTimer<SYST> {
    fn max_period(&self) -> MicroSecond {
        crate::time::duration(self.clk, (1 << 24) - 1)
    }
}

impl Cancel for CountDownTimer<SYST> {
    type Error = Error;

    fn cancel(&mut self) -> Result<(), Self::Error> {
        if !self.tim.is_counter_enabled() {
            return Err(Self::Error::Disabled);
        }

        self.tim.disable_counter();
        Ok(())
    }
}

/// A monotonic non-decreasing timer
///
/// This uses the timer in the debug watch trace peripheral. This means, that if the
/// core is stopped, the timer does not count up. This may be relevant if you are using
/// cortex_m_semihosting::hprintln for debugging in which case the timer will be stopped
/// while printing
#[derive(Clone, Copy)]
pub struct MonoTimer {
    frequency: Hertz,
}

impl MonoTimer {
    /// Creates a new `Monotonic` timer
    pub fn new(mut dwt: DWT, mut dcb: DCB, clocks: &Clocks) -> Self {
        dcb.enable_trace();
        dwt.enable_cycle_counter();

        // now the CYCCNT counter can't be stopped or reset
        #[allow(clippy::drop_non_drop)]
        drop(dwt);

        MonoTimer {
            frequency: clocks.hclk,
        }
    }

    /// Returns the frequency at which the monotonic timer is operating at
    pub fn frequency(self) -> Hertz {
        self.frequency
    }

    /// Returns an `Instant` corresponding to "now"
    pub fn now(self) -> Instant {
        Instant {
            now: DWT::cycle_count(),
        }
    }
}

/// A measurement of a monotonically non-decreasing clock
#[derive(Clone, Copy)]
pub struct Instant {
    now: u32,
}

impl Instant {
    /// Ticks elapsed since the `Instant` was created
    pub fn elapsed(self) -> u32 {
        DWT::cycle_count().wrapping_sub(self.now)
    }
}
pub trait Instance: crate::Sealed + rcc::Enable + rcc::Reset + rcc::BusTimerClock {}

impl<TIM> Timer<TIM>
where
    TIM: Instance ,
{
    /// Initialize timer
    pub fn new(tim: TIM, clocks: &Clocks) -> Self {
        unsafe {
            //NOTE(unsafe) this reference will only be used for atomic writes with no side effects
            let rcc = &(*Rcc ::ptr());
            // Enable and reset the timer peripheral
            TIM::enable(rcc);
            TIM::reset(rcc);
        }
        Self {
            clk: TIM::timer_clock(clocks),
            tim,
        }
    }
}


macro_rules! hal_ext_trgo {
    ($($TIM:ty: ($tim:ident, $mms:ident),)+) => {
        $(
            impl Timer<$TIM> {
                pub fn set_trigger_source(&mut self, trigger_source: TriggerSource) {
                    self.tim.ctrl2().modify(|_, w| unsafe {w.$mms().bits(trigger_source as u8)});
                }
            }
        )+
    }
}

macro_rules! hal {
    ($($TIM:ty: ($tim:ident),)+) => {
        $(
            impl Instance for $TIM { }

            impl CountDownTimer<$TIM> {
                /// Starts listening for an `event`
                ///
                /// Note, you will also have to enable the TIM2 interrupt in the NVIC to start
                /// receiving events.
                pub fn listen(&mut self, event: Event) {
                    match event {
                        Event::TimeOut => {
                            // Enable update event interrupt
                            self.tim.dinten().write(|w| w.uien().set_bit());
                        }
                    }
                }

                /// Clears interrupt associated with `event`.
                ///
                /// If the interrupt is not cleared, it will immediately retrigger after
                /// the ISR has finished.
                pub fn clear_interrupt(&mut self, event: Event) {
                    match event {
                        Event::TimeOut => {
                            // Clear interrupt flag
                            self.tim.sts().write(|w| w.uditf().clear_bit());
                        }
                    }
                }

                /// Stops listening for an `event`
                pub fn unlisten(&mut self, event: Event) {
                    match event {
                        Event::TimeOut => {
                            // Enable update event interrupt
                            self.tim.dinten().write(|w| w.uien().clear_bit());
                        }
                    }
                }

                /// Releases the TIM peripheral
                pub fn release(self) -> $TIM {
                    // pause counter
                    self.tim.ctrl1().modify(|_, w| w.cnten().clear_bit());
                    self.tim
                }
            }

            impl embedded_hal_02::timer::CountDown for CountDownTimer<$TIM> {
                type Time = MicroSecond;

                fn start<T>(&mut self, timeout: T)
                where
                    T: Into<MicroSecond>,
                {
                    // pause
                    self.tim.ctrl1().modify(|_, w| w.cnten().clear_bit());
                    // reset counter
                    self.tim.cnt().reset();

                    let ticks = crate::time::cycles(timeout.into(), self.clk);

                    let psc = u16((ticks - 1) / (1 << 16)).unwrap();
                    self.tim.psc().write(|w| unsafe {w.psc().bits(psc)} );

                    let arr = u16(ticks / u32(psc + 1)).unwrap();
                    self.tim.ar().write(|w| unsafe { w.bits(u32(arr)) });

                    // Trigger update event to load the registers
                    self.tim.ctrl1().modify(|_, w| w.uprs().set_bit());
                    self.tim.evtgen().write(|w| w.udgn().set_bit());
                    self.tim.ctrl1().modify(|_, w| w.uprs().clear_bit());

                    // start counter
                    self.tim.ctrl1().modify(|_, w| w.cnten().set_bit());
                }

                fn wait(&mut self) -> nb::Result<(), Void> {
                    if self.tim.sts().read().uditf().bit_is_clear() {
                        Err(nb::Error::WouldBlock)
                    } else {
                        self.tim.sts().modify(|_, w| w.uditf().clear_bit());
                        Ok(())
                    }
                }
            }

            impl CountDown for CountDownTimer<$TIM> {
                fn max_period(&self) -> MicroSecond {
                    crate::time::duration(self.clk, u16::MAX as u32)
                }
            }

            impl Cancel for CountDownTimer<$TIM>
            {
                type Error = Error;

                fn cancel(&mut self) -> Result<(), Self::Error> {
                    let is_counter_enabled = self.tim.ctrl1().read().cnten().bit_is_set();
                    if !is_counter_enabled {
                        return Err(Self::Error::Disabled);
                    }

                    // disable counter
                    self.tim.ctrl1().modify(|_, w| w.cnten().clear_bit());
                    Ok(())
                }
            }
        )+
    }
}

hal! {
    crate::pac::Tim1: (tim1),
    crate::pac::Tim2: (tim2),
    crate::pac::Tim3: (tim3),
    crate::pac::Tim4: (tim4),
    crate::pac::Tim6: (tim6),
    crate::pac::Tim7: (tim7),
    crate::pac::Tim8: (tim8),
}

hal_ext_trgo! {
    crate::pac::Tim1: (tim1, mmsel),
    crate::pac::Tim2: (tim2, mmsel),
    crate::pac::Tim3: (tim3, mmsel),
    crate::pac::Tim4: (tim4, mmsel),
    crate::pac::Tim6: (tim6, mmsel),
    crate::pac::Tim7: (tim7, mmsel),
    crate::pac::Tim8: (tim8, mmsel),
}
