use core::marker::{ConstParamTy, PhantomData};
use core::ops::{Deref, DerefMut};
use core::sync::atomic::Ordering;
use core::sync::atomic;
use crate::dma::*;
use crate::gpio::alt::altmap::Remap;
use crate::gpio::{self, NoPin};
use crate::pac;
use embedded_dma::WriteBuffer;
use embedded_dma::ReadBuffer;
/// Clock polarity
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    /// Clock signal low when idle
    IdleLow,
    /// Clock signal high when idle
    IdleHigh,
}

/// Clock phase
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Data in "captured" on the first clock transition
    CaptureOnFirstTransition,
    /// Data in "captured" on the second clock transition
    CaptureOnSecondTransition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ConstParamTy)]
pub enum TransferMode {
    TransferModeNormal,
    TransferModeBidirectional,
    TransferModeRecieveOnly,
    TransferModeTransmitOnly
}

/// SPI mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
    /// Clock polarity
    pub polarity: Polarity,
    /// Clock phase
    pub phase: Phase,
}

mod hal_02;
mod hal_1;

use crate::pac::spi1;
use crate::rcc;

use crate::rcc::Clocks;
use enumflags2::BitFlags;
use fugit::HertzU32 as Hertz;

/// SPI error
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[non_exhaustive]
pub enum Error {
    /// Overrun occurred
    Overrun,
    /// Mode fault occurred
    ModeFault,
    /// CRC error
    Crc,
}

/// A filler type for when the SCK pin is unnecessary
pub type NoSck = NoPin;
/// A filler type for when the Miso pin is unnecessary
pub type NoMiso = NoPin;
/// A filler type for when the Mosi pin is unnecessary
pub type NoMosi = NoPin;

/// SPI interrupt events
#[enumflags2::bitflags]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(u32)]
pub enum Event {
    /// An error occurred.
    ///
    /// This bit controls the generation of an interrupt
    /// when an error condition occurs
    /// (OVR, CRCERR, MODF, FRE in SPI mode,
    /// and UDR, OVR, FRE in I2S mode)
    Error = 1 << 5,
    /// New data has been received
    ///
    /// RX buffer not empty interrupt enable
    RxNotEmpty = 1 << 6,
    /// Data can be sent
    ///
    /// Tx buffer empty interrupt enable
    TxEmpty = 1 << 7,
}

/// SPI status flags
#[enumflags2::bitflags]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(u32)]
pub enum Flag {
    /// Receive buffer not empty
    RxNotEmpty = 1 << 0,
    /// Transmit buffer empty
    TxEmpty = 1 << 1,
    /// CRC error flag
    CrcError = 1 << 4,
    /// Mode fault
    ModeFault = 1 << 5,
    /// Overrun flag
    Overrun = 1 << 6,
    /// Busy flag
    Busy = 1 << 7,
    /// Frame Error
    FrameError = 1 << 8,
}

/// SPI clearable flags
#[enumflags2::bitflags]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[repr(u32)]
pub enum CFlag {
    /// CRC error flag
    CrcError = 1 << 4,
}

pub trait FrameSize: Copy + Default {
    const DFF: bool;
}

impl FrameSize for u8 {
    const DFF: bool = false;
}

impl FrameSize for u16 {
    const DFF: bool = true;
}

/// The bit format to send the data in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitFormat {
    /// Least significant bit first
    LsbFirst,
    /// Most significant bit first
    MsbFirst,
}

#[derive(Debug)]
pub struct Inner<SPI: Instance> {
    spi: SPI,
}

/// Spi in Master mode
#[derive(Debug)]
pub struct Spi<SPI: Instance, const XFER_MODE : TransferMode = {TransferMode::TransferModeNormal}, W = u8> {
    inner: Inner<SPI>,
    pins: (SPI::Sck, SPI::Miso, SPI::Mosi),
    _operation: PhantomData<W>,
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W> Deref for Spi<SPI, XFER_MODE, W> {
    type Target = Inner<SPI>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W> DerefMut for Spi<SPI, XFER_MODE, W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Spi in Slave mode
#[derive(Debug)]
pub struct SpiSlave<SPI: Instance, const XFER_MODE : TransferMode = {TransferMode::TransferModeNormal}, W = u8> {
    inner: Inner<SPI>,
    pins: (SPI::Sck, SPI::Miso, SPI::Mosi, Option<SPI::Nss>),
    _operation: PhantomData<W>,
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W> Deref for SpiSlave<SPI, XFER_MODE, W> {
    type Target = Inner<SPI>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W> DerefMut for SpiSlave<SPI, XFER_MODE, W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// Implemented by all SPI instances
pub trait Instance:
    crate::Sealed
    + Deref<Target = spi1::RegisterBlock>
    + rcc::Enable
    + rcc::Reset
    + rcc::BusClock
    + gpio::alt::SpiCommon
{
    #[doc(hidden)]
    fn ptr() -> *const spi1::RegisterBlock;
}

// Implemented by all SPI instances
macro_rules! spi {
    ($SPI:ty: $Spi:ident, $SpiSlave:ident) => {
        pub type $Spi<const XFER_MODE : TransferMode = {TransferMode::TransferModeNormal}, W = u8> = Spi<$SPI, XFER_MODE, W>;
        pub type $SpiSlave<const XFER_MODE : TransferMode = {TransferMode::TransferModeNormal}, W = u8> = SpiSlave<$SPI, XFER_MODE, W>;

        impl Instance for $SPI {
            fn ptr() -> *const spi1::RegisterBlock {
                <$SPI>::ptr() as *const _
            }
        }
    };
}

spi! { pac::Spi1: Spi1, SpiSlave1 }
spi! { pac::Spi2: Spi2, SpiSlave2 }
spi! { pac::Spi3: Spi3, SpiSlave3 }


pub trait SpiExt: Sized + Instance {
    fn spi<RMP : Remap,
    SCK: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Sck>,
    MISO: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Miso>,
    MOSI: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Mosi>>(
        self,
        pins: (SCK,MISO,MOSI),
        mode: impl Into<Mode>,
        freq: Hertz,
        clocks: &Clocks,
        afio: &mut pac::Afio,
    ) -> Spi<Self, {TransferMode::TransferModeNormal}, u8>;

    fn spi_bidi<RMP : Remap,
    SCK: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Sck>,
    MOSI: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Mosi>>(
        self,
        pins: (SCK,MOSI),
        mode: impl Into<Mode>,
        freq: Hertz,
        clocks: &Clocks,
        afio: &mut pac::Afio,
    ) -> Spi<Self, {TransferMode::TransferModeBidirectional}, u8>
    where
        NoPin: Into<Self::Miso>;

    fn spi_rxonly<RMP : Remap,
    SCK: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Sck>,
    MISO: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Miso>>(
        self,
        pins: (SCK,MISO),
        mode: impl Into<Mode>,
        freq: Hertz,
        clocks: &Clocks,
        afio: &mut pac::Afio,
    ) -> Spi<Self, {TransferMode::TransferModeRecieveOnly}, u8>
    where
        NoPin: Into<Self::Mosi>;

    fn spi_slave<RMP : Remap,
    SCK: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Sck>,
    MISO: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Miso>,
    MOSI: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Mosi>,
    NSS: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Nss>>(
        self,
        pins: (
            SCK,
            MISO,
            MOSI,
            Option<NSS>
        ),
        mode: impl Into<Mode>,
    ) -> SpiSlave<Self, {TransferMode::TransferModeNormal}, u8>;

    fn spi_bidi_slave(
        self,
        pins: (
            impl Into<Self::Sck>,
            impl Into<Self::Miso>,
            Option<Self::Nss>,
        ),
        mode: impl Into<Mode>,
    ) -> SpiSlave<Self, {TransferMode::TransferModeBidirectional}, u8>
    where
        NoPin: Into<Self::Mosi>;
}

impl<SPI: Instance> SpiExt for SPI {
    /// Enables the SPI clock, resets the peripheral, sets `Alternate` mode for `pins` and initialize the peripheral as SPI Master Normal mode.
    ///
    /// # Note
    /// Depending on `freq` you may need to set GPIO speed for `pins` (the `Speed::Low` is default for GPIO) before create `Spi` instance.
    /// Otherwise it may lead to the 'wrong last bit in every received byte' problem.
    fn spi<RMP : Remap,SCK: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Sck>,
    MISO: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Miso>,
    MOSI: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Mosi>>(
        self,
        pins: (SCK,MISO,MOSI),
        mode: impl Into<Mode>,
        freq: Hertz,
        clocks: &Clocks,
        afio: &mut pac::Afio,
    ) -> Spi<Self, {TransferMode::TransferModeNormal}, u8> {
        RMP::remap(afio);
        Spi::new(self, pins, mode, freq, clocks)
    }
    /// Enables the SPI clock, resets the peripheral, sets `Alternate` mode for `pins` and initialize the peripheral as SPI Master XFER_MODE mode.
    ///
    /// # Note
    /// Depending on `freq` you may need to set GPIO speed for `pins` (the `Speed::Low` is default for GPIO) before create `Spi` instance.
    /// Otherwise it may lead to the 'wrong last bit in every received byte' problem.
    fn spi_bidi<RMP : Remap,
    SCK: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Sck>,
    MOSI: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Mosi>>(
        self,
        pins: (SCK,MOSI),
        mode: impl Into<Mode>,
        freq: Hertz,
        clocks: &Clocks,
        afio: &mut pac::Afio,
    ) -> Spi<Self, {TransferMode::TransferModeBidirectional}, u8>
    where
        NoPin: Into<Self::Miso>,
    {
        RMP::remap(afio);
        Spi::new_bidi(self, pins, mode, freq, clocks)
    }

        /// Enables the SPI clock, resets the peripheral, sets `Alternate` mode for `pins` and initialize the peripheral as SPI Master XFER_MODE mode.
    ///
    /// # Note
    /// Depending on `freq` you may need to set GPIO speed for `pins` (the `Speed::Low` is default for GPIO) before create `Spi` instance.
    /// Otherwise it may lead to the 'wrong last bit in every received byte' problem.
    fn spi_rxonly<RMP : Remap,
    SCK: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Sck>,
    MISO: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Miso>>(
        self,
        pins: (SCK,MISO),
        mode: impl Into<Mode>,
        freq: Hertz,
        clocks: &Clocks,
        afio: &mut pac::Afio,

    ) -> Spi<Self, {TransferMode::TransferModeRecieveOnly}, u8>
    where
        NoPin: Into<Self::Mosi>,
    {
        RMP::remap(afio);
        Spi::new_rxonly(self, pins, mode, freq, clocks)
    }
    /// Enables the SPI clock, resets the peripheral, sets `Alternate` mode for `pins` and initialize the peripheral as SPI Slave Normal mode.
    ///
    /// # Note
    /// Depending on `freq` you may need to set GPIO speed for `pins` (the `Speed::Low` is default for GPIO) before create `Spi` instance.
    /// Otherwise it may lead to the 'wrong last bit in every received byte' problem.
    fn spi_slave<RMP : Remap,
        SCK: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Sck>,
        MISO: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Miso>,
        MOSI: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Mosi>,
        NSS: crate::gpio::alt::altmap::RemapIO<Self,RMP> + Into<Self::Nss>>(
            self,
            pins: (
                SCK,
                MISO,
                MOSI,
                Option<NSS>
            ),
        mode: impl Into<Mode>,
    ) -> SpiSlave<Self, {TransferMode::TransferModeNormal}, u8> {
        SpiSlave::new(self, pins, mode)
    }
    /// Enables the SPI clock, resets the peripheral, sets `Alternate` mode for `pins` and initialize the peripheral as SPI Slave XFER_MODE mode.
    ///
    /// # Note
    /// Depending on `freq` you may need to set GPIO speed for `pins` (the `Speed::Low` is default for GPIO) before create `Spi` instance.
    /// Otherwise it may lead to the 'wrong last bit in every received byte' problem.
    fn spi_bidi_slave(
        self,
        pins: (
            impl Into<Self::Sck>,
            impl Into<Self::Miso>,
            Option<Self::Nss>,
        ),
        mode: impl Into<Mode>,
    ) -> SpiSlave<Self, {TransferMode::TransferModeBidirectional}, u8>
    where
        NoPin: Into<Self::Mosi>,
    {
        SpiSlave::new_bidi(self, pins, mode)
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W: FrameSize> Spi<SPI, XFER_MODE, W> {
    pub fn init(self) -> Self {
        self.spi.ctrl1().modify(|_, w| {
            // bidimode: 2-line or 1-line unidirectional
            w.bidirmode().bit(XFER_MODE == TransferMode::TransferModeBidirectional);
            w.bidiroen().bit(XFER_MODE == TransferMode::TransferModeBidirectional);
            // data frame size
            w.datff().bit(W::DFF);
            // spe: enable the SPI bus
            w.spien().bit(XFER_MODE != TransferMode::TransferModeRecieveOnly)
        });

        self
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W: FrameSize> SpiSlave<SPI, XFER_MODE, W> {
    pub fn init(self) -> Self {
        self.spi.ctrl1().modify(|_, w| {
            // bidimode: 2-line or 1-line unidirectional
            w.bidirmode().bit(XFER_MODE == TransferMode::TransferModeBidirectional);
            w.bidiroen().bit(XFER_MODE == TransferMode::TransferModeBidirectional);
            // data frame size
            w.datff().bit(W::DFF);
            // spe: enable the SPI bus
            w.spien().set_bit()
        });

        self
    }
}

impl<SPI: Instance, W: FrameSize> Spi<SPI, {TransferMode::TransferModeNormal}, W> {
    pub fn to_bidi_transfer_mode(self) -> Spi<SPI, {TransferMode::TransferModeBidirectional}, W> {
        self.into_mode()
    }
}

impl<SPI: Instance, W: FrameSize> Spi<SPI, {TransferMode::TransferModeBidirectional}, W> {
    pub fn to_normal_transfer_mode(self) -> Spi<SPI, {TransferMode::TransferModeNormal}, W> {
        self.into_mode()
    }
}

impl<SPI: Instance, W: FrameSize> SpiSlave<SPI, {TransferMode::TransferModeNormal}, W> {
    pub fn to_bidi_transfer_mode(self) -> SpiSlave<SPI, {TransferMode::TransferModeBidirectional}, W> {
        self.into_mode()
    }
}

impl<SPI: Instance, W: FrameSize> SpiSlave<SPI, {TransferMode::TransferModeBidirectional}, W> {
    pub fn to_normal_transfer_mode(self) -> SpiSlave<SPI, {TransferMode::TransferModeNormal}, W> {
        self.into_mode()
    }
}

impl<SPI, const XFER_MODE : TransferMode> Spi<SPI, XFER_MODE, u8>
where
    SPI: Instance,
{
    /// Converts from 8bit dataframe to 16bit.
    pub fn frame_size_16bit(self) -> Spi<SPI, XFER_MODE, u16> {
        self.into_mode()
    }
}

impl<SPI, const XFER_MODE : TransferMode> Spi<SPI, XFER_MODE, u16>
where
    SPI: Instance,
{
    /// Converts from 16bit dataframe to 8bit.
    pub fn frame_size_8bit(self) -> Spi<SPI, XFER_MODE, u8> {
        self.into_mode()
    }
}

impl<SPI, const XFER_MODE : TransferMode> SpiSlave<SPI, XFER_MODE, u8>
where
    SPI: Instance,
{
    /// Converts from 8bit dataframe to 16bit.
    pub fn frame_size_16bit(self) -> SpiSlave<SPI, XFER_MODE, u16> {
        self.into_mode()
    }
}

impl<SPI, const XFER_MODE : TransferMode> SpiSlave<SPI, XFER_MODE, u16>
where
    SPI: Instance,
{
    /// Converts from 16bit dataframe to 8bit.
    pub fn frame_size_8bit(self) -> SpiSlave<SPI, XFER_MODE, u8> {
        self.into_mode()
    }
}

impl<SPI: Instance> Spi<SPI, {TransferMode::TransferModeNormal}, u8> {
    /// Enables the SPI clock, resets the peripheral, sets `Alternate` mode for `pins` and initialize the peripheral as SPI Master Normal mode.
    ///
    /// # Note
    /// Depending on `freq` you may need to set GPIO speed for `pins` (the `Speed::Low` is default for GPIO) before create `Spi` instance.
    /// Otherwise it may lead to the 'wrong last bit in every received byte' problem.
    pub fn new<RMP : Remap,
    SCK: crate::gpio::alt::altmap::RemapIO<SPI,RMP> + Into<SPI::Sck>,
    MISO: crate::gpio::alt::altmap::RemapIO<SPI,RMP> + Into<SPI::Miso>,
    MOSI: crate::gpio::alt::altmap::RemapIO<SPI,RMP> + Into<SPI::Mosi>>(
        spi: SPI,
        pins: (SCK,MISO,MOSI),
        mode: impl Into<Mode>,
        freq: Hertz,
        clocks: &Clocks,
    ) -> Self {
        unsafe {
            SPI::enable_unchecked();
            SPI::reset_unchecked();
        }

        let pins = (pins.0.into(), pins.1.into(), pins.2.into());

        Self::_new(spi, pins)
            .pre_init(mode.into(), freq, SPI::clock(clocks))
            .init()
    }
}

impl<SPI: Instance> Spi<SPI, {TransferMode::TransferModeRecieveOnly}, u8> {
    /// Enables the SPI clock, resets the peripheral, sets `Alternate` mode for `pins` and initialize the peripheral as SPI Master XFER_MODE mode.
    ///
    /// # Note
    /// Depending on `freq` you may need to set GPIO speed for `pins` (the `Speed::Low` is default for GPIO) before create `Spi` instance.
    /// Otherwise it may lead to the 'wrong last bit in every received byte' problem.
    pub fn new_rxonly(
        spi: SPI,
        pins: (impl Into<SPI::Sck>, impl Into<SPI::Miso>),
        mode: impl Into<Mode>,
        freq: Hertz,
        clocks: &Clocks,
    ) -> Self
    where
        NoPin: Into<SPI::Mosi>,
    {
        unsafe {
            SPI::enable_unchecked();
            SPI::reset_unchecked();
        }

        let pins = (pins.0.into(),  pins.1.into(),NoPin::new().into());
        
        Self::_new(spi, pins)
            .pre_init(mode.into(), freq, SPI::clock(clocks))
            .init()
    }

}

impl<SPI: Instance> Spi<SPI, {TransferMode::TransferModeBidirectional}, u8> {
    /// Enables the SPI clock, resets the peripheral, sets `Alternate` mode for `pins` and initialize the peripheral as SPI Master XFER_MODE mode.
    ///
    /// # Note
    /// Depending on `freq` you may need to set GPIO speed for `pins` (the `Speed::Low` is default for GPIO) before create `Spi` instance.
    /// Otherwise it may lead to the 'wrong last bit in every received byte' problem.
    pub fn new_bidi(
        spi: SPI,
        pins: (impl Into<SPI::Sck>, impl Into<SPI::Mosi>),
        mode: impl Into<Mode>,
        freq: Hertz,
        clocks: &Clocks,
    ) -> Self
    where
        NoPin: Into<SPI::Miso>,
    {
        unsafe {
            SPI::enable_unchecked();
            SPI::reset_unchecked();
        }

        let pins = (pins.0.into(), NoPin::new().into(), pins.1.into());

        Self::_new(spi, pins)
            .pre_init(mode.into(), freq, SPI::clock(clocks))
            .init()
    }
}

impl<SPI: Instance> SpiSlave<SPI, {TransferMode::TransferModeNormal}, u8> {
    /// Enables the SPI clock, resets the peripheral, sets `Alternate` mode for `pins` and initialize the peripheral as SPI Slave Normal mode.
    ///
    /// # Note
    /// Depending on `freq` you may need to set GPIO speed for `pins` (the `Speed::Low` is default for GPIO) before create `Spi` instance.
    /// Otherwise it may lead to the 'wrong last bit in every received byte' problem.
    pub fn new<RMP : Remap,
    SCK: crate::gpio::alt::altmap::RemapIO<SPI,RMP> + Into<SPI::Sck>,
    MISO: crate::gpio::alt::altmap::RemapIO<SPI,RMP> + Into<SPI::Miso>,
    MOSI: crate::gpio::alt::altmap::RemapIO<SPI,RMP> + Into<SPI::Mosi>,
    NSS: crate::gpio::alt::altmap::RemapIO<SPI,RMP> + Into<SPI::Nss>>(
        spi: SPI,
        pins: (
            SCK,
            MISO,
            MOSI,
            Option<NSS>
        ),
        mode: impl Into<Mode>,
    ) -> Self {
        unsafe {
            SPI::enable_unchecked();
            SPI::reset_unchecked();
        }

        let pins = (pins.0.into(), pins.1.into(), pins.2.into(), pins.3.map(|v| v.into()));

        Self::_new(spi, pins).pre_init(mode.into()).init()
    }
}

impl<SPI: Instance> SpiSlave<SPI, {TransferMode::TransferModeBidirectional}, u8> {
    /// Enables the SPI clock, resets the peripheral, sets `Alternate` mode for `pins` and initialize the peripheral as SPI Slave XFER_MODE mode.
    ///
    /// # Note
    /// Depending on `freq` you may need to set GPIO speed for `pins` (the `Speed::Low` is default for GPIO) before create `Spi` instance.
    /// Otherwise it may lead to the 'wrong last bit in every received byte' problem.
    pub fn new_bidi(
        spi: SPI,
        pins: (impl Into<SPI::Sck>, impl Into<SPI::Miso>, Option<SPI::Nss>),
        mode: impl Into<Mode>,
    ) -> Self
    where
        NoPin: Into<SPI::Mosi>,
    {
        unsafe {
            SPI::enable_unchecked();
            SPI::reset_unchecked();
        }

        let pins = (pins.0.into(), pins.1.into(), NoPin::new().into(), pins.2);

        Self::_new(spi, pins).pre_init(mode.into()).init()
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W> Spi<SPI, XFER_MODE, W> {
    #[allow(clippy::type_complexity)]
    pub fn release(self) -> (SPI, (SPI::Sck, SPI::Miso, SPI::Mosi)) {
        (self.inner.spi, self.pins)
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W> SpiSlave<SPI, XFER_MODE, W> {
    #[allow(clippy::type_complexity)]
    pub fn release(self) -> (SPI, (SPI::Sck, SPI::Miso, SPI::Mosi, Option<SPI::Nss>)) {
        (self.inner.spi, self.pins)
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W> Spi<SPI, XFER_MODE, W> {
    fn _new(spi: SPI, pins: (SPI::Sck, SPI::Miso, SPI::Mosi)) -> Self {
        Self {
            inner: Inner::new(spi),
            pins,
            _operation: PhantomData,
        }
    }

    /// Convert the spi to another mode.
    fn into_mode<const XFER_MODE2: TransferMode, W2: FrameSize>(self) -> Spi<SPI, XFER_MODE2, W2> {
        let mut spi = Spi::_new(self.inner.spi, self.pins);
        spi.enable(false);
        spi.init()
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W> SpiSlave<SPI, XFER_MODE, W> {
    fn _new(spi: SPI, pins: (SPI::Sck, SPI::Miso, SPI::Mosi, Option<SPI::Nss>)) -> Self {
        Self {
            inner: Inner::new(spi),
            pins,
            _operation: PhantomData,
        }
    }

    /// Convert the spi to another mode.
    fn into_mode<const XFER_MODE2: TransferMode, W2: FrameSize>(self) -> SpiSlave<SPI, XFER_MODE2, W2> {
        let mut spi = SpiSlave::_new(self.inner.spi, self.pins);
        spi.enable(false);
        spi.init()
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W> Spi<SPI, XFER_MODE, W> {
    /// Pre initializing the SPI bus.
    fn pre_init(self, mode: Mode, freq: Hertz, clock: Hertz) -> Self {
        // disable SS output
        self.spi.ctrl2().modify(|_,w| w.ssoen().clear_bit());

        let br = match clock.raw() / freq.raw() {
            0 => unreachable!(),
            1..=2 => 0b000,
            3..=5 => 0b001,
            6..=11 => 0b010,
            12..=23 => 0b011,
            24..=47 => 0b100,
            48..=95 => 0b101,
            96..=191 => 0b110,
            _ => 0b111,
        };

        self.spi.ctrl1().modify(|_,w| {
            w.clkpha().bit(mode.phase == Phase::CaptureOnSecondTransition);
            w.clkpol().bit(mode.polarity == Polarity::IdleHigh);
            // mstr: master configuration
            w.msel().set_bit();
            unsafe { w.br().bits(br) };
            // lsbfirst: MSB first
            w.lsbff().clear_bit();
            // ssm: enable software slave management (NSS pin free for other uses)
            w.ssmen().set_bit();
            // ssi: set nss high
            w.ssel().set_bit();
            w.ronly().bit(XFER_MODE == TransferMode::TransferModeRecieveOnly);
            // dff: 8 bit frames
            w.datff().clear_bit()
        });

        self
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W> SpiSlave<SPI, XFER_MODE, W> {
    /// Pre initializing the SPI bus.
    fn pre_init(self, mode: Mode) -> Self {
        self.spi.ctrl1().modify(|_,w| {
            w.clkpha().bit(mode.phase == Phase::CaptureOnSecondTransition);
            w.clkpol().bit(mode.polarity == Polarity::IdleHigh);
            // mstr: slave configuration
            w.msel().clear_bit();
            unsafe { w.br().bits(0) };
            // lsbfirst: MSB first
            w.lsbff().clear_bit();
            // ssm: enable software slave management (NSS pin free for other uses)
            w.ssmen().bit(self.pins.3.is_none());
            // ssi: set nss high = master mode
            w.ssel().set_bit();
            w.ronly().clear_bit();
            // dff: 8 bit frames
            w.datff().clear_bit()
        });

        self
    }

    /// Set the slave select bit programmatically.
    #[inline]
    pub fn set_internal_nss(&mut self, value: bool) {
        self.spi.ctrl1().modify(|_, w| w.ssel().bit(value));
    }
}

impl<SPI: Instance> Inner<SPI> {
    fn new(spi: SPI) -> Self {
        Self { spi }
    }

    /// Enable/disable spi
    pub fn enable(&mut self, enable: bool) {
        self.spi.ctrl1().modify(|_, w| {
            // spe: enable the SPI bus
            w.spien().bit(enable)
        });
    }

    /// Select which frame format is used for data transfers
    pub fn bit_format(&mut self, format: BitFormat) {
        self.spi
            .ctrl1()
            .modify(|_, w| w.lsbff().bit(format == BitFormat::LsbFirst));
    }

    /// Return `true` if the TXE flag is set, i.e. new data to transmit
    /// can be written to the SPI.
    #[inline]
    pub fn is_tx_empty(&self) -> bool {
        self.spi.sts().read().te().bit_is_set()
    }

    /// Return `true` if the RXNE flag is set, i.e. new data has been received
    /// and can be read from the SPI.
    #[inline]
    pub fn is_rx_not_empty(&self) -> bool {
        self.spi.sts().read().rne().bit_is_set()
    }

    /// Return `true` if the MODF flag is set, i.e. the SPI has experienced a
    /// Master Mode Fault. (see chapter 28.3.10 of the STM32F4 Reference Manual)
    #[inline]
    pub fn is_modf(&self) -> bool {
        self.spi.sts().read().moderr().bit_is_set()
    }

    /// Returns true if the transfer is in progress
    #[inline]
    pub fn is_busy(&self) -> bool {
        self.spi.sts().read().busy().bit_is_set()
    }

    /// Return `true` if the OVR flag is set, i.e. new data has been received
    /// while the receive data register was already filled.
    #[inline]
    pub fn is_overrun(&self) -> bool {
        self.spi.sts().read().over().bit_is_set()
    }

    #[inline]
    fn bidi_output(&mut self) {
        self.spi.ctrl1().modify(|_, w| w.bidiroen().set_bit());
    }

    #[inline]
    fn bidi_input(&mut self) {
        self.spi.ctrl1().modify(|_, w| w.bidiroen().clear_bit());
    }

    fn read_data_reg<W: FrameSize>(&mut self) -> W {
        // NOTE(read_volatile) read only 1 byte (the svd2rust API only allows
        // reading a half-word)
        unsafe { (*(self.spi.dat() as *const pac::spi1::Dat).cast::<vcell::VolatileCell<W>>()).get() }
    }

    fn write_data_reg<W: FrameSize>(&mut self, data: W) {
        // NOTE(write_volatile) see note above
        unsafe {
            (*(self.spi.dat() as *const pac::spi1::Dat).cast::<vcell::VolatileCell<W>>()).set(data)
        }
    }

    #[inline(always)]
    fn check_read<W: FrameSize>(&mut self) -> nb::Result<W, Error> {
        let sr = self.spi.sts().read();

        Err(if sr.over().bit_is_set() {
            Error::Overrun.into()
        } else if sr.moderr().bit_is_set() {
            Error::ModeFault.into()
        } else if sr.crcerr().bit_is_set() {
            Error::Crc.into()
        } else if sr.rne().bit_is_set() {
            return Ok(self.read_data_reg());
        } else {
            nb::Error::WouldBlock
        })
    }

    #[inline(always)]
    fn check_send<W: FrameSize>(&mut self, byte: W) -> nb::Result<(), Error> {
        let sr = self.spi.sts().read();

        Err(if sr.over().bit_is_set() {
            // Read from the DR to clear the OVR bit
            let _ = self.spi.dat().read();
            Error::Overrun.into()
        } else if sr.moderr().bit_is_set() {
            // Write to CR1 to clear MODF
            self.spi.ctrl1().modify(|_r, w| w);
            Error::ModeFault.into()
        } else if sr.crcerr().bit_is_set() {
            // Clear the CRCERR bit
            self.spi.sts().modify(|_r, w| w.crcerr().clear_bit());
            Error::Crc.into()
        } else if sr.te().bit_is_set() {
            self.write_data_reg(byte);
            return Ok(());
        } else {
            nb::Error::WouldBlock
        })
    }
    fn listen_event(&mut self, disable: Option<BitFlags<Event>>, enable: Option<BitFlags<Event>>) {
        self.spi.ctrl2().modify(|r, w| unsafe {
            w.bits({
                let mut bits = r.bits();
                if let Some(d) = disable {
                    bits &= !d.bits();
                }
                if let Some(e) = enable {
                    bits |= e.bits();
                }
                bits
            })
        });
    }
}

impl<SPI: Instance> crate::Listen for Inner<SPI> {
    type Event = Event;

    fn listen(&mut self, event: impl Into<BitFlags<Self::Event>>) {
        self.listen_event(None, Some(event.into()));
    }

    fn listen_only(&mut self, event: impl Into<BitFlags<Self::Event>>) {
        self.listen_event(Some(BitFlags::ALL), Some(event.into()));
    }

    fn unlisten(&mut self, event: impl Into<BitFlags<Self::Event>>) {
        self.listen_event(Some(event.into()), None);
    }
}

impl<SPI: Instance> crate::ClearFlags for Inner<SPI> {
    type Flag = CFlag;
    fn clear_flags(&mut self, flags: impl Into<BitFlags<Self::Flag>>) {
        if flags.into().contains(CFlag::CrcError) {
            self.spi
                .sts()
                .write(|w| unsafe { w.bits(0xffff).crcerr().clear_bit() })
        }
    }
}

impl<SPI: Instance> crate::ReadFlags for Inner<SPI> {
    type Flag = Flag;
    fn flags(&self) -> BitFlags<Self::Flag> {
        BitFlags::from_bits_truncate(self.spi.sts().read().bits())
    }
}

// Spi DMA

impl<SPI: Instance, const XFER_MODE : TransferMode> Spi<SPI, XFER_MODE, u8> {
    pub fn use_dma(self) -> DmaBuilder<SPI> {
        DmaBuilder {
            spi: self.inner.spi,
        }
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode> SpiSlave<SPI, XFER_MODE, u8> {
    pub fn use_dma(self) -> DmaBuilder<SPI> {
        DmaBuilder {
            spi: self.inner.spi,
        }
    }
}

pub struct DmaBuilder<SPI> {
    spi: SPI,
}

pub struct Tx<SPI> {
    spi: PhantomData<SPI>,
}

pub struct Rx<SPI> {
    spi: PhantomData<SPI>,
}

impl<SPI: Instance> DmaBuilder<SPI> {
    pub fn tx(self) -> Tx<SPI> {
        self.spi.ctrl2().modify(|_, w| w.tdmaen().set_bit());
        Tx { spi: PhantomData }
    }

    pub fn rx(self) -> Rx<SPI> {
        self.spi.ctrl2().modify(|_, w| w.rdmaen().set_bit());
        Rx { spi: PhantomData }
    }

    pub fn txrx(self) -> (Tx<SPI>, Rx<SPI>) {
        self.spi.ctrl2().modify(|_, w| {
            w.tdmaen().set_bit();
            w.rdmaen().set_bit()
        });
        (Tx { spi: PhantomData }, Rx { spi: PhantomData })
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W: FrameSize> Spi<SPI, XFER_MODE, W> {
    pub fn read_nonblocking(&mut self) -> nb::Result<W, Error> {
        if XFER_MODE == TransferMode::TransferModeBidirectional {
            self.bidi_input();
        }
        self.check_read()
    }

    pub fn write_nonblocking(&mut self, byte: W) -> nb::Result<(), Error> {
        if XFER_MODE == TransferMode::TransferModeBidirectional {
            self.bidi_output();
        }
        self.check_send(byte)
    }

    pub fn transfer_in_place(&mut self, words: &mut [W]) -> Result<(), Error> {
        for word in words {
            nb::block!(self.write_nonblocking(*word))?;
            *word = nb::block!(self.read_nonblocking())?;
        }

        Ok(())
    }

    pub fn transfer(&mut self, buff: &mut [W], data: &[W]) -> Result<(), Error> {
        assert_eq!(data.len(), buff.len());

        for (d, b) in data.iter().cloned().zip(buff.iter_mut()) {
            nb::block!(self.write_nonblocking(d))?;
            *b = nb::block!(self.read_nonblocking())?;
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }

    pub fn write(&mut self, words: &[W]) -> Result<(), Error> {
        if XFER_MODE == TransferMode::TransferModeBidirectional {
            self.bidi_output();
            for word in words {
                nb::block!(self.check_send(*word))?;
            }
        } else {
            for word in words {
                nb::block!(self.check_send(*word))?;
                nb::block!(self.check_read::<W>())?;
            }
        }

        Ok(())
    }

    pub fn write_iter(&mut self, words: impl IntoIterator<Item = W>) -> Result<(), Error> {
        if XFER_MODE == TransferMode::TransferModeBidirectional {
            self.bidi_output();
            for word in words.into_iter() {
                nb::block!(self.check_send(word))?;
            }
        } else {
            for word in words.into_iter() {
                nb::block!(self.check_send(word))?;
                nb::block!(self.check_read::<W>())?;
            }
        }

        Ok(())
    }

    pub fn read(&mut self, words: &mut [W]) -> Result<(), Error> {
        if XFER_MODE == TransferMode::TransferModeBidirectional {
            self.bidi_input();
            for word in words {
                *word = nb::block!(self.check_read())?;
            }
        } else if XFER_MODE == TransferMode::TransferModeRecieveOnly {
            self.spi.ctrl1().modify(|_,w| w.spien().set_bit());
            for word in words {
                *word = nb::block!(self.check_read())?;
            }
            self.spi.ctrl1().modify(|_,w| w.spien().clear_bit());
        } else {
            for word in words {
                nb::block!(self.check_send(W::default()))?;
                *word = nb::block!(self.check_read())?;
            }
        }

        Ok(())
    }
}

impl<SPI: Instance, const XFER_MODE : TransferMode, W: FrameSize> SpiSlave<SPI, XFER_MODE, W> {
    pub fn read_nonblocking(&mut self) -> nb::Result<W, Error> {
        if XFER_MODE == TransferMode::TransferModeBidirectional {
            self.bidi_input();
        }
        self.check_read()
    }

    pub fn write_nonblocking(&mut self, byte: W) -> nb::Result<(), Error> {
        if XFER_MODE == TransferMode::TransferModeBidirectional {
            self.bidi_output();
        }
        self.check_send(byte)
    }

    pub fn transfer_in_place(&mut self, words: &mut [W]) -> Result<(), Error> {
        for word in words {
            nb::block!(self.write_nonblocking(*word))?;
            *word = nb::block!(self.read_nonblocking())?;
        }

        Ok(())
    }

    pub fn transfer(&mut self, buff: &mut [W], data: &[W]) -> Result<(), Error> {
        assert_eq!(data.len(), buff.len());

        for (d, b) in data.iter().cloned().zip(buff.iter_mut()) {
            nb::block!(self.write_nonblocking(d))?;
            *b = nb::block!(self.read_nonblocking())?;
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }

    pub fn write(&mut self, words: &[W]) -> Result<(), Error> {
        if XFER_MODE == TransferMode::TransferModeBidirectional {
            self.bidi_output();
            for word in words {
                nb::block!(self.check_send(*word))?;
            }
        } else {
            for word in words {
                nb::block!(self.check_send(*word))?;
                nb::block!(self.check_read::<W>())?;
            }
        }

        Ok(())
    }

    pub fn read(&mut self, words: &mut [W]) -> Result<(), Error> {
        if XFER_MODE == TransferMode::TransferModeBidirectional {
            self.bidi_input();
            for word in words {
                *word = nb::block!(self.check_read())?;
            }
        } else {
            for word in words {
                nb::block!(self.check_send(W::default()))?;
                *word = nb::block!(self.check_read())?;
            }
        }

        Ok(())
    }
}

pub type SpiTxDma<SPI, const XFER_MODE : TransferMode, CHANNEL> = TxDma<Spi<SPI, XFER_MODE, u8>, CHANNEL>;
pub type SpiRxDma<SPI, const XFER_MODE : TransferMode, CHANNEL> = RxDma<Spi<SPI, XFER_MODE, u8>, CHANNEL>;
pub type SpiRxTxDma<SPI, const XFER_MODE : TransferMode, RXCHANNEL, TXCHANNEL> =
    RxTxDma<Spi<SPI, XFER_MODE, u8>, RXCHANNEL, TXCHANNEL>;

pub trait SpiDma<PER : Instance, const XFER_MODE : TransferMode, RXCH : crate::dma::CompatibleChannel<PER,R> + crate::dma::DMAChannel, TXCH : crate::dma::CompatibleChannel<PER,W> + crate::dma::DMAChannel> {
    fn with_rx_tx_dma(
        self,
        rxchannel: RXCH,
        txchannel: TXCH,
    ) -> SpiRxTxDma<PER, XFER_MODE, RXCH, TXCH>;
    fn with_rx_dma(self, channel: RXCH) -> SpiRxDma<PER, XFER_MODE, RXCH>;
    fn with_tx_dma(self, channel: TXCH) -> SpiTxDma<PER, XFER_MODE, TXCH>;
}

macro_rules! spi_dma {
    ($SPIi:ty, $rxdma:ident, $txdma:ident, $rxtxdma:ident) => {
        pub type $rxdma<const XFER_MODE : TransferMode, RXCH> = SpiRxDma<$SPIi, XFER_MODE, RXCH>;
        pub type $txdma<const XFER_MODE : TransferMode, TXCH> = SpiTxDma<$SPIi, XFER_MODE, TXCH>;
        pub type $rxtxdma<const XFER_MODE : TransferMode,RXCH,TXCH> = SpiRxTxDma<$SPIi, XFER_MODE, RXCH, TXCH>;

        impl<const XFER_MODE : TransferMode, RXCH,TXCH> SpiDma<$SPIi,XFER_MODE,RXCH,TXCH> for Spi<$SPIi,XFER_MODE,u8>  where
        RXCH: crate::dma::CompatibleChannel<$SPIi,R> + crate::dma::DMAChannel,
        TXCH: crate::dma::CompatibleChannel<$SPIi,W> + crate::dma::DMAChannel
        {
            fn with_tx_dma(self, mut channel: TXCH) -> SpiTxDma<$SPIi, XFER_MODE, TXCH> {
                self.spi.ctrl2().modify(|_, w| w.tdmaen().set_bit());
                channel.configure_channel();
                SpiTxDma {
                    payload: self,
                    channel,
                }
            }
            fn with_rx_dma(self, mut channel: RXCH) -> SpiRxDma<$SPIi, XFER_MODE, RXCH>
            {
               self.spi.ctrl2().modify(|_, w| w.rdmaen().set_bit());
               channel.configure_channel();
               SpiRxDma {
                   payload: self,
                   channel,
               }
           }
            fn with_rx_tx_dma(
                self,
                mut rxchannel: RXCH,
                mut txchannel: TXCH,
            ) -> SpiRxTxDma<$SPIi, XFER_MODE, RXCH, TXCH> {
                self.spi
                .ctrl2()
                .modify(|_, w| w.rdmaen().set_bit().tdmaen().set_bit());
                rxchannel.configure_channel();
                txchannel.configure_channel();
                
                SpiRxTxDma {
                    payload: self,
                    rxchannel,
                    txchannel,
                }
            }
        }

        impl<const XFER_MODE : TransferMode,TXCH: crate::dma::CompatibleChannel<$SPIi,W> + crate::dma::DMAChannel> Transmit for SpiTxDma<$SPIi, XFER_MODE, TXCH> {
            type TxChannel = TXCH;
            type ReceivedWord = u8;
        }

        impl<const XFER_MODE : TransferMode,RXCH: crate::dma::CompatibleChannel<$SPIi,R> + crate::dma::DMAChannel> Receive for SpiRxDma<$SPIi, XFER_MODE, RXCH> {
            type RxChannel = RXCH;
            type TransmittedWord = u8;
        }

        impl<const XFER_MODE : TransferMode,RXCH: crate::dma::CompatibleChannel<$SPIi,R> + crate::dma::DMAChannel,TXCH: crate::dma::CompatibleChannel<$SPIi,W> + crate::dma::DMAChannel> Transmit for SpiRxTxDma<$SPIi, XFER_MODE, RXCH,TXCH> {
            type TxChannel = TXCH;
            type ReceivedWord = u8;
        }

        impl<const XFER_MODE : TransferMode,RXCH: crate::dma::CompatibleChannel<$SPIi,R> + crate::dma::DMAChannel,TXCH: crate::dma::CompatibleChannel<$SPIi,W> + crate::dma::DMAChannel> Receive for SpiRxTxDma<$SPIi, XFER_MODE, RXCH,TXCH> {
            type RxChannel = RXCH;
            type TransmittedWord = u8;
        }

        impl<const XFER_MODE : TransferMode, TXCH: crate::dma::CompatibleChannel<$SPIi,W> + crate::dma::DMAChannel> SpiTxDma<$SPIi, XFER_MODE, TXCH> {
            pub fn release(self) -> (Spi<$SPIi, XFER_MODE, u8>, TXCH) {
                let SpiTxDma { payload, channel } = self;
                payload.spi.ctrl2().modify(|_, w| w.tdmaen().clear_bit());
                (payload, channel)
            }
        }

        impl<const XFER_MODE : TransferMode, RXCH: crate::dma::CompatibleChannel<$SPIi,R> + crate::dma::DMAChannel> SpiRxDma<$SPIi, XFER_MODE, RXCH> {
            pub fn release(self) -> (Spi<$SPIi, XFER_MODE, u8>, RXCH) {
                let SpiRxDma { payload, channel } = self;
                payload.spi.ctrl2().modify(|_, w| w.rdmaen().clear_bit());
                (payload, channel)
            }
        }

        impl<const XFER_MODE : TransferMode, RXCH: crate::dma::CompatibleChannel<$SPIi,R> + crate::dma::DMAChannel,TXCH: crate::dma::CompatibleChannel<$SPIi,W> + crate::dma::DMAChannel> SpiRxTxDma<$SPIi, XFER_MODE, RXCH, TXCH> {
            pub fn release(self) -> (Spi<$SPIi, XFER_MODE, u8>, RXCH, TXCH) {
                let SpiRxTxDma {
                    payload,
                    rxchannel,
                    txchannel,
                } = self;
                payload
                    .spi
                    .ctrl2()
                    .modify(|_, w| w.rdmaen().clear_bit().tdmaen().clear_bit());
                (payload, rxchannel, txchannel)
            }
        }

        impl<const XFER_MODE : TransferMode,TXCH: crate::dma::CompatibleChannel<$SPIi,W> + crate::dma::DMAChannel> TransferPayload for SpiTxDma<$SPIi, XFER_MODE, TXCH> {
            fn start(&mut self) {
                self.channel.start();
            }
            fn stop(&mut self) {
                self.channel.stop();
            }
        }

        impl<const XFER_MODE : TransferMode,RXCH: crate::dma::CompatibleChannel<$SPIi,R> + crate::dma::DMAChannel> TransferPayload for SpiRxDma<$SPIi, XFER_MODE, RXCH> {
            fn start(&mut self) {
                self.channel.start();
                if XFER_MODE == TransferMode::TransferModeRecieveOnly {
                    self.payload.enable(true);
                }

            }
            fn stop(&mut self) {
                self.channel.stop();
                if XFER_MODE == TransferMode::TransferModeRecieveOnly {
                    self.payload.enable(false);
                }
            }
        }

        impl<const XFER_MODE : TransferMode,RXCH: crate::dma::CompatibleChannel<$SPIi,R> + crate::dma::DMAChannel,TXCH: crate::dma::CompatibleChannel<$SPIi,W> + crate::dma::DMAChannel> TransferPayload for SpiRxTxDma<$SPIi, XFER_MODE,RXCH,TXCH> {
            fn start(&mut self) {
                self.rxchannel.start();
                self.txchannel.start();
            }
            fn stop(&mut self) {
                self.txchannel.stop();
                self.rxchannel.stop();
            }
        }

        impl<B, const XFER_MODE : TransferMode, RXCH: crate::dma::CompatibleChannel<$SPIi,R> + crate::dma::DMAChannel> crate::dma::ReadDma<B, u8> for SpiRxDma<$SPIi, XFER_MODE, RXCH>
        where
            B: WriteBuffer<Word = u8>,
        {
            fn read(mut self, mut buffer: B) -> Transfer<W, B, Self> {
                // NOTE(unsafe) We own the buffer now and we won't call other `&mut` on it
                // until the end of the transfer.
                let (ptr, len) = unsafe { buffer.write_buffer() };
                self.channel.set_peripheral_address(
                    unsafe { (*<$SPIi>::ptr()).dat().as_ptr() as u32 },
                    false,
                );
                self.channel.set_memory_address(ptr as u32, true);
                self.channel.set_transfer_length(len);

                atomic::compiler_fence(Ordering::Release);
                self.channel.st().chcfg().modify(|_, w| {
                    w
                        // memory to memory mode disabled
                        .mem2mem()
                        .disabled()
                        // medium channel priority level
                        .priolvl()
                        .medium()
                        // 8-bit memory size
                        .msize()
                        .bits8()
                        // 8-bit peripheral size
                        .psize()
                        .bits8()
                        // circular mode disabled
                        .circ()
                        .disabled()
                        // write to memory
                        .dir()
                        .from_peripheral()
                });
                self.start();

                Transfer::w(buffer, self)
            }
        }

        impl<B, const XFER_MODE : TransferMode,TXCH: crate::dma::CompatibleChannel<$SPIi,W> + crate::dma::DMAChannel> crate::dma::WriteDma<B, u8>
            for SpiTxDma<$SPIi, XFER_MODE, TXCH>
        where
            B: ReadBuffer<Word = u8>,
        {
            fn write(mut self, buffer: B) -> Transfer<R, B, Self> {
                // NOTE(unsafe) We own the buffer now and we won't call other `&mut` on it
                // until the end of the transfer.
                let (ptr, len) = unsafe { buffer.read_buffer() };
                self.channel.set_peripheral_address(
                    unsafe { (*<$SPIi>::ptr()).dat().as_ptr() as u32 },
                    false,
                );
                self.channel.set_memory_address(ptr as u32, true);
                self.channel.set_transfer_length(len);

                atomic::compiler_fence(Ordering::Release);
                self.channel.st().chcfg().modify(|_, w| {
                    w
                        // memory to memory mode disabled
                        .mem2mem()
                        .disabled()
                        // medium channel priority level
                        .priolvl()
                        .medium()
                        // 8-bit memory size
                        .msize()
                        .bits8()
                        // 8-bit peripheral size
                        .psize()
                        .bits8()
                        // circular mode disabled
                        .circ()
                        .disabled()
                        // read from memory
                        .dir()
                        .from_memory()
                });
                self.start();

                Transfer::r(buffer, self)
            }
        }

        impl<RXB, TXB, const XFER_MODE : TransferMode, RXCH: crate::dma::CompatibleChannel<$SPIi,R> + crate::dma::DMAChannel,TXCH: crate::dma::CompatibleChannel<$SPIi,W> + crate::dma::DMAChannel> crate::dma::ReadWriteDma<RXB, TXB, u8>
            for SpiRxTxDma<$SPIi, XFER_MODE, RXCH, TXCH>
        where
            RXB: WriteBuffer<Word = u8>,
            TXB: ReadBuffer<Word = u8>,
        {
            fn read_write(
                mut self,
                mut rxbuffer: RXB,
                txbuffer: TXB,
            ) -> Transfer<W, (RXB, TXB), Self> {
                // NOTE(unsafe) We own the buffer now and we won't call other `&mut` on it
                // until the end of the transfer.
                let (rxptr, rxlen) = unsafe { rxbuffer.write_buffer() };
                let (txptr, txlen) = unsafe { txbuffer.read_buffer() };

                if rxlen != txlen {
                    panic!("receive and send buffer lengths do not match!");
                }

                self.rxchannel.set_peripheral_address(
                    unsafe { (*<$SPIi>::ptr()).dat().as_ptr() as u32 },
                    false,
                );
                self.rxchannel.set_memory_address(rxptr as u32, true);
                self.rxchannel.set_transfer_length(rxlen);

                self.txchannel.set_peripheral_address(
                    unsafe { (*<$SPIi>::ptr()).dat().as_ptr() as u32 },
                    false,
                );
                self.txchannel.set_memory_address(txptr as u32, true);
                self.txchannel.set_transfer_length(txlen);

                atomic::compiler_fence(Ordering::Release);
                self.rxchannel.st().chcfg().modify(|_, w| {
                    w
                        // memory to memory mode disabled
                        .mem2mem()
                        .disabled()
                        // medium channel priority level
                        .priolvl()
                        .medium()
                        // 8-bit memory size
                        .msize()
                        .bits8()
                        // 8-bit peripheral size
                        .psize()
                        .bits8()
                        // circular mode disabled
                        .circ()
                        .disabled()
                        // write to memory
                        .dir()
                        .from_peripheral()
                });
                self.txchannel.st().chcfg().modify(|_, w| {
                    w
                        // memory to memory mode disabled
                        .mem2mem()
                        .disabled()
                        // medium channel priority level
                        .priolvl()
                        .medium()
                        // 8-bit memory size
                        .msize()
                        .bits8()
                        // 8-bit peripheral size
                        .psize()
                        .bits8()
                        // circular mode disabled
                        .circ()
                        .disabled()
                        // read from memory
                        .dir()
                        .from_memory()
                });
                self.start();

                Transfer::w((rxbuffer, txbuffer), self)
            }
        }
    };
}

spi_dma!(
    pac::Spi1,
    Spi1RxDma,
    Spi1TxDma,
    Spi1RxTxDma
);
spi_dma!(
    pac::Spi2,
    Spi2RxDma,
    Spi2TxDma,
    Spi2RxTxDma
);
spi_dma!(
    pac::Spi3,
    Spi3RxDma,
    Spi3TxDma,
    Spi3RxTxDma
);