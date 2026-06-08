use super::*;

impl<const P: char, const N: u8> Pin<P, N, Alternate<PushPull>> {
    /// Turns pin alternate configuration pin into open drain
    pub fn set_open_drain(self) -> Pin<P, N, Alternate<OpenDrain>> {
        self.into_mode()
    }
}

impl<const P: char, const N: u8, MODE: PinMode> Pin<P, N, MODE> {
    /// Configures the pin to operate alternate mode
    pub fn into_alternate(self) -> Pin<P, N, Alternate<PushPull>>
    {
        self.into_mode()
    }

    /// Configures the pin to operate in alternate open drain mode
    #[allow(path_statements)]
    pub fn into_alternate_open_drain(self) -> Pin<P, N, Alternate<OpenDrain>>
    {
        self.into_mode()
    }

    /// Configures the pin to operate as a floating input pin
    pub fn into_floating_input(self) -> Pin<P, N, Input<Floating>> {
        self.into_mode()
    }

    /// Configures the pin to operate as a pulled down input pin
    pub fn into_pull_down_input(self) -> Pin<P, N, Input<PullDown>> {
        self.into_mode()
    }

    /// Configures the pin to operate as a pulled up input pin
    pub fn into_pull_up_input(self) -> Pin<P, N, Input<PullUp>> {
        self.into_mode()
    }

    /// Configures the pin to operate as an open drain output pin
    /// Initial state will be low.
    pub fn into_open_drain_output(self) -> Pin<P, N, Output<OpenDrain>> {
        self.into_mode()
    }

    /// Configures the pin to operate as an open-drain output pin.
    /// `initial_state` specifies whether the pin should be initially high or low.
    pub fn into_open_drain_output_in_state(
        mut self,
        initial_state: PinState,
    ) -> Pin<P, N, Output<OpenDrain>> {
        self._set_state(initial_state);
        self.into_mode()
    }

    /// Configures the pin to operate as an push pull output pin
    /// Initial state will be low.
    pub fn into_push_pull_output(mut self) -> Pin<P, N, Output<PushPull>> {
        self._set_low();
        self.into_mode()
    }

    /// Configures the pin to operate as an push-pull output pin.
    /// `initial_state` specifies whether the pin should be initially high or low.
    pub fn into_push_pull_output_in_state(
        mut self,
        initial_state: PinState,
    ) -> Pin<P, N, Output<PushPull>> {
        self._set_state(initial_state);
        self.into_mode()
    }

    /// Configures the pin to operate as an analog input pin
    pub fn into_analog(self) -> Pin<P, N, Analog> {
        self.into_mode()
    }

    /// Configures the pin as a pin that can change between input
    /// and output without changing the type. It starts out
    /// as a floating input
    pub fn into_dynamic(self) -> DynamicPin<P, N> {
        self.into_floating_input();
        DynamicPin::new(Dynamic::InputFloating)
    }

    /// Puts `self` into mode `M`.
    ///
    /// This violates the type state constraints from `MODE`, so callers must
    /// ensure they use this properly.
    #[inline(always)]
    pub(super) fn mode<M: PinMode>(&mut self) {
        // Input<PullUp> or Input<PullDown> mode
        let gpio = unsafe { &(*crate::gpio::gpiox::<P>()) };

        if let Some(pull) = MODE::PULL {
            if pull {
                gpio.pbsc().write(|w| unsafe { w.bits(1 << N) });
            } else {
                gpio.pbc().write(|w| unsafe { w.bits(1 << N) });
            }
        }
        
        match self.pin_id() {
            0 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg0().bits(M::CNF as u8).pmode0().bits(M::MODE as u8) }),
            1 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg1().bits(M::CNF as u8).pmode1().bits(M::MODE as u8) }),
            2 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg2().bits(M::CNF as u8).pmode2().bits(M::MODE as u8) }),
            3 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg3().bits(M::CNF as u8).pmode3().bits(M::MODE as u8) }),
            4 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg4().bits(M::CNF as u8).pmode4().bits(M::MODE as u8) }),
            5 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg5().bits(M::CNF as u8).pmode5().bits(M::MODE as u8) }),
            6 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg6().bits(M::CNF as u8).pmode6().bits(M::MODE as u8) }),
            7 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg7().bits(M::CNF as u8).pmode7().bits(M::MODE as u8) }),
            8 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg8().bits(M::CNF as u8).pmode8().bits(M::MODE as u8) }),
            9 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg9().bits(M::CNF as u8).pmode9().bits(M::MODE as u8) }),
            10 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg10().bits(M::CNF as u8).pmode10().bits(M::MODE as u8) }),
            11 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg11().bits(M::CNF as u8).pmode11().bits(M::MODE as u8) }),
            12 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg12().bits(M::CNF as u8).pmode12().bits(M::MODE as u8) }),
            13 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg13().bits(M::CNF as u8).pmode13().bits(M::MODE as u8) }),
            14 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg14().bits(M::CNF as u8).pmode14().bits(M::MODE as u8) }),
            15 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg15().bits(M::CNF as u8).pmode15().bits(M::MODE as u8) }),
            _ => unreachable!()
        };
    }

    #[inline(always)]
    /// Converts pin into specified mode
    pub fn into_mode<M: PinMode>(mut self) -> Pin<P, N, M> {
        self.mode::<M>();
        Pin::new()
    }
}

impl<MODE: PinMode> ErasedPin<MODE> {
    #[inline(always)]
    pub(super) fn mode<M: PinMode>(&mut self) {
        let n = self.pin_id();
        // Input<PullUp> or Input<PullDown> mode
        let gpio = self.block();

        if let Some(pull) = MODE::PULL {
            if pull {
                gpio.pbsc().write(|w| unsafe { w.bits(1 << n) });
            } else {
                gpio.pbc().write(|w| unsafe { w.bits(1 << n) });
            }
        }


        match self.pin_id() {
            0 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg0().bits(M::CNF as u8).pmode0().bits(M::MODE as u8) }),
            1 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg1().bits(M::CNF as u8).pmode1().bits(M::MODE as u8) }),
            2 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg2().bits(M::CNF as u8).pmode2().bits(M::MODE as u8) }),
            3 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg3().bits(M::CNF as u8).pmode3().bits(M::MODE as u8) }),
            4 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg4().bits(M::CNF as u8).pmode4().bits(M::MODE as u8) }),
            5 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg5().bits(M::CNF as u8).pmode5().bits(M::MODE as u8) }),
            6 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg6().bits(M::CNF as u8).pmode6().bits(M::MODE as u8) }),
            7 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg7().bits(M::CNF as u8).pmode7().bits(M::MODE as u8) }),
            8 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg8().bits(M::CNF as u8).pmode8().bits(M::MODE as u8) }),
            9 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg9().bits(M::CNF as u8).pmode9().bits(M::MODE as u8) }),
            10 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg10().bits(M::CNF as u8).pmode10().bits(M::MODE as u8) }),
            11 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg11().bits(M::CNF as u8).pmode11().bits(M::MODE as u8) }),
            12 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg12().bits(M::CNF as u8).pmode12().bits(M::MODE as u8) }),
            13 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg13().bits(M::CNF as u8).pmode13().bits(M::MODE as u8) }),
            14 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg14().bits(M::CNF as u8).pmode14().bits(M::MODE as u8) }),
            15 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg15().bits(M::CNF as u8).pmode15().bits(M::MODE as u8) }),
            _ => unreachable!()
        };
    }

    #[inline(always)]
    /// Converts pin into specified mode
    pub fn into_mode<M: PinMode>(mut self) -> ErasedPin<M> {
        self.mode::<M>();
        ErasedPin::from_pin_port(self.into_pin_port())
    }
}

impl<const P: char, MODE: PinMode> PartiallyErasedPin<P, MODE> {
    #[inline(always)]
    pub(super) fn mode<M: PinMode>(&mut self) {
        let n = self.pin_id();
        // Input<PullUp> or Input<PullDown> mode
        let gpio = unsafe { &(*crate::gpio::gpiox::<P>()) };
        if let Some(pull) = MODE::PULL {
            if pull {
                gpio.pbsc().write(|w| unsafe { w.bits(1 << n) });
            } else {
                gpio.pbc().write(|w| unsafe { w.bits(1 << n) });
            }
        }


        match self.pin_id() {
            0 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg0().bits(M::CNF as u8).pmode0().bits(M::MODE as u8) }),
            1 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg1().bits(M::CNF as u8).pmode1().bits(M::MODE as u8) }),
            2 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg2().bits(M::CNF as u8).pmode2().bits(M::MODE as u8) }),
            3 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg3().bits(M::CNF as u8).pmode3().bits(M::MODE as u8) }),
            4 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg4().bits(M::CNF as u8).pmode4().bits(M::MODE as u8) }),
            5 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg5().bits(M::CNF as u8).pmode5().bits(M::MODE as u8) }),
            6 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg6().bits(M::CNF as u8).pmode6().bits(M::MODE as u8) }),
            7 =>  gpio.pl_cfg().modify(|_,w| unsafe { w.pcfg7().bits(M::CNF as u8).pmode7().bits(M::MODE as u8) }),
            8 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg8().bits(M::CNF as u8).pmode8().bits(M::MODE as u8) }),
            9 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg9().bits(M::CNF as u8).pmode9().bits(M::MODE as u8) }),
            10 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg10().bits(M::CNF as u8).pmode10().bits(M::MODE as u8) }),
            11 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg11().bits(M::CNF as u8).pmode11().bits(M::MODE as u8) }),
            12 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg12().bits(M::CNF as u8).pmode12().bits(M::MODE as u8) }),
            13 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg13().bits(M::CNF as u8).pmode13().bits(M::MODE as u8) }),
            14 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg14().bits(M::CNF as u8).pmode14().bits(M::MODE as u8) }),
            15 =>  gpio.ph_cfg().modify(|_,w| unsafe { w.pcfg15().bits(M::CNF as u8).pmode15().bits(M::MODE as u8) }),
            _ => unreachable!()
        };
    }

    #[inline(always)]
    /// Converts pin into specified mode
    pub fn into_mode<M: PinMode>(mut self) -> PartiallyErasedPin<P, M> {
        self.mode::<M>();
        PartiallyErasedPin::new(self.i)
    }
}

impl<const P: char, const N: u8, MODE> Pin<P, N, MODE>
where
    MODE: PinMode,
{
    fn with_mode<M, F, R>(&mut self, f: F) -> R
    where
        M: PinMode,
        F: FnOnce(&mut Pin<P, N, M>) -> R,
    {
        self.mode::<M>(); // change physical mode, without changing typestate

        // This will reset the pin back to the original mode when dropped.
        // (so either when `with_mode` returns or when `f` unwinds)
        let mut resetti = ResetMode::<P, N, M, MODE>::new();

        f(&mut resetti.pin)
    }

    /// Temporarily configures this pin as a floating input.
    ///
    /// The closure `f` is called with the reconfigured pin. After it returns,
    /// the pin will be configured back.
    pub fn with_floating_input<R>(&mut self, f: impl FnOnce(&mut Pin<P, N, Input<Floating>>) -> R) -> R {
        self.with_mode(f)
    }

    /// Temporarily configures this pin as a pull-up input.
    ///
    /// The closure `f` is called with the reconfigured pin. After it returns,
    /// the pin will be configured back.
    pub fn with_pull_up_input<R>(&mut self, f: impl FnOnce(&mut Pin<P, N, Input<PullUp>>) -> R) -> R {
        self.with_mode(f)
    }

    /// Temporarily configures this pin as a pull-down input.
    ///
    /// The closure `f` is called with the reconfigured pin. After it returns,
    /// the pin will be configured back.
    pub fn with_pull_down_input<R>(&mut self, f: impl FnOnce(&mut Pin<P, N, Input<PullDown>>) -> R) -> R {
        self.with_mode(f)
    }

    /// Temporarily configures this pin as an analog pin.
    ///
    /// The closure `f` is called with the reconfigured pin. After it returns,
    /// the pin will be configured back.
    pub fn with_analog<R>(&mut self, f: impl FnOnce(&mut Pin<P, N, Analog>) -> R) -> R {
        self.with_mode(f)
    }

    /// Temporarily configures this pin as an open drain output.
    ///
    /// The closure `f` is called with the reconfigured pin. After it returns,
    /// the pin will be configured back.
    /// The value of the pin after conversion is undefined. If you
    /// want to control it, use `with_open_drain_output_in_state`
    pub fn with_open_drain_output<R>(
        &mut self,
        f: impl FnOnce(&mut Pin<P, N, Output<OpenDrain>>) -> R,
    ) -> R {
        self.with_mode(f)
    }

    /// Temporarily configures this pin as an open drain output .
    ///
    /// The closure `f` is called with the reconfigured pin. After it returns,
    /// the pin will be configured back.
    /// Note that the new state is set slightly before conversion
    /// happens. This can cause a short output glitch if switching
    /// between output modes
    pub fn with_open_drain_output_in_state<R>(
        &mut self,
        state: PinState,
        f: impl FnOnce(&mut Pin<P, N, Output<OpenDrain>>) -> R,
    ) -> R {
        self._set_state(state);
        self.with_mode(f)
    }

    /// Temporarily configures this pin as a push-pull output.
    ///
    /// The closure `f` is called with the reconfigured pin. After it returns,
    /// the pin will be configured back.
    /// The value of the pin after conversion is undefined. If you
    /// want to control it, use `with_push_pull_output_in_state`
    pub fn with_push_pull_output<R>(
        &mut self,
        f: impl FnOnce(&mut Pin<P, N, Output<PushPull>>) -> R,
    ) -> R {
        self.with_mode(f)
    }

    /// Temporarily configures this pin as a push-pull output.
    ///
    /// The closure `f` is called with the reconfigured pin. After it returns,
    /// the pin will be configured back.
    /// Note that the new state is set slightly before conversion
    /// happens. This can cause a short output glitch if switching
    /// between output modes
    pub fn with_push_pull_output_in_state<R>(
        &mut self,
        state: PinState,
        f: impl FnOnce(&mut Pin<P, N, Output<PushPull>>) -> R,
    ) -> R {
        self._set_state(state);
        self.with_mode(f)
    }
}

/// Wrapper around a pin that transitions the pin to mode ORIG when dropped
struct ResetMode<const P: char, const N: u8, CURRENT: PinMode, ORIG: PinMode> {
    pub pin: Pin<P, N, CURRENT>,
    _mode: PhantomData<ORIG>,
}
impl<const P: char, const N: u8, CURRENT: PinMode, ORIG: PinMode> ResetMode<P, N, CURRENT, ORIG> {
    fn new() -> Self {
        Self {
            pin: Pin::new(),
            _mode: PhantomData,
        }
    }
}
impl<const P: char, const N: u8, CURRENT: PinMode, ORIG: PinMode> Drop
    for ResetMode<P, N, CURRENT, ORIG>
{
    fn drop(&mut self) {
        self.pin.mode::<ORIG>();
    }
}

/// Marker trait for valid pin modes (type state).
///
/// It can not be implemented by outside types.
pub trait PinMode: Default {
    const CNF: u32;
    const MODE: u32;
    const PULL: Option<bool> = None;
}

impl PinMode for Input<Floating> {
    const CNF: u32 = 0b01;
    const MODE: u32 = 0b00;
    const PULL: Option<bool> = None;
}

impl PinMode for Input<PullDown> {
    const CNF: u32 = 0b10;
    const MODE: u32 = 0b00;
    const PULL: Option<bool> = Some(false);
}

impl PinMode for Input<PullUp> {
    const CNF: u32 = 0b10;
    const MODE: u32 = 0b00;
    const PULL: Option<bool> = Some(true);
}

impl PinMode for Output<OpenDrain> {
    const CNF: u32 = 0b01;
    const MODE: u32 = 0b11;
}

impl PinMode for Output<PushPull> {
    const CNF: u32 = 0b00;
    const MODE: u32 = 0b11;
}

impl PinMode for Analog {
    const CNF: u32 = 0b00;
    const MODE: u32 = 0b00;
}

impl PinMode for Alternate<PushPull> {
    const CNF: u32 = 0b10;
    const MODE: u32 = 0b11;
}

impl PinMode for Alternate<Input> {
    const CNF: u32 = 0b01;
    const MODE: u32 = 0b00;
}


impl PinMode for Alternate<OpenDrain> {
    const CNF: u32 = 0b11;
    const MODE: u32 = 0b11;
}
