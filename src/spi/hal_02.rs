pub use embedded_hal_02::spi::{Mode, Phase, Polarity};

impl From<Polarity> for super::Polarity {
    fn from(p: Polarity) -> Self {
        match p {
            Polarity::IdleLow => Self::IdleLow,
            Polarity::IdleHigh => Self::IdleHigh,
        }
    }
}

impl From<Phase> for super::Phase {
    fn from(p: Phase) -> Self {
        match p {
            Phase::CaptureOnFirstTransition => Self::CaptureOnFirstTransition,
            Phase::CaptureOnSecondTransition => Self::CaptureOnSecondTransition,
        }
    }
}

impl From<Mode> for super::Mode {
    fn from(m: Mode) -> Self {
        Self {
            polarity: m.polarity.into(),
            phase: m.phase.into(),
        }
    }
}

mod nb {

    use crate::spi::TransferMode;

    use super::super::{Error, FrameSize, Instance, Spi};
    use embedded_hal_02::spi::FullDuplex;

    impl<SPI, const XFER_MODE: TransferMode, W: FrameSize> FullDuplex<W> for Spi<SPI, XFER_MODE, W>
    where
        SPI: Instance,
    {
        type Error = Error;

        fn read(&mut self) -> nb::Result<W, Error> {
            self.read_nonblocking()
        }

        fn send(&mut self, byte: W) -> nb::Result<(), Error> {
            self.write_nonblocking(byte)
        }
    }
}

mod blocking {
    use crate::spi::TransferMode;
    use super::super::{Error, Instance, Spi};
    use embedded_hal_02::blocking::spi::{Operation, Transactional, Transfer, Write, WriteIter};

    impl<SPI> Transfer<u8> for Spi<SPI, {TransferMode::TransferModeNormal}, u8>
    where
        SPI: Instance,
    {
        type Error = Error;

        fn transfer<'w>(&mut self, words: &'w mut [u8]) -> Result<&'w [u8], Self::Error> {
            self.transfer_in_place(words)?;

            Ok(words)
        }
    }

    impl<SPI> Transfer<u16> for Spi<SPI, {TransferMode::TransferModeNormal}, u16>
    where
        SPI: Instance,
    {
        type Error = Error;

        fn transfer<'w>(&mut self, words: &'w mut [u16]) -> Result<&'w [u16], Self::Error> {
            self.transfer_in_place(words)?;
            Ok(words)
        }
    }

    impl<SPI, const XFER_MODE: TransferMode> Write<u8> for Spi<SPI, XFER_MODE, u8>
    where
        SPI: Instance,
    {
        type Error = Error;

        fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
            self.write(words)
        }
    }

    impl<SPI, const XFER_MODE: TransferMode> WriteIter<u8> for Spi<SPI, XFER_MODE, u8>
    where
        SPI: Instance,
    {
        type Error = Error;

        fn write_iter<WI>(&mut self, words: WI) -> Result<(), Self::Error>
        where
            WI: IntoIterator<Item = u8>,
        {
            self.write_iter(words)
        }
    }

    impl<SPI, const XFER_MODE: TransferMode> Write<u16> for Spi<SPI, XFER_MODE, u16>
    where
        SPI: Instance,
    {
        type Error = Error;

        fn write(&mut self, words: &[u16]) -> Result<(), Self::Error> {
            self.write(words)
        }
    }

    impl<SPI, const XFER_MODE: TransferMode> WriteIter<u16> for Spi<SPI, XFER_MODE, u16>
    where
        SPI: Instance,
    {
        type Error = Error;

        fn write_iter<WI>(&mut self, words: WI) -> Result<(), Self::Error>
        where
            WI: IntoIterator<Item = u16>,
        {
            self.write_iter(words)
        }
    }

    impl<SPI, const XFER_MODE: TransferMode, W: Copy + 'static> Transactional<W> for Spi<SPI, XFER_MODE, W>
    where
        Self: Transfer<W, Error = Error> + Write<W, Error = Error>,
        SPI: Instance,
    {
        type Error = Error;

        fn exec(&mut self, operations: &mut [Operation<'_, W>]) -> Result<(), Error> {
            for op in operations {
                match op {
                    Operation::Write(w) => self.write(w)?,
                    Operation::Transfer(t) => self.transfer(t).map(|_| ())?,
                }
            }

            Ok(())
        }
    }
}
