

use core::{marker::PhantomData, mem::transmute};

use super::{I2c, Instance};
use crate::dma::{ChannelStatus, CompatibleChannel, DMAChannel, TransferPayload};


#[non_exhaustive]
pub enum Error {
    I2CError(super::Error),
    TransferError,
}

/// Tag for TX/RX channel that a corresponding channel should not be used in DMA mode
#[non_exhaustive]
pub struct NoDMA;

/// Callback type to notify user code of completion I2C transfers
//pub type I2cCompleteCallback = fn(Result<(), Error>);

pub trait I2CMasterWriteDMA {
    /// Writes `bytes` to slave with address `addr` in non-blocking mode
    ///
    /// # Arguments
    /// * `addr` - slave address
    /// * `bytes` - byte slice that need to send
    ///
    /// # Safety
    /// This function relies on supplied slice `bytes` until the DMA completes (e.g. in the interrupt). So the slice must live until that moment.
    ///
    unsafe fn write_dma(
        &mut self,
        addr: u8,
        bytes: &[u8],
    ) -> nb::Result<(), super::Error>;
}

pub trait I2CMasterReadDMA {
    /// Reads bytes from slave device with address `addr` in non-blocking mode and writes these bytes in `buf`
    ///
    /// # Arguments
    /// * `addr` - slave address
    /// * `buf` - byte slice where received bytes will be written
    ///
    /// # Safety
    /// This function relies on supplied slice `buf` until the DMA completes (e.g. in the interrupt). So the slice must live until that moment.
    ///
    unsafe fn read_dma(
        &mut self,
        addr: u8,
        buf: &mut [u8],
    ) -> nb::Result<(), super::Error>;
}

pub trait I2CMasterWriteReadDMA {
    /// Writes `bytes` to slave with address `addr` in non-blocking mode and then generate ReStart and receive a bytes from a same device
    ///
    /// # Arguments
    /// * `addr` - slave address
    /// * `bytes` - byte slice that need to send
    /// * `buf` - byte slice where received bytes will be written
    ///
    /// # Safety
    /// This function relies on supplied slices `bytes` and `buf` until the DMA completion interrupt is triggered. So slices must live until that moment.
    ///
    /// # Warning
    /// `callback` may be called before function returns value. It happens on errors in preparation stages.
    unsafe fn write_read_dma(
        &mut self,
        addr: u8,
        bytes: &[u8],
        buf: &mut [u8],
    ) -> nb::Result<(), super::Error>;
}

/// Trait with handle interrupts functions
pub trait I2CMasterHandleIT {
    fn handle_dma_interrupt(&mut self) -> Result<ChannelStatus, Error>;
    fn handle_error_interrupt(&mut self) -> Result<(), Error>;
}

impl<I2C: Instance,PINS> I2c<I2C,PINS> {
    /// Converts blocking [I2c] to non-blocking [I2CMasterDma] that use `tx_channel` and `rx_channel` to send/receive data
    pub fn use_dma<TX_CH: DMAChannel + CompatibleChannel<I2C, crate::dma::R>, RX_CH: DMAChannel + CompatibleChannel<I2C, crate::dma::W>>(
        self,
        tx_ch: TX_CH,
        rx_ch: RX_CH,
    ) -> I2CMasterDma<I2C, PINS, TxDMATransfer<I2C, TX_CH>, RxDMATransfer<I2C, RX_CH>>
    {
        let tx = TxDMATransfer::new(tx_ch);
        let rx = RxDMATransfer::new(rx_ch);

        I2CMasterDma {
            hal_i2c: self,

            address: 0,
            rx_len: 0,

            tx,
            rx,
            state: I2CMasterDmaState::Idle,
        }
    }

    /// Converts blocking [I2c] to non-blocking [I2CMasterDma] that use `tx_channel` to only send data
    pub fn use_dma_tx<TXCH>(
        self,
        txch: TXCH,
    ) -> I2CMasterDma<I2C, PINS, TxDMATransfer<I2C, TXCH>, NoDMA>
    where
        TXCH: DMAChannel + CompatibleChannel<I2C, crate::dma::R>,
        Tx<I2C>: TransferPayload,
    {
        let tx = TxDMATransfer::new(txch);
        let rx = NoDMA;

        I2CMasterDma {
            hal_i2c: self,

            address: 0,
            rx_len: 0,

            tx,
            rx,
            state: I2CMasterDmaState::Idle,
        }
    }

    /// Converts blocking [I2c] to non-blocking [I2CMasterDma] that use `rx_channel` to only receive data
    pub fn use_dma_rx<RXCH>(
        self,
        rx_channel: RXCH,
    ) -> I2CMasterDma<I2C, PINS, NoDMA, RxDMATransfer<I2C, RXCH>>
    where
        RXCH: DMAChannel + CompatibleChannel<I2C, crate::dma::W>,
        Rx<I2C>: TransferPayload,
    {
        let tx = NoDMA;
        let rx = RxDMATransfer::new(rx_channel);

        I2CMasterDma {
            hal_i2c: self,

            address: 0,
            rx_len: 0,

            tx,
            rx,
            state: I2CMasterDmaState::Idle,
        }
    }
}

#[allow(unused)]
#[derive(Copy, Clone)]
enum I2CMasterDmaState {
    Idle,
    Write,
    Read,
    WriteRead(usize, usize), // address for the read
}

/// # WARNING: EVERYTHING ASSOCIATED WITH I2C DMA IS BROKEN. THIS IS AN ACTIVE AREA OF RESEARCH AND WILL CHANGE RAPIDLY.
/// 
/// I2c abstraction that can work in non-blocking mode by using DMA
///
/// The struct should be used for sending/receiving bytes to/from slave device in non-blocking mode.
/// A client must follow these requirements to use that feature:
/// * Enable interrupts DMAx_STREAMy used for transmit and another DMAq_STREAMp used for receive.
/// * In these interrupts call [`handle_dma_interrupt`](Self::handle_dma_interrupt); defined in trait I2CMasterHandleIT
/// * Enable interrupts I2Cx_ER for handling errors and call [`handle_error_interrupt`](Self::handle_error_interrupt) in corresponding handler; defined in trait I2CMasterHandleIT
///
/// The struct can be also used to send/receive bytes in blocking mode with methods:
/// [`write`](Self::write()), [`read`](Self::read()), [`write_read`](Self::write_read()).
///
pub struct I2CMasterDma<I2C, PINS, TX_TRANSFER, RX_TRANSFER>
where
    I2C: Instance,
{
    hal_i2c: I2c<I2C,PINS>,

    state: I2CMasterDmaState,

    /// Last address used in `write_read_dma` method
    address: u8,
    /// Len of `buf` in `write_read_dma` method
    rx_len: usize,

    tx: TX_TRANSFER,
    rx: RX_TRANSFER,
}

/// trait for DMA transfer holder
pub trait DMATransfer<BUF> {
    /// Creates DMA Transfer using specified buffer
    fn create_transfer(&mut self, buf: BUF);
    /// Destroys created transfer
    /// # Panics
    ///   - If transfer had not created before
    fn destroy_transfer(&mut self);
    /// Checks if transfer created
    fn created(&self) -> bool;
}

// Mock implementations for NoDMA
// For Tx operations
impl DMATransfer<&'static [u8]> for NoDMA {
    fn create_transfer(&mut self, _: &'static [u8]) {
        unreachable!()
    }
    fn destroy_transfer(&mut self) {
        unreachable!()
    }
    fn created(&self) -> bool {
        false
    }
}
// ... and for Rx operations
impl DMATransfer<&'static mut [u8]> for NoDMA {
    fn create_transfer(&mut self, _: &'static mut [u8]) {
        unreachable!()
    }
    fn destroy_transfer(&mut self) {
        unreachable!()
    }
    fn created(&self) -> bool {
        false
    }
}


/// DMA Transfer holder for Tx operations
pub struct TxDMATransfer<I2C,  TXCH>
where
    I2C: Instance,
    TXCH: DMAChannel,
{
    _tx: Tx<I2C>,
    tx_channel: TXCH,
    tx_transfer: Option<()>,
}


impl<I2C, TXCH> TxDMATransfer<I2C, TXCH>
where
    I2C: Instance,
    TXCH : DMAChannel + CompatibleChannel<I2C,crate::dma::R>
{
    fn new(channel: TXCH) -> Self {
        Self {
            _tx: Tx { i2c: PhantomData },
            tx_channel: channel,
            tx_transfer: None,
        }
    }
}

impl<I2C, TX_CH> DMATransfer<&'static [u8]> for TxDMATransfer<I2C, TX_CH>
where
    I2C: Instance,
    TX_CH: DMAChannel + crate::dma::CompatibleChannel<I2C, crate::dma::R>,
{
    fn create_transfer(&mut self, buf: &'static [u8]) {
        assert!(self.tx_transfer.is_none());
        self.tx_channel.configure_channel();
        self.tx_channel.set_transfer_direction(crate::dma::TransferDirection::MemoryToPeripheral);
        self.tx_channel.set_peripheral_address(unsafe { (*I2C::ptr()).dat().as_ptr() as u32}, false);
        self.tx_channel.set_memory_address(buf.as_ptr() as u32, true);
        self.tx_channel.set_transfer_length(buf.len());
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);

        self.tx_channel.listen(crate::dma::Event::TransferComplete);
        self.tx_channel.listen(crate::dma::Event::TransferError);
        

        self.tx_transfer = Some(());
    }

    fn destroy_transfer(&mut self) {
        assert!(self.tx_transfer.is_some());
        self.tx_channel.stop();
        self.tx_transfer.take();
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Acquire);

    }

    fn created(&self) -> bool {
        self.tx_transfer.is_some()
    }
}

/// DMA Transfer holder for Rx operations
pub struct RxDMATransfer<I2C, RXCH>
where
    I2C: Instance,
    RXCH: DMAChannel + CompatibleChannel<I2C, crate::dma::W>,
{
    _rx: Rx<I2C>,
    rx_channel: RXCH,
    rx_transfer: Option<()>,
}

impl<I2C, RXCH> RxDMATransfer<I2C,  RXCH>
where
    I2C: Instance,
    RXCH: DMAChannel + CompatibleChannel<I2C, crate::dma::W>,
{
    fn new(channel: RXCH) -> Self {

        Self {
            _rx: Rx { i2c: PhantomData },
            rx_channel: channel,
            rx_transfer: None,
        }
    }
}

impl<I2C, RXCH> DMATransfer<&'static mut [u8]>
    for RxDMATransfer<I2C, RXCH>
where
    I2C: Instance,
    RXCH: DMAChannel + CompatibleChannel<I2C, crate::dma::W>,
    Rx<I2C>: TransferPayload,
{
    fn create_transfer(&mut self, buf: &'static mut [u8]) {
        assert!(self.rx_transfer.is_none());
        self.rx_channel.configure_channel();
        self.rx_channel.set_transfer_direction(crate::dma::TransferDirection::PeripheralToMemory);
        self.rx_channel.set_peripheral_address(unsafe { (*I2C::ptr()).dat().as_ptr() as u32}, false);
        self.rx_channel.set_memory_address(buf.as_ptr() as u32, true);
        self.rx_channel.set_transfer_length(buf.len());
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);

        self.rx_channel.listen(crate::dma::Event::TransferComplete);
        self.rx_channel.listen(crate::dma::Event::TransferError);


        self.rx_transfer = Some(());
    }

    fn destroy_transfer(&mut self) {
        assert!(self.rx_transfer.is_some());
        self.rx_channel.stop();
        self.rx_transfer.take();
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Acquire);
    }

    fn created(&self) -> bool {
        self.rx_transfer.is_some()
    }
}

/// Common implementation
impl<I2C, PINS, TX_TRANSFER, RX_TRANSFER> I2CMasterDma<I2C, PINS, TX_TRANSFER, RX_TRANSFER>
where
    I2C: Instance,
    TX_TRANSFER: DMATransfer<&'static [u8]>,
    RX_TRANSFER: DMATransfer<&'static mut [u8]>,
{

    /// Checks if there is communication in progress
    #[inline(always)]
    pub fn busy(&self) -> bool {
        self.hal_i2c.i2c.sts2().read().busy().bit_is_set()
    }

    /// Like `busy` but returns `WouldBlock` if busy
    fn busy_res(&self) -> nb::Result<(), super::Error> {
        if self.busy() {
            return nb::Result::Err(nb::Error::WouldBlock);
        }
        Ok(())
    }

    #[inline(always)]
    fn enable_dma_requests(&mut self) {
        self.hal_i2c.i2c.ctrl2().modify(|_, w| w.dmaen().set_bit());
    }

    #[inline(always)]
    fn disable_dma_requests(&mut self) {
        self.hal_i2c.i2c.ctrl2().modify(|_, w| w.dmaen().clear_bit());
    }

    #[inline(always)]
    fn enable_error_interrupt_generation(&mut self) {
        self.hal_i2c.i2c.ctrl2().modify(|_, w| w.errinten().set_bit());
    }

    #[inline(always)]
    fn disable_error_interrupt_generation(&mut self) {
        self.hal_i2c.i2c.ctrl2().modify(|_, w| w.errinten().clear_bit());
    }

    fn send_start(&mut self, read: bool) -> Result<(), super::Error> {
        let i2c = &self.hal_i2c.i2c;

        // Make sure the ack and start bit is set together in a single
        // read-modify-write operation to avoid race condition.
        // See PR: https://github.com/stm32-rs/stm32f4xx-hal/pull/662
        if read {
            i2c.ctrl1().modify(|_, w| w.acken().set_bit().startgen().set_bit());
        } else {
            i2c.ctrl1().modify(|_, w| w.startgen().set_bit());
        }

        // Wait until START condition was generated
        while self
            .hal_i2c
            .check_and_clear_error_flags()?
            .startbf()
            .bit_is_clear()
        {}

        // Also wait until signalled we're master and everything is waiting for us
        loop {
            self.hal_i2c.check_and_clear_error_flags()?;

            let sr2 = i2c.sts2().read();
            if !(sr2.msmode().bit_is_clear() && sr2.busy().bit_is_clear()) {
                break;
            }
        }

        Ok(())
    }

    fn send_stop(&mut self) {
        self.hal_i2c.i2c.ctrl1().modify(|_, w| w.stopgen().set_bit());
    }

    fn send_address(&mut self, addr: u8, read: bool) -> Result<(), super::Error> {
        let i2c = &self.hal_i2c.i2c;

        let mut to_send_addr = u32::from(addr) << 1;
        if read {
            to_send_addr += 1;
        }

        // Set up current address, we're trying to talk to
        i2c.dat().write(|w| unsafe { w.bits(to_send_addr) });

        // Wait until address was sent
        loop {
            // Check for any I2C errors. If a NACK occurs, the ADDR bit will never be set.
            let sr1 = self
                .hal_i2c
                .check_and_clear_error_flags()
                .map_err(super::Error::nack_addr)?;

            // Wait for the address to be acknowledged
            if sr1.addrf().bit_is_set() {
                break;
            }
        }

        Ok(())
    }

    fn prepare_write(&mut self, addr: u8) -> Result<(), super::Error> {
        // Start
        self.send_start(false)?;

        // Send address
        self.send_address(addr, false)?;

        // Clear condition by reading SR2. This will clear ADDR flag
        self.hal_i2c.i2c.sts2().read();

        // Enable error interrupts
        self.enable_error_interrupt_generation();

        Ok(())
    }

    /// Generates start and send address for read commands
    fn prepare_read(&mut self, addr: u8, buf_len: usize) -> Result<(), super::Error> {
        // Start
        self.send_start(true)?;

        // Send address
        self.send_address(addr, true)?;

        // Note from STM32 RM0090:
        // When the number of bytes to be received is equal to or greater than two,
        // the DMA controller sends a hardware signal, EOT_1, corresponding to the
        // last but one data byte (number_of_bytes – 1). If, in the I2C_CR2 register,
        // the LAST bit is set, I2C automatically sends a NACK after the next byte
        // following EOT_1. The user can generate a Stop condition in the DMA
        // Transfer Complete interrupt routine if enabled.
        // On small sized array we need to set ACK=0 before ADDR cleared
        if buf_len >= 2 {
            self.hal_i2c.i2c.ctrl2().modify(|_, w| w.dmalast().set_bit());
        // When a single byte must be received: the NACK must be programmed during
        // EV6 event, i.e. program ACK=0 when ADDR=1, before clearing ADDR flag.
        // Then the user can program the STOP condition either after clearing ADDR
        // flag, or in the DMA Transfer Complete interrupt routine.
        } else {
            self.hal_i2c.i2c.ctrl1().modify(|_, w| w.acken().clear_bit());
        }

        // Clear condition by reading SR2. This will clear ADDR flag
        self.hal_i2c.i2c.sts2().read();

        // Enable error interrupts
        self.enable_error_interrupt_generation();

        Ok(())
    }

    /// Reads in blocking mode but if i2c is busy returns `WouldBlock` and do nothing
    pub fn read(&mut self, addr: u8, buffer: &mut [u8]) -> nb::Result<(), super::Error> {
        self.busy_res()?;
        match self.hal_i2c.read(addr, buffer) {
            Ok(_) => Ok(()),
            Err(super::Error::NoAcknowledge(source)) => {
                self.send_stop();
                Err(nb::Error::Other(super::Error::NoAcknowledge(source)))
            }
            Err(error) => Err(nb::Error::Other(error)),
        }
    }

    /// Write and then read in blocking mode but if i2c is busy returns `WouldBlock` and do nothing
    pub fn write_read(
        &mut self,
        addr: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> nb::Result<(), super::Error> {
        self.busy_res()?;
        match self.hal_i2c.write_read(addr, bytes, buffer) {
            Ok(_) => Ok(()),
            Err(super::Error::NoAcknowledge(source)) => {
                self.send_stop();
                Err(nb::Error::Other(super::Error::NoAcknowledge(source)))
            }
            Err(error) => Err(nb::Error::Other(error)),
        }
    }

    /// Write in blocking mode but if i2c is busy returns `WouldBlock` and do nothing
    pub fn write(&mut self, addr: u8, bytes: &[u8]) -> nb::Result<(), super::Error> {
        self.busy_res()?;
        match self.hal_i2c.write(addr, bytes) {
            Ok(_) => Ok(()),
            Err(super::Error::NoAcknowledge(source)) => {
                self.send_stop();
                Err(nb::Error::Other(super::Error::NoAcknowledge(source)))
            }
            Err(error) => Err(nb::Error::Other(error)),
        }
    }

    fn finish_transfer_with_result(&mut self, result: Result<(), Error>) -> Result<(), Error> {
        self.disable_dma_requests();
        self.disable_error_interrupt_generation();
        self.hal_i2c.i2c.ctrl2().modify(|_, w| w.dmalast().clear_bit());

        if let Err(Error::I2CError(super::Error::NoAcknowledge(_))) = &result {
            self.send_stop();
        }

        if self.tx.created() {
            self.tx.destroy_transfer();
        }

        if self.rx.created() {
            self.rx.destroy_transfer();
        }
        result
    }
}

impl<I2C, PINS, TXCH> I2CMasterHandleIT
    for I2CMasterDma<I2C, PINS, TxDMATransfer<I2C, TXCH>, NoDMA>
where
    I2C: Instance,

    TXCH: DMAChannel + CompatibleChannel<I2C, crate::dma::R>,
    Tx<I2C>: TransferPayload,
{
    fn handle_dma_interrupt(&mut self) -> Result<ChannelStatus, Error> {
        if let Some(_) = &mut self.tx.tx_transfer {
            match self.tx.tx_channel.status() {
                ChannelStatus::TransferInProgress => Ok(ChannelStatus::TransferInProgress),
                ChannelStatus::TransferComplete => {
                    self.tx.tx_channel.clear_flag(crate::dma::Event::TransferComplete);

                    self.finish_transfer_with_result(Ok(())).ok();
    
                    // Wait for BTF
                    while self.hal_i2c.i2c.sts1().read().bytef().bit_is_clear() {}
    
                    self.send_stop();
                    self.state = I2CMasterDmaState::Idle;
                    Ok(ChannelStatus::TransferComplete)
    
                },
                ChannelStatus::TransferError => {
                    self.tx.tx_channel.clear_flag(crate::dma::Event::TransferError);
                    self.finish_transfer_with_result(Err(Error::TransferError)).ok();
                    self.state = I2CMasterDmaState::Idle;
                    Err(Error::TransferError)
                },
            }
        } else {
            Ok(ChannelStatus::TransferComplete) // TODO: is this what we semantically want?
        }
    }

    fn handle_error_interrupt(&mut self) -> Result<(), Error> {
        let res = self.hal_i2c.check_and_clear_error_flags();
        if let Err(e) = res {
            self.state = I2CMasterDmaState::Idle;
            self.finish_transfer_with_result(Err(Error::I2CError(e)))
        } else { Ok(()) }
    }
}

impl<I2C, PINS, RXCH> I2CMasterHandleIT
    for I2CMasterDma<I2C, PINS, NoDMA, RxDMATransfer<I2C, RXCH>>
where
    I2C: Instance,

    RXCH: DMAChannel + crate::dma::CompatibleChannel<I2C, crate::dma::W>,
    Rx<I2C>: TransferPayload,
{
    fn handle_dma_interrupt(&mut self) -> Result<ChannelStatus, Error> {

        if let Some(_) = &mut self.rx.rx_transfer {
            match self.rx.rx_channel.status() {
                ChannelStatus::TransferInProgress => Ok(ChannelStatus::TransferInProgress),
                ChannelStatus::TransferComplete => {
                    self.rx.rx_channel.clear_flag(crate::dma::Event::TransferComplete);

                    self.finish_transfer_with_result(Ok(())).ok();

                    // Clear ACK
                    self.hal_i2c.i2c.ctrl1().modify(|_, w| w.acken().clear_bit());
    
                    self.send_stop();
                    self.state = I2CMasterDmaState::Idle;
                    Ok(ChannelStatus::TransferComplete)
                },
                crate::dma::ChannelStatus::TransferError => {
                    self.rx.rx_channel.clear_flag(crate::dma::Event::TransferError);
                    self.state = I2CMasterDmaState::Idle;
                    self.finish_transfer_with_result(Err(Error::TransferError)).ok();
                    Err(Error::TransferError)
                },
            }
        } else {
            Ok(ChannelStatus::TransferComplete) // TODO: semantics good?
        }
    }

    fn handle_error_interrupt(&mut self) -> Result<(), Error> {
        let res = self.hal_i2c.check_and_clear_error_flags();
        if let Err(e) = res {
            self.state = I2CMasterDmaState::Idle;
            self.finish_transfer_with_result(Err(Error::I2CError(e)))
        } else { Ok(()) }
    }
}

/// Only for both TX and RX DMA I2c
impl<I2C, PINS, RXCH, TXCH> I2CMasterHandleIT
    for I2CMasterDma<I2C, PINS, TxDMATransfer<I2C, TXCH>, RxDMATransfer<I2C, RXCH>>
where
    I2C: Instance,
    TXCH: DMAChannel + crate::dma::CompatibleChannel<I2C, crate::dma::R>,
    Tx<I2C>: TransferPayload,

    RXCH: DMAChannel + crate::dma::CompatibleChannel<I2C, crate::dma::W>,
    Rx<I2C>: TransferPayload,
{
    fn handle_dma_interrupt(&mut self) -> Result<ChannelStatus, Error> {
        // Handle Transmit
        if let Some(_) = &mut self.tx.tx_transfer {
            let status = self.tx.tx_channel.status();
            match status {
                crate::dma::ChannelStatus::TransferInProgress => (),
                crate::dma::ChannelStatus::TransferComplete => {
                    self.tx.tx_channel.clear_flag(crate::dma::Event::TransferComplete);

                    // If we have prepared Rx Transfer, there are write_read command, generate restart signal and do not disable DMA requests
                    // Indicate that we have read after this transmit
                    let have_read_after = match self.state {
                        I2CMasterDmaState::WriteRead(ptr, len) => Some(unsafe { 
                            core::slice::from_raw_parts_mut(&mut *(ptr as *mut u8), len) 
                        }),
                        _ => None,
                    };
    
                    self.tx.destroy_transfer();
                    if have_read_after.is_none() {
                        self.finish_transfer_with_result(Ok(())).ok();
                        self.state = I2CMasterDmaState::Idle;
                    }
    
                    // Wait for BTF
                    while self.hal_i2c.i2c.sts1().read().bytef().bit_is_clear() {}
    
                    // If we have prepared Rx Transfer, there are write_read command, generate restart signal
                    if let Some(buf) = have_read_after {
                        self.rx.create_transfer(buf);
                        // Prepare for reading
                        if let Err(e) = self.prepare_read(self.address, self.rx_len) {
                            self.finish_transfer_with_result(Err(Error::I2CError(e)))?;
                        }
                        self.state = I2CMasterDmaState::Read;
    
                        self.rx.rx_channel.start();
                    } else {
                        self.send_stop();
                    }
    
                }
                ChannelStatus::TransferError => {
                    self.tx.tx_channel.clear_flag(crate::dma::Event::TransferError);
                    self.finish_transfer_with_result(Err(Error::TransferError))?;
                },
            };

            // If Transmit handled then receive should not be handled even if exists.
            // This return protects for handling Tx and Rx events in one interrupt.
            return Ok(ChannelStatus::TransferComplete);
        }

        if let Some(_) = &mut self.rx.rx_transfer {
            let status = self.rx.rx_channel.status();

            match status {
                crate::dma::ChannelStatus::TransferInProgress => (),
                crate::dma::ChannelStatus::TransferComplete => {
                    self.rx.rx_channel.clear_flag(crate::dma::Event::TransferComplete);

                    self.finish_transfer_with_result(Ok(())).ok();
    
                    // Clear ACK
                    self.hal_i2c.i2c.ctrl1().modify(|_, w| w.acken().clear_bit());
    
                    self.send_stop();
    
                },
                crate::dma::ChannelStatus::TransferError => {
                    self.rx.rx_channel.clear_flag(crate::dma::Event::TransferError);
                    self.finish_transfer_with_result(Err(Error::TransferError))?;

                },
            };
        }
        Ok(ChannelStatus::TransferComplete)
    }

    fn handle_error_interrupt(&mut self) -> Result<(), Error> {
        let res = self.hal_i2c.check_and_clear_error_flags();
        if let Err(e) = res {
            self.finish_transfer_with_result(Err(Error::I2CError(e)))
        } else { Ok(()) }
    }
}

// Write DMA implementations for TX only and TX/RX I2C DMA
impl<I2C, PINS, TXCH, RX_TRANSFER> I2CMasterWriteDMA
    for I2CMasterDma<I2C, PINS, TxDMATransfer<I2C, TXCH>, RX_TRANSFER>
where
    I2C: Instance,
    TXCH: DMAChannel + CompatibleChannel<I2C, crate::dma::R>,
    Tx<I2C>: TransferPayload,

    RX_TRANSFER: DMATransfer<&'static mut [u8]>,
{
    unsafe fn write_dma(
        &mut self,
        addr: u8,
        bytes: &[u8],
    ) -> nb::Result<(), super::Error> {
        self.busy_res()?;

        // Prepare transfer
        self.enable_dma_requests();
        let static_bytes: &'static [u8] = transmute(bytes);
        self.tx.create_transfer(static_bytes);

        if let Err(e) = self.prepare_write(addr) {
            // Reset struct on errors
            self.finish_transfer_with_result(Err(Error::I2CError(e))).map_err(|_| nb::Error::Other(e))?;
        }
        self.state = I2CMasterDmaState::Write;

        // Start DMA processing
        self.tx.tx_channel.start();

        Ok(())
    }
}

// Write DMA implementations for RX only and TX/RX I2C DMA
impl<I2C, PINS, TX_TRANSFER, RXCH> I2CMasterReadDMA
    for I2CMasterDma<I2C, PINS, TX_TRANSFER, RxDMATransfer<I2C, RXCH>>
where
    I2C: Instance,

    RXCH: DMAChannel + CompatibleChannel<I2C, crate::dma::W>,
    Rx<I2C>: TransferPayload,

    TX_TRANSFER: DMATransfer<&'static [u8]>,
{
    unsafe fn read_dma(
        &mut self,
        addr: u8,
        buf: &mut [u8],
    ) -> nb::Result<(), super::Error> {
        self.busy_res()?;

        //  If size is small we need to set ACK=0 before cleaning ADDR(reading SR2)
        let buf_len = buf.len();

        self.enable_dma_requests();
        let static_buf: &'static mut [u8] = transmute(buf);
        self.rx.create_transfer(static_buf);

        if let Err(e) = self.prepare_read(addr, buf_len) {
            // Reset struct on errors
            self.finish_transfer_with_result(Err(Error::I2CError(e))).map_err(|_| nb::Error::Other(e))?;
        }
        self.state = I2CMasterDmaState::Read;

        // Start DMA processing
        self.rx.rx_channel.start();

        Ok(())
    }
}

impl<I2C,PINS, TXCH, RXCH> I2CMasterWriteReadDMA
    for I2CMasterDma<I2C, PINS, TxDMATransfer<I2C, TXCH>, RxDMATransfer<I2C, RXCH>>
where
    I2C: Instance,
    TXCH: DMAChannel + CompatibleChannel<I2C, crate::dma::R>,
    Tx<I2C>: TransferPayload,

    RXCH: DMAChannel + CompatibleChannel<I2C, crate::dma::W>,
    Rx<I2C>: TransferPayload,
{
    unsafe fn write_read_dma(
        &mut self,
        addr: u8,
        bytes: &[u8],
        buf: &mut [u8],
    ) -> nb::Result<(), super::Error> {
        self.busy_res()?;

        self.address = addr;
        self.rx_len = buf.len();

        self.enable_dma_requests(); // enables i2c_ctrl2.dmaen
        let static_bytes: &'static [u8] = transmute(bytes);
        self.tx.create_transfer(static_bytes);

        // TODO: deal with
        //let static_buf: &'static mut [u8] = transmute(buf);
        //self.rx.create_transfer(static_buf);
        // this punts setting up the rx dma until after the tx dma completes
        self.state = I2CMasterDmaState::Write; //WriteRead(buf.as_ptr() as usize, buf.len());

        if let Err(e) = self.prepare_write(addr) {
            // Reset struct on errors
            self.finish_transfer_with_result(Err(Error::I2CError(e))).map_err(|_| nb::Error::Other(e))?;
        }

        // Start DMA processing
        self.tx.tx_channel.start();

        Ok(())
    }
}

pub struct Tx<I2C> {
    i2c: PhantomData<I2C>,
}

pub struct Rx<I2C> {
    i2c: PhantomData<I2C>,
}

impl<I2C> TransferPayload for Tx<I2C> 
where 
    I2C: Instance,
{
    fn start(&mut self) {}
    fn stop(&mut self) {}
}

impl<I2C> TransferPayload for Rx<I2C> 
where 
    I2C: Instance,
{
    fn start(&mut self) {}
    fn stop(&mut self) {}
}