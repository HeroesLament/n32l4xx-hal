//!
//! Asynchronous serial communication using USART peripherals
//!
//! # Word length
//!
//! By default, the UART/USART uses 8 data bits. The `Serial`, `Rx`, and `Tx` structs implement
//! the embedded-hal read and write traits with `u8` as the word type.
//!
//! You can also configure the hardware to use 9 data bits with the `Config` `wordlength_9()`
//! function. After creating a `Serial` with this option, use the `with_u16_data()` function to
//! convert the `Serial<_, u8>` object into a `Serial<_, u16>` that can send and receive `u16`s.
//!
//! In this mode, the `Serial<_, u16>`, `Rx<_, u16>`, and `Tx<_, u16>` structs instead implement
//! the embedded-hal read and write traits with `u16` as the word type. You can use these
//! implementations for 9-bit words.

use core::marker::PhantomData;
use embedded_dma::WriteBuffer;
mod hal_02;
mod hal_1;

pub(crate) mod uart_impls;
pub use uart_impls::Instance;
use uart_impls::RegisterBlockImpl;

use crate::gpio::alt::altmap::Remap;
use crate::gpio::{self, Floating, Input, PushPull};

use crate::pac;

use crate::gpio::NoPin;
use crate::rcc::Clocks;

/// Serial error kind
///
/// This represents a common set of serial operation errors. HAL implementations are
/// free to define more specific or additional error types. However, by providing
/// a mapping to these common serial errors, generic code can still react to them.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum Error {
    /// The peripheral receive buffer was overrun.
    Overrun,
    /// Received data does not conform to the peripheral configuration.
    /// Can be caused by a misconfigured device on either end of the serial line.
    FrameFormat,
    /// Parity check failed.
    Parity,
    /// Serial line is too noisy to read valid data.
    Noise,
    /// A different error occurred. The original error may contain more information.
    Other,
}

/// UART interrupt events
#[enumflags2::bitflags]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(u32)]
pub enum Event {
    /// IDLE interrupt enable
    Idle = 1 << 4,
    /// RXNE interrupt enable
    RxNotEmpty = 1 << 5,
    /// Transmission complete interrupt enable
    TransmissionComplete = 1 << 6,
    /// TXE interrupt enable
    TxEmpty = 1 << 7,
    /// PE interrupt enable
    ParityError = 1 << 8,
}

/// UART/USART status flags
#[enumflags2::bitflags]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(u32)]
pub enum Flag {
    /// Parity error
    ParityError = 1 << 0,
    /// Framing error
    FramingError = 1 << 1,
    /// Noise detected flag
    Noise = 1 << 2,
    /// Overrun error
    Overrun = 1 << 3,
    /// IDLE line detected
    Idle = 1 << 4,
    /// Read data register not empty
    RxNotEmpty = 1 << 5,
    /// Transmission complete
    TransmissionComplete = 1 << 6,
    /// Transmit data register empty
    TxEmpty = 1 << 7,
    /// LIN break detection flag
    LinBreak = 1 << 8,
    /// CTS flag
    Cts = 1 << 9,
}

/// UART clearable flags
#[enumflags2::bitflags]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(u32)]
pub enum CFlag {
    /// Read data register not empty
    RxNotEmpty = 1 << 5,
    /// Transmission complete
    TransmissionComplete = 1 << 6,
    /// LIN break detection flag
    LinBreak = 1 << 8,
}

pub mod config;

pub use config::Config;

/// A filler type for when the Tx pin is unnecessary
pub use gpio::NoPin as NoTx;
/// A filler type for when the Rx pin is unnecessary
pub use gpio::NoPin as NoRx;

pub use gpio::alt::SerialAsync as CommonPins;

/// Trait for [`Rx`] interrupt handling.
pub trait RxISR {
    /// Return true if the line idle status is set
    fn is_idle(&self) -> bool;

    /// Return true if the rx register is not empty (and can be read)
    fn is_rx_not_empty(&self) -> bool;

    /// Clear idle line interrupt flag
    fn clear_idle_interrupt(&self);
}

/// Trait for [`Tx`] interrupt handling.
pub trait TxISR {
    /// Return true if the tx register is empty (and can accept data)
    fn is_tx_empty(&self) -> bool;
}

/// Trait for listening [`Rx`] interrupt events.
pub trait RxListen {
    /// Start listening for an rx not empty interrupt event
    ///
    /// Note, you will also have to enable the corresponding interrupt
    /// in the NVIC to start receiving events.
    fn listen(&mut self);

    /// Stop listening for the rx not empty interrupt event
    fn unlisten(&mut self);

    /// Start listening for a line idle interrupt event
    ///
    /// Note, you will also have to enable the corresponding interrupt
    /// in the NVIC to start receiving events.
    fn listen_idle(&mut self);

    /// Stop listening for the line idle interrupt event
    fn unlisten_idle(&mut self);
}

/// Trait for listening [`Tx`] interrupt event.
pub trait TxListen {
    /// Start listening for a tx empty interrupt event
    ///
    /// Note, you will also have to enable the corresponding interrupt
    /// in the NVIC to start receiving events.
    fn listen(&mut self);

    /// Stop listening for the tx empty interrupt event
    fn unlisten(&mut self);
}

/// Serial abstraction
pub struct Serial<USART: CommonPins, WORD = u8> {
    tx: Tx<USART, WORD>,
    rx: Rx<USART, WORD>,
}

/// Serial receiver containing RX pin
pub struct Rx<USART: CommonPins, WORD = u8> {
    _word: PhantomData<(USART, WORD)>,
    pin: USART::Rx<Floating>,
}

/// Serial transmitter containing TX pin
pub struct Tx<USART: CommonPins, WORD = u8> {
    _word: PhantomData<WORD>,
    usart: USART,
    pin: USART::Tx<PushPull>,
}

pub trait SerialExt: Sized + Instance {
    fn serial<WORD,RMP : Remap,TX: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Tx<PushPull>>,RX : crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Rx<Floating>>>(
        self,
        pins: (TX,RX),
        config: impl Into<config::Config>,
        clocks: &Clocks,
        afio: &mut crate::pac::Afio,
    ) -> Result<Serial<Self, WORD>, config::InvalidConfig>;

    fn tx<WORD,RMP : Remap,TX: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Tx<PushPull>>>(
        self,
        tx_pin: TX,
        config: impl Into<config::Config>,
        clocks: &Clocks,
        afio: &mut crate::pac::Afio,
    ) -> Result<Tx<Self, WORD>, config::InvalidConfig>
    where NoPin<Input>: Into<Self::Rx<Floating>>;

    fn rx<WORD,RMP : Remap,RX: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Rx<Floating>>>(
        self,
        rx_pin: RX,
        config: impl Into<config::Config>,
        clocks: &Clocks,
        afio: &mut crate::pac::Afio,
    ) -> Result<Rx<Self, WORD>, config::InvalidConfig>
    where NoPin<PushPull>: Into<Self::Tx<PushPull>>;
}

impl<USART: Instance, WORD> Serial<USART, WORD> {
    pub fn new(
        usart: USART,
        pins: (impl Into<USART::Tx<PushPull>>, impl Into<USART::Rx<Floating>>),
        config: impl Into<config::Config>,
        clocks: &Clocks,
        _afio: &mut crate::pac::Afio

    ) -> Result<Self, config::InvalidConfig>
    where
        <USART as Instance>::RegisterBlock: uart_impls::RegisterBlockImpl,
    {
        <USART as Instance>::RegisterBlock::new(usart, pins, config, clocks)
    }
}

impl<UART: CommonPins, WORD> Serial<UART, WORD> {
    pub fn split(self) -> (Tx<UART, WORD>, Rx<UART, WORD>) {
        (self.tx, self.rx)
    }

    #[allow(clippy::type_complexity)]
    pub fn release(self) -> (UART, (UART::Tx<PushPull>, UART::Rx<Floating>)) {
        (self.tx.usart, (self.tx.pin, self.rx.pin))
    }
}

macro_rules! halUsart {
    ($USART:ty, $USARTMOD:tt , $Serial:ident, $Rx:ident, $Tx:ident) => {
        pub type $Serial<WORD = u8> = Serial<$USART, WORD>;
        pub type $Tx<WORD = u8> = Tx<$USART, WORD>;
        pub type $Rx<WORD = u8> = Rx<$USART, WORD>;

        impl Instance for $USART {
            type RegisterBlock = crate::serial::uart_impls::RegisterBlockUsart;

            fn ptr() -> *const crate::serial::uart_impls::RegisterBlockUsart {
                <$USART>::ptr() as *const _
            }

            fn set_stopbits(&self, bits: config::StopBits) {
                use crate::pac::$USARTMOD::ctrl2::Stpb;
                use config::StopBits;

                self.ctrl2().modify(|_,w| {
                    w.stpb().variant(match bits {
                        StopBits::STOP0P5 => Stpb::Stop0p5,
                        StopBits::STOP1 => Stpb::Stop1,
                        StopBits::STOP1P5 => Stpb::Stop1p5,
                        StopBits::STOP2 => Stpb::Stop2,
                    })
                });
            }
        }
    };
}

macro_rules! halUart {
    ($USART:ty, $USARTMOD:tt , $Serial:ident, $Rx:ident, $Tx:ident) => {
        pub type $Serial<WORD = u8> = Serial<$USART, WORD>;
        pub type $Tx<WORD = u8> = Tx<$USART, WORD>;
        pub type $Rx<WORD = u8> = Rx<$USART, WORD>;

        impl Instance for $USART {
            type RegisterBlock = crate::serial::uart_impls::RegisterBlockUart;

            fn ptr() -> *const crate::serial::uart_impls::RegisterBlockUart {
                <$USART>::ptr() as *const _
            }

            fn set_stopbits(&self, bits: config::StopBits) {
                use crate::pac::$USARTMOD::ctrl2::Stpb;
                use config::StopBits;

                self.ctrl2().modify(|_,w| {
                    w.stpb().variant(match bits {
                        StopBits::STOP0P5 => Stpb::Stop0p5,
                        StopBits::STOP1 => Stpb::Stop1,
                        StopBits::STOP1P5 => Stpb::Stop1p5,
                        StopBits::STOP2 => Stpb::Stop2,
                    })
                });
            }
        }
    };
}


halUsart! { pac::Usart1, usart1, Serial1, Rx1, Tx1 }
halUsart! { pac::Usart2, usart1, Serial2, Rx2, Tx2 }
halUsart! { pac::Usart3, usart1, Serial3, Rx3, Tx3 }
halUart! { pac::Uart4, uart4, Serial4, Rx4, Tx4 }
halUart! { pac::Uart5, uart4, Serial5, Rx5, Tx5 }
halUart! { pac::Uart6, uart4, Serial6, Rx6, Tx6 }
halUart! { pac::Uart7, uart4, Serial7, Rx7, Tx7 }

impl<UART: CommonPins> Rx<UART, u8> {
    pub(crate) fn with_u16_data(self) -> Rx<UART, u16> {
        Rx::new(self.pin)
    }
}

impl<UART: CommonPins> Rx<UART, u16> {
    pub(crate) fn with_u8_data(self) -> Rx<UART, u8> {
        Rx::new(self.pin)
    }
}

impl<UART: CommonPins> Tx<UART, u8> {
    pub(crate) fn with_u16_data(self) -> Tx<UART, u16> {
        Tx::new(self.usart, self.pin)
    }
}

impl<UART: CommonPins> Tx<UART, u16> {
    pub(crate) fn with_u8_data(self) -> Tx<UART, u8> {
        Tx::new(self.usart, self.pin)
    }
}

impl<UART: CommonPins, WORD> Rx<UART, WORD> {
    pub(crate) fn new(pin: UART::Rx<Floating>) -> Self {
        Self {
            _word: PhantomData,
            pin,
        }
    }

    pub fn join(self, tx: Tx<UART, WORD>) -> Serial<UART, WORD> {
        Serial { tx, rx: self }
    }
}

impl<UART: CommonPins, WORD> Tx<UART, WORD> {
    pub(crate) fn new(usart: UART, pin: UART::Tx<PushPull>) -> Self {
        Self {
            _word: PhantomData,
            usart,
            pin,
        }
    }

    pub fn join(self, rx: Rx<UART, WORD>) -> Serial<UART, WORD> {
        Serial { tx: self, rx }
    }
}

impl<UART: Instance, WORD> AsRef<Tx<UART, WORD>> for Serial<UART, WORD> {
    #[inline(always)]
    fn as_ref(&self) -> &Tx<UART, WORD> {
        &self.tx
    }
}

impl<UART: Instance, WORD> AsRef<Rx<UART, WORD>> for Serial<UART, WORD> {
    #[inline(always)]
    fn as_ref(&self) -> &Rx<UART, WORD> {
        &self.rx
    }
}

impl<UART: Instance, WORD> AsMut<Tx<UART, WORD>> for Serial<UART, WORD> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut Tx<UART, WORD> {
        &mut self.tx
    }
}

impl<UART: Instance, WORD> AsMut<Rx<UART, WORD>> for Serial<UART, WORD> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut Rx<UART, WORD> {
        &mut self.rx
    }
}

impl<UART: Instance> Serial<UART, u8> {
    /// Converts this Serial into a version that can read and write `u16` values instead of `u8`s
    ///
    /// This can be used with a word length of 9 bits.
    pub fn with_u16_data(self) -> Serial<UART, u16> {
        Serial {
            tx: self.tx.with_u16_data(),
            rx: self.rx.with_u16_data(),
        }
    }
}

impl<UART: Instance> Serial<UART, u16> {
    /// Converts this Serial into a version that can read and write `u8` values instead of `u16`s
    ///
    /// This can be used with a word length of 8 bits.
    pub fn with_u8_data(self) -> Serial<UART, u8> {
        Serial {
            tx: self.tx.with_u8_data(),
            rx: self.rx.with_u8_data(),
        }
    }
}

use crate::dma::{DMAMode, Receive, TransferPayload, Transmit};

pub trait SerialDma<PER,MODE : DMAMode, DMACH : crate::dma::CompatibleChannel<PER,MODE> + crate::dma::DMAChannel> {
    type DmaType;
    fn with_dma(self, channel: DMACH) -> Self::DmaType;
}
macro_rules! serialdma {
    ($(
        $USARTX:ident: (
            $rxdma:tt,
            $txdma:tt,
        ),
    )+) => {
        $(

            
            pub type $rxdma<RXCH> = crate::dma::RxDma<Rx<$USARTX>, RXCH>;
            pub type $txdma<TXCH> = crate::dma::TxDma<Tx<$USARTX>, TXCH>;

            impl<RXCH: crate::dma::DMAChannel> Receive for $rxdma<RXCH> {
                type RxChannel = RXCH;
                type TransmittedWord = u8;
            }

            impl<TXCH: crate::dma::DMAChannel> Transmit for $txdma<TXCH> {
                type TxChannel = TXCH;
                type ReceivedWord = u8;
            }

            impl<RXCH: crate::dma::DMAChannel> TransferPayload for $rxdma<RXCH> {
                fn start(&mut self) {
                    self.channel.start();
                }
                fn stop(&mut self) {
                    self.channel.stop();
                }
            }

            impl<TXCH : crate::dma::DMAChannel> TransferPayload for $txdma<TXCH> {
                fn start(&mut self) {
                    self.channel.start();
                }
                fn stop(&mut self) {
                    self.channel.stop();
                }
            }

            impl<RXCH : crate::dma::DMAChannel + crate::dma::CompatibleChannel<$USARTX, crate::dma::R>> SerialDma<$USARTX,crate::dma::R, RXCH> for Rx<$USARTX> {
                type DmaType = $rxdma<RXCH>;
                fn with_dma(self, mut channel: RXCH) -> Self::DmaType {
                    unsafe { (*$USARTX::ptr()).ctrl3().modify(|_, w| w.dmarxen().set_bit()); }
                    channel.configure_channel();
                    crate::dma::RxDma {
                        payload: self,
                        channel,
                    }
                }
            }

            impl<TXCH : crate::dma::DMAChannel + crate::dma::CompatibleChannel<$USARTX, crate::dma::W>> SerialDma<$USARTX,crate::dma::W, TXCH> for Tx<$USARTX> {
                type DmaType = $txdma<TXCH> ;
                fn with_dma(self, mut channel: TXCH) -> Self::DmaType {
                    unsafe { (*$USARTX::ptr()).ctrl3().modify(|_, w| w.dmarxen().set_bit()); }
                    channel.configure_channel();
                    crate::dma::TxDma {
                        payload: self,
                        channel,
                    }
                }
            }

            impl<T : crate::dma::DMAChannel> $rxdma<T> {
                pub fn release(mut self) -> (Rx<$USARTX>, T) {
                    self.stop();
                    unsafe { (*$USARTX::ptr()).ctrl3().modify(|_, w| w.dmarxen().clear_bit()); }
                    let crate::dma::RxDma {payload, channel} = self;
                    (
                        payload,
                        channel
                    )
                }
            }

            impl<T : crate::dma::DMAChannel> $txdma<T> {
                pub fn release(mut self) -> (Tx<$USARTX>, T) {
                    self.stop();
                    unsafe { (*$USARTX::ptr()).ctrl3().modify(|_, w| w.dmatxen().clear_bit()); }
                    let crate::dma::TxDma {payload, channel} = self;
                    (
                        payload,
                        channel,
                    )
                }
            }

            impl<B,RXCH : crate::dma::DMAChannel> crate::dma::CircReadDma<B, u8> for $rxdma<RXCH>
            where
                &'static mut [B; 2]: embedded_dma::WriteBuffer<Word = u8>,
                B: 'static,
            {
                fn circ_read(mut self, mut buffer: &'static mut [B; 2]) -> crate::dma::CircBuffer<B, Self> {
                    // NOTE(unsafe) We own the buffer now and we won't call other `&mut` on it
                    // until the end of the transfer.
                    let (ptr, len) = unsafe { buffer.write_buffer() };
                    self.channel.set_peripheral_address(unsafe{ (*$USARTX::ptr()).dat().as_ptr() as u32 }, false);
                    self.channel.set_memory_address(ptr as u32, true);
                    self.channel.set_transfer_length(len);

                    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);

                    self.channel.st().chcfg().modify(|_, w| { w
                        .mem2mem() .clear_bit()
                        .priolvl() .medium()
                        .msize()   .bits8()
                        .psize()   .bits8()
                        .circ()    .set_bit()
                        .dir()     .clear_bit()
                    });

                    self.start();

                    crate::dma::CircBuffer::new(buffer, self)
                }
            }

            impl<B,RXCH : crate::dma::DMAChannel> crate::dma::ReadDma<B, u8> for $rxdma<RXCH>
            where
                B: embedded_dma::WriteBuffer<Word = u8>,
            {
                fn read(mut self, mut buffer: B) -> crate::dma::Transfer<crate::dma::W, B, Self> {
                    // NOTE(unsafe) We own the buffer now and we won't call other `&mut` on it
                    // until the end of the transfer.
                    let (ptr, len) = unsafe { buffer.write_buffer() };
                    self.channel.set_peripheral_address(unsafe{ (*$USARTX::ptr()).dat().as_ptr() as u32 }, false);
                    self.channel.set_memory_address(ptr as u32, true);
                    self.channel.set_transfer_length(len);

                    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
                    self.channel.st().chcfg().modify(|_, w| { w
                        .mem2mem() .clear_bit()
                        .priolvl() .medium()
                        .msize()   .bits8()
                        .psize()   .bits8()
                        .circ()    .clear_bit()
                        .dir()     .clear_bit()
                    });
                    self.start();

                    crate::dma::Transfer::w(buffer, self)
                }
            }

            impl<B,TXCH : crate::dma::DMAChannel> crate::dma::WriteDma<B, u8> for $txdma<TXCH>
            where
                B: embedded_dma::ReadBuffer<Word = u8>,
            {
                fn write(mut self, buffer: B) -> crate::dma::Transfer<crate::dma::R, B, Self> {
                    // NOTE(unsafe) We own the buffer now and we won't call other `&mut` on it
                    // until the end of the transfer.
                    let (ptr, len) = unsafe { buffer.read_buffer() };

                    self.channel.set_peripheral_address(unsafe{ (*$USARTX::ptr()).dat().as_ptr() as u32 }, false);

                    self.channel.set_memory_address(ptr as u32, true);
                    self.channel.set_transfer_length(len);

                    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);

                    self.channel.st().chcfg().modify(|_, w| { w
                        .mem2mem() .clear_bit()
                        .priolvl() .medium()
                        .msize()   .bits8()
                        .psize()   .bits8()
                        .circ()    .clear_bit()
                        .dir()     .set_bit()
                    });
                    self.start();

                    crate::dma::Transfer::r(buffer, self)
                }
            }
        )+
    }
}
use crate::pac::{Usart1,Usart2,Usart3,Uart4,Uart5,Uart6,Uart7};
serialdma! {
    Usart1: (
        RxDma1,
        TxDma1,
    ),
    Usart2: (
        RxDma2,
        TxDma2,
    ),
    Usart3: (
        RxDma3,
        TxDma3,
    ),
    Uart4: (
        RxDma4,
        TxDma4,
    ),
    Uart5: (
        RxDma5,
        TxDma5,
    ),
    Uart6: (
        RxDma6,
        TxDma6,
    ),
    Uart7: (
        RxDma7,
        TxDma7,
    ),
}