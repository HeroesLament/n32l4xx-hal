use super::*;

/// Apply the L4-model pin configuration for pin `n` on a gpio register block:
/// PMODE (2-bit), optional POTYPE (1-bit), PUPD (2-bit), and optional AF code
/// into AFL/AFH (4-bit nibble). All via raw bits to avoid a 16-arm match.
#[inline(always)]
pub(super) fn set_mode_bits(
    gpio: &crate::pac::gpioa::RegisterBlock,
    n: u8,
    pmode: u32,
    otype: Option<bool>,
    pupd: u32,
    afnum: Option<u32>,
) {
    let n = n as u32;
    // PMODE: 2 bits per pin
    gpio.pmode().modify(|r, w| unsafe {
        w.bits((r.bits() & !(0b11 << (2 * n))) | (pmode << (2 * n)))
    });
    // PUPD: 2 bits per pin
    gpio.pupd().modify(|r, w| unsafe {
        w.bits((r.bits() & !(0b11 << (2 * n))) | (pupd << (2 * n)))
    });
    // POTYPE: 1 bit per pin (only when the mode specifies it)
    if let Some(od) = otype {
        gpio.potype().modify(|r, w| unsafe {
            w.bits((r.bits() & !(1 << n)) | ((od as u32) << n))
        });
    }
    // Alternate function selector: 4 bits per pin, AFL (0..7) / AFH (8..15)
    if let Some(af) = afnum {
        if n < 8 {
            gpio.afl().modify(|r, w| unsafe {
                w.bits((r.bits() & !(0xF << (4 * n))) | (af << (4 * n)))
            });
        } else {
            let nn = n - 8;
            gpio.afh().modify(|r, w| unsafe {
                w.bits((r.bits() & !(0xF << (4 * nn))) | (af << (4 * nn)))
            });
        }
    }
}

impl<const P: char, const N: u8> Pin<P, N, Alternate<0, PushPull>> {
    /// Turns pin alternate configuration pin into open drain
    pub fn set_open_drain(self) -> Pin<P, N, Alternate<0, OpenDrain>> {
        self.into_mode()
    }
}

impl<const P: char, const N: u8, MODE: PinMode> Pin<P, N, MODE> {
    /// Configures the pin to operate alternate mode
    pub fn into_alternate<const A: u8>(self) -> Pin<P, N, Alternate<A, PushPull>>
    where
        Alternate<A, PushPull>: PinMode,
    {
        self.into_mode()
    }

    /// Configures the pin to operate in alternate open drain mode
    pub fn into_alternate_open_drain<const A: u8>(self) -> Pin<P, N, Alternate<A, OpenDrain>>
    where
        Alternate<A, OpenDrain>: PinMode,
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
        let gpio = unsafe { &(*crate::gpio::gpiox::<P>()) };
        set_mode_bits(gpio, N, M::PMODE, M::OTYPE, M::PUPD, M::AFNUM);
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
        let gpio = self.block();
        set_mode_bits(gpio, n, M::PMODE, M::OTYPE, M::PUPD, M::AFNUM);
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
        let gpio = unsafe { &(*crate::gpio::gpiox::<P>()) };
        set_mode_bits(gpio, n, M::PMODE, M::OTYPE, M::PUPD, M::AFNUM);
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
    /// PMODE field value (00 input, 01 output, 10 alternate, 11 analog)
    const PMODE: u32;
    /// POTYPE bit (false push-pull, true open-drain). None = don't touch.
    const OTYPE: Option<bool> = None;
    /// PUPD field value (00 none, 01 pull-up, 10 pull-down)
    const PUPD: u32 = 0b00;
    /// Alternate function number to write into afsel (only for Alternate)
    const AFNUM: Option<u32> = None;
}

impl PinMode for Input<Floating> {
    const PMODE: u32 = 0b00;
    const PUPD: u32 = 0b00;
}

impl PinMode for Input<PullDown> {
    const PMODE: u32 = 0b00;
    const PUPD: u32 = 0b10;
}

impl PinMode for Input<PullUp> {
    const PMODE: u32 = 0b00;
    const PUPD: u32 = 0b01;
}

impl PinMode for Output<OpenDrain> {
    const PMODE: u32 = 0b01;
    const OTYPE: Option<bool> = Some(true);
}

impl PinMode for Output<PushPull> {
    const PMODE: u32 = 0b01;
    const OTYPE: Option<bool> = Some(false);
}

impl PinMode for Analog {
    const PMODE: u32 = 0b11;
}

impl<const A: u8> PinMode for Alternate<A, PushPull> {
    const PMODE: u32 = 0b10;
    const OTYPE: Option<bool> = Some(false);
    const AFNUM: Option<u32> = Some(A as u32);
}

impl<const A: u8> PinMode for Alternate<A, OpenDrain> {
    const PMODE: u32 = 0b10;
    const OTYPE: Option<bool> = Some(true);
    const AFNUM: Option<u32> = Some(A as u32);
}
