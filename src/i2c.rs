use core::ops::Deref;

use crate::pac::{self, I2c1, I2c2};
#[cfg(any(feature = "n32g451",feature = "n32g452",feature = "n32g455",feature = "n32g457",feature = "n32g4fr"))]
use crate::pac::{I2c3, I2c4};

use crate::rcc::{Enable, Reset};

use crate::gpio::{self, Alternate, OpenDrain};

use crate::rcc::Clocks;
use fugit::{HertzU32 as Hertz, RateExtU32};

mod hal_02;
mod hal_1;

pub mod dma;

#[derive(Debug, Eq, PartialEq)]
pub enum DutyCycle {
    Ratio2to1,
    Ratio16to9,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Standard {
        frequency: Hertz,
    },
    Fast {
        frequency: Hertz,
        duty_cycle: DutyCycle,
    },
}

impl Mode {
    pub fn standard(frequency: Hertz) -> Self {
        Self::Standard { frequency }
    }

    pub fn fast(frequency: Hertz, duty_cycle: DutyCycle) -> Self {
        Self::Fast {
            frequency,
            duty_cycle,
        }
    }

    pub fn get_frequency(&self) -> Hertz {
        match *self {
            Self::Standard { frequency } => frequency,
            Self::Fast { frequency, .. } => frequency,
        }
    }
}

impl From<Hertz> for Mode {
    fn from(frequency: Hertz) -> Self {
        let k100: Hertz = 100.kHz();
        if frequency <= k100 {
            Self::Standard { frequency }
        } else {
            Self::Fast {
                frequency,
                duty_cycle: DutyCycle::Ratio2to1,
            }
        }
    }
}

/// I2C abstraction
pub struct I2c<I2C: Instance, PINS>
{
    i2c: I2C,
    pins: PINS,
}

pub use embedded_hal::i2c::NoAcknowledgeSource;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[non_exhaustive]
pub enum Error {
    Overrun,
    NoAcknowledge(NoAcknowledgeSource),
    Timeout,
    // Note: The Bus error type is not currently returned, but is maintained for compatibility.
    Bus,
    Crc,
    ArbitrationLoss,
}

impl Error {
    pub(crate) fn nack_addr(self) -> Self {
        match self {
            Error::NoAcknowledge(NoAcknowledgeSource::Unknown) => {
                Error::NoAcknowledge(NoAcknowledgeSource::Address)
            }
            e => e,
        }
    }
    pub(crate) fn nack_data(self) -> Self {
        match self {
            Error::NoAcknowledge(NoAcknowledgeSource::Unknown) => {
                Error::NoAcknowledge(NoAcknowledgeSource::Data)
            }
            e => e,
        }
    }
}

pub trait Instance:
    crate::Sealed + Deref<Target = crate::pac::i2c1::RegisterBlock> + Enable + Reset 
{

    #[doc(hidden)]
    fn ptr() -> *const crate::pac::i2c1::RegisterBlock;
}

pub trait Pins<I2C>: Sized {
    const REMAP: bool;
}

impl Pins<pac::I2c1>
    for (
        gpio::PB6<Alternate<OpenDrain>>,
        gpio::PB7<Alternate<OpenDrain>>,
    )
{
    const REMAP: bool = false;
}

impl Pins<pac::I2c1>
    for (
        gpio::PB8<Alternate<OpenDrain>>,
        gpio::PB9<Alternate<OpenDrain>>,
    )
{
    const REMAP: bool = true;
}

impl Pins<pac::I2c2>
    for (
        gpio::PB10<Alternate<OpenDrain>>,
        gpio::PB11<Alternate<OpenDrain>>,
    )
{
    const REMAP: bool = false;
}

// editor's note: the rmp register docs in the user guide claims this is pc4 but this is a typo
impl Pins<pac::I2c2>
    for (
        gpio::PA4<Alternate<OpenDrain>>,
        gpio::PA5<Alternate<OpenDrain>>,
    )
{
    const REMAP: bool = true;
}


// Implemented by all I2C instances
macro_rules! i2c {
    ($I2C:ty: $I2c:ident) => {
        pub type $I2c = I2c<$I2C, dyn Pins<$I2C>>;

        impl Instance for $I2C {
            fn ptr() -> *const crate::pac::i2c1::RegisterBlock {
                <$I2C>::ptr() as *const _
            }
        }
    };
}

i2c! { pac::I2c1: I2c1Inst }
i2c! { pac::I2c2: I2c2Inst }

#[cfg(any(feature = "n32g451",feature = "n32g452",feature = "n32g455",feature = "n32g457",feature = "n32g4fr"))]
i2c! { pac::I2c3: I2c3Inst }
#[cfg(any(feature = "n32g451",feature = "n32g452",feature = "n32g455",feature = "n32g457",feature = "n32g4fr"))]
i2c! { pac::I2c4: I2c4Inst }

impl<PINS> I2c<I2c1, PINS> {
    /// Creates a generic I2C2 object on pins PB10 and PB11 using the embedded-hal `BlockingI2c` trait.
    pub fn i2c1<M: Into<Mode>>(i2c: I2c1, pins: PINS, mode: M, clocks: &Clocks) -> Self
    where
        PINS: Pins<I2c1>,
    {
        I2c::<I2c1, _>::new(i2c, pins, mode, clocks)
    }
}

impl<PINS> I2c<I2c2, PINS> {
    /// Creates a generic I2C2 object on pins PB10 and PB11 using the embedded-hal `BlockingI2c` trait.
    pub fn i2c2<M: Into<Mode>>(i2c: I2c2, pins: PINS, mode: M, clocks: &Clocks) -> Self
    where
        PINS: Pins<I2c2>,
    {
        I2c::<I2c2, _>::new(i2c, pins, mode, clocks)
    }
}

#[cfg(any(feature = "n32g451",feature = "n32g452",feature = "n32g455",feature = "n32g457",feature = "n32g4fr"))]
impl<PINS> I2c<I2c3, PINS> {
    /// Creates a generic I2C2 object on pins PB10 and PB11 using the embedded-hal `BlockingI2c` trait.
    pub fn i2c3<M: Into<Mode>>(i2c: I2c3, pins: PINS, mode: M, clocks: &Clocks) -> Self
    where
        PINS: Pins<I2c3>,
    {
        I2c::<I2c3, _>::new(i2c, pins, mode, clocks)
    }
}

#[cfg(any(feature = "n32g451",feature = "n32g452",feature = "n32g455",feature = "n32g457",feature = "n32g4fr"))]
impl<PINS> I2c<I2c4, PINS> {
    /// Creates a generic I2C2 object on pins PB10 and PB11 using the embedded-hal `BlockingI2c` trait.
    pub fn i2c4<M: Into<Mode>>(i2c: I2c4, pins: PINS, mode: M, clocks: &Clocks) -> Self
    where
        PINS: Pins<I2c4>,
    {
        I2c::<I2c4, _>::new(i2c, pins, mode, clocks)
    }
}


impl<I2C, PINS> I2c<I2C, PINS>
where
    I2C: Instance,
    PINS: Pins<I2C>
{
    pub fn new(
        i2c: I2C,
        pins: PINS,
        mode: impl Into<Mode>,
        clocks: &Clocks,
    ) -> Self {
        unsafe {
            // Enable and reset clock.
            I2C::enable_unchecked();
            I2C::reset_unchecked();
        }

        let i2c = I2c { i2c, pins };
        i2c.i2c_init(mode, clocks.pclk1());
        i2c
    }

    pub fn release(self) -> (I2C, PINS) {
        (self.i2c, self.pins)
    }
}

impl<I2C: Instance,PINS> I2c<I2C,PINS> {
    fn i2c_init(&self, mode: impl Into<Mode>, pclk: Hertz) {
        let mode = mode.into();
        // Make sure the I2C unit is disabled so we can configure it
        self.i2c.ctrl1().modify(|_, w| w.en().clear_bit());

        // Calculate settings for I2C speed modes
        let clock = pclk.raw();
        let clc_mhz = clock / 1_000_000;
        assert!((2..=50).contains(&clc_mhz));

        // Configure bus frequency into I2C peripheral
        self.i2c
            .ctrl2()
            .write(|w| unsafe { w.clkfreq().bits(clc_mhz as u8) });

        let trise = match mode {
            Mode::Standard { .. } => clc_mhz + 1,
            Mode::Fast { .. } => clc_mhz * 300 / 1000 + 1,
        };

        // Configure correct rise times
        unsafe { self.i2c.tmrise().write(|w| w.tmrise().bits(trise as u8)) };

        match mode {
            // I2C clock control calculation
            Mode::Standard { frequency } => {
                let mut ccr = (clock / (frequency.raw() * 2)).max(4);
                if ccr < 0x04 {
                    ccr = 0x04
                }
                // Set clock to standard mode with appropriate parameters for selected speed
                self.i2c.clkctrl().modify(|_,w| unsafe {
                    w.fsmode()
                        .clear_bit()
                        .duty()
                        .clear_bit()
                        .clkctrl()
                        .bits(ccr as u16)
                });
            }
            Mode::Fast {
                frequency,
                duty_cycle,
            } => match duty_cycle {
                DutyCycle::Ratio2to1 => {
                    let ccr = (clock / (frequency.raw() * 3)).max(1);

                    // Set clock to fast mode with appropriate parameters for selected speed (2:1 duty cycle)
                    self.i2c.clkctrl().write(|w| unsafe {
                        w.fsmode().set_bit().duty().clear_bit().clkctrl().bits(ccr as u16)
                    });
                }
                DutyCycle::Ratio16to9 => {
                    let ccr = (clock / (frequency.raw() * 25)).max(1);

                    // Set clock to fast mode with appropriate parameters for selected speed (16:9 duty cycle)
                    self.i2c.clkctrl().write(|w| unsafe {
                        w.fsmode().set_bit().duty().set_bit().clkctrl().bits(ccr as u16)
                    });
                }
            },
        }

        // Enable the I2C processing
        self.i2c.ctrl1().modify(|_, w| w.en().set_bit());
    }

    fn check_and_clear_error_flags(&self) -> Result<pac::i2c1::sts1::R, Error> {
        // Note that flags should only be cleared once they have been registered. If flags are
        // cleared otherwise, there may be an inherent race condition and flags may be missed.
        let sts1 = self.i2c.sts1().read();

        if sts1.timout().bit_is_set() {
            self.i2c.sts1().modify(|_, w| w.timout().clear_bit());
            return Err(Error::Timeout);
        }

        if sts1.pecerr().bit_is_set() {
            self.i2c.sts1().modify(|_, w| w.pecerr().clear_bit());
            return Err(Error::Crc);
        }

        if sts1.overrun().bit_is_set() {
            self.i2c.sts1().modify(|_, w| w.overrun().clear_bit());
            return Err(Error::Overrun);
        }

        if sts1.ackfail().bit_is_set() {
            self.i2c.sts1().modify(|_, w| w.ackfail().clear_bit());
            return Err(Error::NoAcknowledge(NoAcknowledgeSource::Unknown));
        }

        if sts1.arlost().bit_is_set() {
            self.i2c.sts1().modify(|_, w| w.arlost().clear_bit());
            return Err(Error::ArbitrationLoss);
        }

        // The errata indicates that BERR may be incorrectly detected. It recommends ignoring and
        // clearing the BERR bit instead.
        if sts1.buserr().bit_is_set() {
            self.i2c.sts1().modify(|_, w| w.buserr().clear_bit());
        }

        Ok(sts1)
    }

    /// Sends START and Address for writing
    #[inline(always)]
    fn prepare_write(&self, addr: u8) -> Result<(), Error> {
        // Send a START condition
        self.i2c.ctrl1().modify(|_, w| w.startgen().set_bit());

        // Wait until START condition was generated
        while self.check_and_clear_error_flags()?.startbf().bit_is_clear() {}

        // Also wait until signalled we're master and everything is waiting for us
        loop {
            self.check_and_clear_error_flags()?;

            let sr2 = self.i2c.sts2().read();
            if sr2.msmode().bit_is_set() && sr2.busy().bit_is_set() {
                break;
            }
        }

        // Set up current address, we're trying to talk to
        self.i2c
            .dat()
            .write(|w| unsafe { w.bits(u32::from(addr) << 1) });

        // Wait until address was sent
        loop {
            // Check for any I2C errors. If a NACK occurs, the ADDR bit will never be set.
            let sts1 = self
                .check_and_clear_error_flags()
                .map_err(Error::nack_addr)?;

            // Wait for the address to be acknowledged
            if sts1.addrf().bit_is_set() {
                break;
            }
        }
        self.i2c.sts1().read();
        // Clear condition by reading SR2
        self.i2c.sts2().read();

        Ok(())
    }

    /// Sends START and Address for reading
    fn prepare_read(&self, addr: u8) -> Result<(), Error> {
        // Send a START condition and set ACK bit
        self.i2c
            .ctrl1()
            .modify(|_, w| w.startgen().set_bit().acken().set_bit());

        // Wait until START condition was generated
        while self.i2c.sts1().read().startbf().bit_is_clear() {}

        // Also wait until signalled we're master and everything is waiting for us
        while {
            let sts2 = self.i2c.sts2().read();
            sts2.msmode().bit_is_clear() && sts2.busy().bit_is_clear()
        } {}

        // Set up current address, we're trying to talk to
        self.i2c
            .dat()
            .write(|w| unsafe { w.bits((u32::from(addr) << 1) + 1) });

        // Wait until address was sent
        loop {
            self.check_and_clear_error_flags()
                .map_err(Error::nack_addr)?;
            if self.i2c.sts1().read().addrf().bit_is_set() {
                break;
            }
        }
        self.i2c.sts1().read();
        // Clear condition by reading SR2
        self.i2c.sts2().read();

        Ok(())
    }

    fn write_bytes(&mut self, bytes: impl Iterator<Item = u8>) -> Result<(), Error> {
        // Send bytes
        for c in bytes {
            self.send_byte(c)?;
        }

        // Fallthrough is success
        Ok(())
    }

    fn send_byte(&self, byte: u8) -> Result<(), Error> {
        // Wait until we're ready for sending
        // Check for any I2C errors. If a NACK occurs, the ADDR bit will never be set.
        while self
            .check_and_clear_error_flags()
            .map_err(Error::nack_addr)?
            .txdate()
            .bit_is_clear()
        {}

        // Push out a byte of data
        self.i2c.dat().write(|w| unsafe { w.bits(u32::from(byte)) });

        // Wait until byte is transferred
        // Check for any potential error conditions.
        while self
            .check_and_clear_error_flags()
            .map_err(Error::nack_data)?
            .bytef()
            .bit_is_clear()
        {}
        Ok(())
    }

    fn recv_byte(&self) -> Result<u8, Error> {
        loop {
            // Check for any potential error conditions.
            self.check_and_clear_error_flags()
                .map_err(Error::nack_data)?;

            if self.i2c.sts1().read().rxdatne().bit_is_set() {
                break;
            }
        }

        let value = self.i2c.dat().read().bits() as u8;
        Ok(value)
    }

    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        // Receive bytes into buffer
        for c in buffer {
            *c = self.recv_byte()?;
        }

        Ok(())
    }

    pub fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Error> {
        if buffer.is_empty() {
            return Err(Error::Overrun);
        }

        self.prepare_read(addr)?;
        self.read_wo_prepare(buffer)
    }

    /// Reads like normal but does'n generate start and don't send address
    fn read_wo_prepare(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        if let Some((last, buffer)) = buffer.split_last_mut() {
            // Read all bytes but not last
            self.read_bytes(buffer)?;

            // Prepare to send NACK then STOP after next byte
            self.i2c
                .ctrl1()
                .modify(|_, w| w.acken().clear_bit().stopgen().set_bit());

            // Receive last byte
            *last = self.recv_byte()?;

            // Wait for the STOP to be sent.
            while self.i2c.ctrl1().read().stopgen().bit_is_set() {}

            // Fallthrough is success
            Ok(())
        } else {
            Err(Error::Overrun)
        }
    }

    pub fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Error> {
        self.prepare_write(addr)?;
        self.write_wo_prepare(bytes)
    }

    /// Writes like normal but does'n generate start and don't send address
    fn write_wo_prepare(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.write_bytes(bytes.iter().cloned())?;

        // Send a STOP condition
        self.i2c.ctrl1().modify(|_, w| w.stopgen().set_bit());

        // Wait for STOP condition to transmit.
        while self.i2c.ctrl1().read().stopgen().bit_is_set() {}

        // Fallthrough is success
        Ok(())
    }

    pub fn write_iter<B>(&mut self, addr: u8, bytes: B) -> Result<(), Error>
    where
        B: IntoIterator<Item = u8>,
    {
        self.prepare_write(addr)?;
        self.write_bytes(bytes.into_iter())?;

        // Send a STOP condition
        self.i2c.ctrl1().modify(|_, w| w.stopgen().set_bit());

        // Wait for STOP condition to transmit.
        while self.i2c.ctrl1().read().stopgen().bit_is_set() {}

        // Fallthrough is success
        Ok(())
    }

    pub fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Error> {
        self.prepare_write(addr)?;
        self.write_bytes(bytes.iter().cloned())?;
        self.read(addr, buffer)
    }

    pub fn write_iter_read<B>(&mut self, addr: u8, bytes: B, buffer: &mut [u8]) -> Result<(), Error>
    where
        B: IntoIterator<Item = u8>,
    {
        self.prepare_write(addr)?;
        self.write_bytes(bytes.into_iter())?;
        self.read(addr, buffer)
    }

    pub fn transaction<'a>(
        &mut self,
        addr: u8,
        mut ops: impl Iterator<Item = Hal1Operation<'a>>,
    ) -> Result<(), Error> {
        if let Some(mut prev_op) = ops.next() {
            // 1. Generate Start for operation
            match &prev_op {
                Hal1Operation::Read(_) => self.prepare_read(addr)?,
                Hal1Operation::Write(_) => self.prepare_write(addr)?,
            };

            for op in ops {
                // 2. Execute previous operations.
                match &mut prev_op {
                    Hal1Operation::Read(rb) => self.read_bytes(rb)?,
                    Hal1Operation::Write(wb) => self.write_bytes(wb.iter().cloned())?,
                };
                // 3. If operation changes type we must generate new start
                match (&prev_op, &op) {
                    (Hal1Operation::Read(_), Hal1Operation::Write(_)) => {
                        self.prepare_write(addr)?
                    }
                    (Hal1Operation::Write(_), Hal1Operation::Read(_)) => self.prepare_read(addr)?,
                    _ => {} // No changes if operation have not changed
                }

                prev_op = op;
            }

            // 4. Now, prev_op is last command use methods variations that will generate stop
            match prev_op {
                Hal1Operation::Read(rb) => self.read_wo_prepare(rb)?,
                Hal1Operation::Write(wb) => self.write_wo_prepare(wb)?,
            };
        }

        // Fallthrough is success
        Ok(())
    }

    pub fn transaction_slice(
        &mut self,
        addr: u8,
        ops_slice: &mut [Hal1Operation<'_>],
    ) -> Result<(), Error> {
        transaction_impl!(self, addr, ops_slice, Hal1Operation);
        // Fallthrough is success
        Ok(())
    }

    fn transaction_slice_hal_02(
        &mut self,
        addr: u8,
        ops_slice: &mut [Hal02Operation<'_>],
    ) -> Result<(), Error> {
        transaction_impl!(self, addr, ops_slice, Hal02Operation);
        // Fallthrough is success
        Ok(())
    }
}

macro_rules! transaction_impl {
    ($self:ident, $addr:ident, $ops_slice:ident, $Operation:ident) => {
        let i2c = $self;
        let addr = $addr;
        let mut ops = $ops_slice.iter_mut();

        if let Some(mut prev_op) = ops.next() {
            // 1. Generate Start for operation
            match &prev_op {
                $Operation::Read(_) => i2c.prepare_read(addr)?,
                $Operation::Write(_) => i2c.prepare_write(addr)?,
            };

            for op in ops {
                // 2. Execute previous operations.
                match &mut prev_op {
                    $Operation::Read(rb) => i2c.read_bytes(rb)?,
                    $Operation::Write(wb) => i2c.write_bytes(wb.iter().cloned())?,
                };
                // 3. If operation changes type we must generate new start
                match (&prev_op, &op) {
                    ($Operation::Read(_), $Operation::Write(_)) => i2c.prepare_write(addr)?,
                    ($Operation::Write(_), $Operation::Read(_)) => i2c.prepare_read(addr)?,
                    _ => {} // No changes if operation have not changed
                }

                prev_op = op;
            }

            // 4. Now, prev_op is last command use methods variations that will generate stop
            match prev_op {
                $Operation::Read(rb) => i2c.read_wo_prepare(rb)?,
                $Operation::Write(wb) => i2c.write_wo_prepare(wb)?,
            };
        }
    };
}
use transaction_impl;

type Hal1Operation<'a> = embedded_hal::i2c::Operation<'a>;
type Hal02Operation<'a> = embedded_hal_02::blocking::i2c::Operation<'a>;
