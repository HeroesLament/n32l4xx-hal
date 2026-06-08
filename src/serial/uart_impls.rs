use core::{fmt, ops::Deref};

use enumflags2::BitFlags;
use nb::block;

use super::{
    config, CFlag, Error, Event, Flag, Rx, RxISR, RxListen, Serial, SerialExt, Tx, TxISR, TxListen,
};
use crate::gpio::Floating;
use crate::gpio::{alt::altmap::Remap, Input};
use crate::gpio::{alt::SerialAsync as CommonPins, NoPin, PushPull};
use crate::rcc::{self, Clocks};

pub(crate) use crate::pac::uart4::RegisterBlock as RegisterBlockUart;
pub(crate) use crate::pac::usart1::RegisterBlock as RegisterBlockUsart;

impl crate::Sealed for RegisterBlockUart {}
impl crate::Sealed for RegisterBlockUsart {}

// Implemented by all USART/UART instances
pub trait Instance: crate::Sealed + rcc::Enable + rcc::Reset + rcc::BusClock + CommonPins {
    type RegisterBlock: RegisterBlockImpl;

    #[doc(hidden)]
    fn ptr() -> *const Self::RegisterBlock;
    #[doc(hidden)]
    fn set_stopbits(&self, bits: config::StopBits);
}

pub trait RegisterBlockImpl: crate::Sealed {
    #[allow(clippy::new_ret_no_self)]
    fn new<UART: Instance<RegisterBlock = Self>, WORD>(
        uart: UART,
        pins: (impl Into<UART::Tx<PushPull>>, impl Into<UART::Rx<Floating>>),
        config: impl Into<config::Config>,
        clocks: &Clocks,
    ) -> Result<Serial<UART, WORD>, config::InvalidConfig>;

    fn read_u16(&self) -> nb::Result<u16, Error>;
    fn write_u16(&self, word: u16) -> nb::Result<(), Error>;

    fn read_u8(&self) -> nb::Result<u8, Error> {
        // Delegate to u16 version, then truncate to 8 bits
        self.read_u16().map(|word16| word16 as u8)
    }

    fn write_u8(&self, word: u8) -> nb::Result<(), Error> {
        // Delegate to u16 version
        self.write_u16(u16::from(word))
    }

    fn flush(&self) -> nb::Result<(), Error>;

    fn bwrite_all_u8(&self, buffer: &[u8]) -> Result<(), Error> {
        for &b in buffer {
            nb::block!(self.write_u8(b))?;
        }
        Ok(())
    }

    fn bwrite_all_u16(&self, buffer: &[u16]) -> Result<(), Error> {
        for &b in buffer {
            nb::block!(self.write_u16(b))?;
        }
        Ok(())
    }

    fn bflush(&self) -> Result<(), Error> {
        nb::block!(self.flush())
    }

    // ISR
    fn flags(&self) -> BitFlags<Flag>;

    fn is_idle(&self) -> bool {
        self.flags().contains(Flag::Idle)
    }
    fn is_rx_not_empty(&self) -> bool {
        self.flags().contains(Flag::RxNotEmpty)
    }
    fn is_tx_empty(&self) -> bool {
        self.flags().contains(Flag::TxEmpty)
    }
    fn clear_flags(&self, flags: BitFlags<CFlag>);
    fn clear_idle_interrupt(&self);

    // Listen
    fn listen_event(&self, disable: Option<BitFlags<Event>>, enable: Option<BitFlags<Event>>);

    #[inline(always)]
    fn listen_rxne(&self) {
        self.listen_event(None, Some(Event::RxNotEmpty.into()))
    }
    #[inline(always)]
    fn unlisten_rxne(&self) {
        self.listen_event(Some(Event::RxNotEmpty.into()), None)
    }
    #[inline(always)]
    fn listen_idle(&self) {
        self.listen_event(None, Some(Event::Idle.into()))
    }
    #[inline(always)]
    fn unlisten_idle(&self) {
        self.listen_event(Some(Event::Idle.into()), None)
    }
    #[inline(always)]
    fn listen_txe(&self) {
        self.listen_event(None, Some(Event::TxEmpty.into()))
    }
    #[inline(always)]
    fn unlisten_txe(&self) {
        self.listen_event(Some(Event::TxEmpty.into()), None)
    }

    // PeriAddress
    fn peri_address(&self) -> u32;
}

macro_rules! uartCommon {
    ($RegisterBlock:ty) => {
        impl RegisterBlockImpl for $RegisterBlock {
            fn new<UART: Instance<RegisterBlock = Self>, WORD>(
                uart: UART,
                pins: (impl Into<UART::Tx<PushPull>>, impl Into<UART::Rx<Floating>>),
                config: impl Into<config::Config>,
                clocks: &Clocks,
            ) -> Result<Serial<UART, WORD>, config::InvalidConfig>
        where {
                use self::config::*;

                let config = config.into();
                unsafe {
                    // Enable clock.
                    UART::enable_unchecked();
                    UART::reset_unchecked();
                }

                let pclk_freq = UART::clock(clocks).raw();
                let baud = config.baudrate.0;

                let div = if (pclk_freq / 16) >= baud {

                    let integerdivider = ((25 * pclk_freq) / (4 * (baud)));
                    let mut tmpregister = (integerdivider / 100) << 4;
                
                    let fractionaldivider = (((((integerdivider - (100 * (tmpregister >> 4))) * 16) + 50) / 100));
                
                    if((fractionaldivider >> 4) == 1){
                        tmpregister = ((integerdivider / 100) + 1) << 4;
                    }
                    
                    /* Implement the fractional part in the register */
                    tmpregister |= fractionaldivider & (0x0F);
                    tmpregister
                } else {
                    return Err(config::InvalidConfig);
                };

                let register_block = unsafe { &*UART::ptr() };
                // Reset other registers to disable advanced USART features
                register_block.ctrl2().reset();
                register_block.ctrl3().reset();

                // Enable transmission and receiving
                // and configure frame

                let serial = Serial {
                    tx: Tx::new(uart, pins.0.into()),
                    rx: Rx::new(pins.1.into()),
                };
                serial.tx.usart.set_stopbits(config.stopbits);
                register_block.ctrl1().modify(|_,w| {
                    w.wl().bit(config.wordlength == WordLength::DataBits9)
                     .pcen().bit(config.parity != Parity::ParityNone)
                     .psel().bit(config.parity == Parity::ParityOdd)
                     .txen().set_bit()
                     .rxen().set_bit()
                });
                register_block.brcf().write(|w| unsafe { w.bits(div) });
                register_block.ctrl1().modify(|_,w| {
                    w.uen().set_bit()
                });
                match config.dma {
                    DmaConfig::Tx => register_block.ctrl3().modify(|_,w| w.dmatxen().set_bit()),
                    DmaConfig::Rx => register_block.ctrl3().modify(|_,w| w.dmarxen().set_bit()),
                    DmaConfig::TxRx => register_block
                        .ctrl3()
                        .modify(|_,w| w.dmarxen().set_bit().dmatxen().set_bit()),
                    DmaConfig::None => {}
                };
                Ok(serial)
            }

            fn read_u16(&self) -> nb::Result<u16, Error> {
                // NOTE(unsafe) atomic read with no side effects
                let sr = self.sts().read();

                // Any error requires the dr to be read to clear
                if sr.pef().bit_is_set()
                    || sr.fef().bit_is_set()
                    || sr.nef().bit_is_set()
                    || sr.oref().bit_is_set()
                {
                    self.dat().read();
                }

                Err(if sr.pef().bit_is_set() {
                    Error::Parity.into()
                } else if sr.fef().bit_is_set() {
                    Error::FrameFormat.into()
                } else if sr.nef().bit_is_set() {
                    Error::Noise.into()
                } else if sr.oref().bit_is_set() {
                    Error::Overrun.into()
                } else if sr.rxdne().bit_is_set() {
                    // NOTE(unsafe) atomic read from stateless register
                    return Ok(self.dat().read().datv().bits());
                } else {
                    nb::Error::WouldBlock
                })
            }

            fn write_u16(&self, word: u16) -> nb::Result<(), Error> {
                // NOTE(unsafe) atomic read with no side effects
                if self.sts().read().txde().bit_is_set() {
                    // NOTE(unsafe) atomic write to stateless register
                    unsafe { self.dat().write(|w| w.datv().bits(word))};
                    Ok(())
                } else {
                    Err(nb::Error::WouldBlock)
                }
            }

            fn flush(&self) -> nb::Result<(), Error> {
                // NOTE(unsafe) atomic read with no side effects
                let sr = self.sts().read();

                if sr.txc().bit_is_set() {
                    Ok(())
                } else {
                    Err(nb::Error::WouldBlock)
                }
            }

            fn flags(&self) -> BitFlags<Flag> {
                BitFlags::from_bits_truncate(self.sts().read().bits())
            }

            fn clear_flags(&self, flags: BitFlags<CFlag>) {
                self.sts().write(|w| unsafe { w.bits(0xffff & !flags.bits()) });
            }

            fn clear_idle_interrupt(&self) {
                let _ = self.sts().read();
                let _ = self.dat().read();
            }

            fn listen_event(
                &self,
                disable: Option<BitFlags<Event>>,
                enable: Option<BitFlags<Event>>,
            ) {
                self.ctrl1().modify(|r, w| unsafe {
                    w.bits({
                        let mut bits = r.bits();
                        if let Some(d) = disable {
                            bits &= !(d.bits() as u32);
                        }
                        if let Some(e) = enable {
                            bits |= e.bits() as u32;
                        }
                        bits
                    })
                });
            }

            fn peri_address(&self) -> u32 {
                self.dat().as_ptr() as u32
            }
        }
    };
}

uartCommon! { RegisterBlockUsart }
uartCommon! { RegisterBlockUart }

impl<UART: Instance, WORD> RxISR for Serial<UART, WORD>
where
    Rx<UART, WORD>: RxISR,
{
    fn is_idle(&self) -> bool {
        self.rx.is_idle()
    }

    fn is_rx_not_empty(&self) -> bool {
        self.rx.is_rx_not_empty()
    }

    /// This clears `Idle`, `Overrun`, `Noise`, `FrameError` and `ParityError` flags
    fn clear_idle_interrupt(&self) {
        self.rx.clear_idle_interrupt();
    }
}

impl<UART: Instance, WORD> RxISR for Rx<UART, WORD> {
    fn is_idle(&self) -> bool {
        unsafe { (*UART::ptr()).is_idle() }
    }

    fn is_rx_not_empty(&self) -> bool {
        unsafe { (*UART::ptr()).is_rx_not_empty() }
    }

    /// This clears `Idle`, `Overrun`, `Noise`, `FrameError` and `ParityError` flags
    fn clear_idle_interrupt(&self) {
        unsafe {
            (*UART::ptr()).clear_idle_interrupt();
        }
    }
}

impl<UART: Instance, WORD> TxISR for Serial<UART, WORD>
where
    Tx<UART, WORD>: TxISR,
{
    fn is_tx_empty(&self) -> bool {
        self.tx.is_tx_empty()
    }
}

impl<UART: Instance, WORD> TxISR for Tx<UART, WORD>
where
    UART: Deref<Target = <UART as Instance>::RegisterBlock>,
{
    fn is_tx_empty(&self) -> bool {
        self.usart.is_tx_empty()
    }
}

impl<UART: Instance, WORD> RxListen for Rx<UART, WORD> {
    fn listen(&mut self) {
        unsafe { (*UART::ptr()).listen_rxne() }
    }

    fn unlisten(&mut self) {
        unsafe { (*UART::ptr()).unlisten_rxne() }
    }

    fn listen_idle(&mut self) {
        unsafe { (*UART::ptr()).listen_idle() }
    }

    fn unlisten_idle(&mut self) {
        unsafe { (*UART::ptr()).unlisten_idle() }
    }
}

impl<UART: Instance, WORD> TxListen for Tx<UART, WORD>
where
    UART: Deref<Target = <UART as Instance>::RegisterBlock>,
{
    fn listen(&mut self) {
        self.usart.listen_txe()
    }

    fn unlisten(&mut self) {
        self.usart.unlisten_txe()
    }
}

impl<UART: Instance, WORD> crate::ClearFlags for Serial<UART, WORD>
where
    UART: Deref<Target = <UART as Instance>::RegisterBlock>,
{
    type Flag = CFlag;

    #[inline(always)]
    fn clear_flags(&mut self, flags: impl Into<BitFlags<Self::Flag>>) {
        self.tx.usart.clear_flags(flags.into())
    }
}

impl<UART: Instance, WORD> crate::ReadFlags for Serial<UART, WORD>
where
    UART: Deref<Target = <UART as Instance>::RegisterBlock>,
{
    type Flag = Flag;

    #[inline(always)]
    fn flags(&self) -> BitFlags<Self::Flag> {
        self.tx.usart.flags()
    }
}

impl<UART: Instance, WORD> crate::Listen for Serial<UART, WORD>
where
    UART: Deref<Target = <UART as Instance>::RegisterBlock>,
{
    type Event = Event;

    #[inline(always)]
    fn listen(&mut self, event: impl Into<BitFlags<Event>>) {
        self.tx.usart.listen_event(None, Some(event.into()));
    }

    #[inline(always)]
    fn listen_only(&mut self, event: impl Into<BitFlags<Self::Event>>) {
        self.tx
            .usart
            .listen_event(Some(BitFlags::ALL), Some(event.into()));
    }

    #[inline(always)]
    fn unlisten(&mut self, event: impl Into<BitFlags<Event>>) {
        self.tx.usart.listen_event(Some(event.into()), None);
    }
}

impl<UART: Instance> fmt::Write for Serial<UART>
where
    Tx<UART>: fmt::Write,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.tx.write_str(s)
    }
}

impl<UART: Instance> fmt::Write for Tx<UART>
where
    UART: Deref<Target = <UART as Instance>::RegisterBlock>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        s.bytes()
            .try_for_each(|c| block!(self.usart.write_u8(c)))
            .map_err(|_| fmt::Error)
    }
}

impl<UART: Instance> SerialExt for UART {
    fn serial<WORD,RMP : Remap,TX: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Tx<PushPull>>,RX : crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Rx<Floating>>>(
        self,
        pins: (TX,RX),
        config: impl Into<config::Config>,
        clocks: &Clocks,
        afio: &mut crate::pac::Afio
    ) -> Result<Serial<Self, WORD>, config::InvalidConfig> {
        RMP::remap(afio);
        Serial::new(self, (pins.0.into(),pins.1.into()), config, clocks,afio)
    }
    fn tx<WORD,RMP : Remap,TX: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Tx<PushPull>>>(
        self,
        tx_pin: TX,
        config: impl Into<config::Config>,
        clocks: &Clocks,
        afio: &mut crate::pac::Afio
    ) -> Result<Tx<Self, WORD>, config::InvalidConfig>
    where
        NoPin<Input>: Into<Self::Rx<Floating>>,
    {
        RMP::remap(afio);
        Serial::tx(self, tx_pin, config, clocks,afio)
    }
    fn rx<WORD,RMP : Remap,RX: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Rx<Floating>>>(
        self,
        rx_pin: RX,
        config: impl Into<config::Config>,
        clocks: &Clocks,
        afio: &mut crate::pac::Afio
    ) -> Result<Rx<Self, WORD>, config::InvalidConfig>
    where
        NoPin<PushPull>: Into<Self::Tx<PushPull>>,
    {
        RMP::remap(afio);
        Serial::rx(self, rx_pin, config, clocks,afio)
    }
}

impl<UART: Instance, WORD> Serial<UART, WORD> {
    pub fn tx(
        usart: UART,
        tx_pin: impl Into<UART::Tx<PushPull>>,
        config: impl Into<config::Config>,
        clocks: &Clocks,
        afio: &mut crate::pac::Afio
    ) -> Result<Tx<UART, WORD>, config::InvalidConfig>
    where
        NoPin<Input>: Into<UART::Rx<Floating>>,
    {
        Self::new(usart, (tx_pin.into(), NoPin::new().into()) , config, clocks,afio).map(|s| s.split().0)
    }
}

impl<UART: Instance, WORD> Serial<UART, WORD> {
    pub fn rx(
        usart: UART,
        rx_pin: impl Into<UART::Rx<Floating>>,
        config: impl Into<config::Config>,
        clocks: &Clocks,
        afio: &mut crate::pac::Afio
    ) -> Result<Rx<UART, WORD>, config::InvalidConfig>
    where
    NoPin<PushPull>: Into<UART::Tx<PushPull>>,
    {
        Self::new(usart, (NoPin::new().into(), rx_pin.into()), config, clocks,afio).map(|s| s.split().1)
    }
}

// unsafe impl<UART: Instance> PeriAddress for Rx<UART, u8> {
//     #[inline(always)]
//     fn address(&self) -> u32 {
//         unsafe { (*UART::ptr()).peri_address() }
//     }

//     type MemSize = u8;
// }

// unsafe impl<UART: CommonPins, STREAM> DMASet<STREAM, PeripheralToMemory>
//     for Rx<UART>
// where
//     UART: DMASet<STREAM, PeripheralToMemory>,
// {
// }

// unsafe impl<UART: Instance> PeriAddress for Tx<UART, u8>
// where
//     UART: Deref<Target = <UART as Instance>::RegisterBlock>,
// {
//     #[inline(always)]
//     fn address(&self) -> u32 {
//         self.usart.peri_address()
//     }

//     type MemSize = u8;
// }

// unsafe impl<UART: CommonPins, STREAM> DMASet<STREAM, MemoryToPeripheral>
//     for Tx<UART>
// where
//     UART: DMASet<STREAM, MemoryToPeripheral>,
// {
// }