//! # Dire`c`t Memory Access
#![allow(dead_code)]

use core::{
    marker::PhantomData, mem, ptr, sync::atomic::{self, compiler_fence, Ordering}
};
use embedded_dma::{ReadBuffer, WriteBuffer};

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    Overrun,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    HalfTransfer,
    TransferComplete,
    TransferError,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Half {
    First,
    Second,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferDirection {
    MemoryToMemory,
    MemoryToPeripheral,
    PeripheralToMemory,
}

pub struct CircBuffer<BUFFER, PAYLOAD>
where
    BUFFER: 'static,
{
    buffer: &'static mut [BUFFER; 2],
    payload: PAYLOAD,
    readable_half: Half,
}

impl<BUFFER, PAYLOAD> CircBuffer<BUFFER, PAYLOAD>
where
    &'static mut [BUFFER; 2]: WriteBuffer,
    BUFFER: 'static,
{
    pub(crate) fn new(buf: &'static mut [BUFFER; 2], payload: PAYLOAD) -> Self {
        CircBuffer {
            buffer: buf,
            payload,
            readable_half: Half::Second,
        }
    }
}

pub trait DmaExt {
    type Channels;

    fn split(self) -> Self::Channels;
}

pub trait TransferPayload {
    fn start(&mut self);
    fn stop(&mut self);
}

pub struct Transfer<MODE, BUFFER, PAYLOAD>
where
    PAYLOAD: TransferPayload,
{
    _mode: PhantomData<MODE>,
    buffer: BUFFER,
    payload: PAYLOAD,
}

impl<BUFFER, PAYLOAD> Transfer<R, BUFFER, PAYLOAD>
where
    PAYLOAD: TransferPayload,
{
    pub(crate) fn r(buffer: BUFFER, payload: PAYLOAD) -> Self {
        Transfer {
            _mode: PhantomData,
            buffer,
            payload,
        }
    }
}

impl<BUFFER, PAYLOAD> Transfer<W, BUFFER, PAYLOAD>
where
    PAYLOAD: TransferPayload,
{
    pub(crate) fn w(buffer: BUFFER, payload: PAYLOAD) -> Self {
        Transfer {
            _mode: PhantomData,
            buffer,
            payload,
        }
    }
}

impl<MODE, BUFFER, PAYLOAD> Drop for Transfer<MODE, BUFFER, PAYLOAD>
where
    PAYLOAD: TransferPayload,
{
    fn drop(&mut self) {
        self.payload.stop();
        compiler_fence(Ordering::SeqCst);
    }
}

/// Read transfer
pub struct R;

/// Write transfer
pub struct W;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChannelStatus {
    TransferInProgress,
    TransferComplete,
    TransferError,
}

pub trait DMAChannel {
    fn set_peripheral_address(&mut self, address: u32, inc: bool);
    fn set_memory_address(&mut self, address: u32, inc: bool);
    fn set_transfer_length(&mut self, len: usize);
    fn set_transfer_direction(&mut self, direction: TransferDirection);
    fn start(&mut self);
    fn stop(&mut self);
    fn in_progress(&self) -> bool;
    fn clear_flag(&mut self, event: Event);
    fn status(&self) -> ChannelStatus;
    fn listen(&mut self, event: Event);
    fn unlisten(&mut self, event: Event);
    fn st(&mut self) -> &crate::pac::dma1::St;
    fn intsts(&self) -> n32g4::raw::R<crate::pac::dma1::intsts::IntstsSpec>;
    fn intclr(&self) -> &crate::pac::dma1::Intclr;
    fn get_txnum(&self) -> u32;
}



impl<BUFFER, PAYLOAD, CX : DMAChannel> Transfer<W, BUFFER, RxDma<PAYLOAD, CX>>
where
    RxDma<PAYLOAD, CX>: TransferPayload,
{
    pub fn peek<T>(&self) -> &[T]
    where
        BUFFER: AsRef<[T]>,
    {
        let pending = self.payload.channel.get_txnum() as usize;

        let slice = self.buffer.as_ref();
        let capacity = slice.len();

        &slice[..(capacity - pending)]
    }
}

impl<RXBUFFER, TXBUFFER, PAYLOAD, CX : DMAChannel, TXC> Transfer<W, (RXBUFFER, TXBUFFER), RxTxDma<PAYLOAD, CX, TXC>>
where
    RxTxDma<PAYLOAD, CX, TXC>: TransferPayload,
{
    pub fn peek<T>(&self) -> &[T]
    where
        RXBUFFER: AsRef<[T]>,
    {
        let pending = self.payload.rxchannel.get_txnum() as usize;

        let slice = self.buffer.0.as_ref();
        let capacity = slice.len();

        &slice[..(capacity - pending)]
    }
}

impl<BUFFER, PAYLOAD, MODE, CX : DMAChannel, TXC> Transfer<MODE, BUFFER, RxTxDma<PAYLOAD, CX, TXC>>
where
    RxTxDma<PAYLOAD, CX, TXC>: TransferPayload,
{
    pub fn is_done(&self) -> bool {
        !self.payload.rxchannel.in_progress()
    }

    pub fn wait(mut self) -> (BUFFER, RxTxDma<PAYLOAD, CX, TXC>) {
        while !self.is_done() {}

        atomic::compiler_fence(Ordering::Acquire);

        self.payload.stop();

        // we need a read here to make the Acquire fence effective
        // we do *not* need this if `dma.stop` does a RMW operation
        unsafe { ptr::read_volatile(&0); }

        // we need a fence here for the same reason we need one in `Transfer.wait`
        atomic::compiler_fence(Ordering::Acquire);

        // `Transfer` needs to have a `Drop` implementation, because we accept
        // managed buffers that can free their memory on drop. Because of that
        // we can't move out of the `Transfer`'s fields, so we use `ptr::read`
        // and `mem::forget`.
        //
        // NOTE(unsafe) There is no panic branch between getting the resources
        // and forgetting `self`.
        unsafe {
            let buffer = ptr::read(&self.buffer);
            let payload = ptr::read(&self.payload);
            mem::forget(self);
            (buffer, payload)
        }
    }
}
impl<BUFFER, PAYLOAD, MODE,CX: DMAChannel> Transfer<MODE, BUFFER, RxDma<PAYLOAD, CX>>
where
    RxDma<PAYLOAD, CX>: TransferPayload,
{
    pub fn is_done(&self) -> bool {
        !self.payload.channel.in_progress()
    }

    pub fn wait(mut self) -> (BUFFER, RxDma<PAYLOAD, CX>) {
        while !self.is_done() {}

        atomic::compiler_fence(Ordering::Acquire);

        self.payload.stop();

        // we need a read here to make the Acquire fence effective
        // we do *not* need this if `dma.stop` does a RMW operation
        unsafe { ptr::read_volatile(&0); }

        // we need a fence here for the same reason we need one in `Transfer.wait`
        atomic::compiler_fence(Ordering::Acquire);

        // `Transfer` needs to have a `Drop` implementation, because we accept
        // managed buffers that can free their memory on drop. Because of that
        // we can't move out of the `Transfer`'s fields, so we use `ptr::read`
        // and `mem::forget`.
        //
        // NOTE(unsafe) There is no panic branch between getting the resources
        // and forgetting `self`.
        unsafe {
            let buffer = ptr::read(&self.buffer);
            let payload = ptr::read(&self.payload);
            mem::forget(self);
            (buffer, payload)
        }
    }
}

impl<BUFFER, PAYLOAD, MODE, CX: DMAChannel> Transfer<MODE, BUFFER, TxDma<PAYLOAD, CX>>
where
    TxDma<PAYLOAD, CX>: TransferPayload,
{
    pub fn is_done(&self) -> bool {
        !self.payload.channel.in_progress()
    }

    pub fn wait(mut self) -> (BUFFER, TxDma<PAYLOAD, CX>) {
        while !self.is_done() {}

        atomic::compiler_fence(Ordering::Acquire);

        self.payload.stop();

        // we need a read here to make the Acquire fence effective
        // we do *not* need this if `dma.stop` does a RMW operation
        unsafe { ptr::read_volatile(&0); }

        // we need a fence here for the same reason we need one in `Transfer.wait`
        atomic::compiler_fence(Ordering::Acquire);

        // `Transfer` needs to have a `Drop` implementation, because we accept
        // managed buffers that can free their memory on drop. Because of that
        // we can't move out of the `Transfer`'s fields, so we use `ptr::read`
        // and `mem::forget`.
        //
        // NOTE(unsafe) There is no panic branch between getting the resources
        // and forgetting `self`.
        unsafe {
            let buffer = ptr::read(&self.buffer);
            let payload = ptr::read(&self.payload);
            mem::forget(self);
            (buffer, payload)
        }
    }
}

macro_rules! dma {
    ($($DMAX:ident: ($dmaX:ident, {
        $($CX:ident: (
            $chX:ident,
            $htxfX:ident,
            $txcfX:ident,
            $errfX:ident,
            $chtxfX:ident,
            $ctxcfX:ident,
            $cglbfX:ident,
            $cerrfX:ident
        ),)+
    }),)+) => {
        $(
            pub mod $dmaX {
                use core::convert::TryFrom;

                use crate::pac::{Rcc, $DMAX, dma1};

                use crate::dma::{CircBuffer, DMAChannel, DmaExt, Error, Event, Half, RxDma, TransferPayload};
                use crate::rcc::Enable;

                #[allow(clippy::manual_non_exhaustive)]
                pub struct Channels((), $(pub $CX),+);

                $(
                    /// A singleton that represents a single DMAx channel (channel X in this case)
                    ///
                    /// This singleton has exclusive access to the registers of the DMAx channel X
                    pub struct $CX { _0: () }

                    impl DMAChannel for $CX {
                        /// Associated peripheral `address`
                        ///
                        /// `inc` indicates whether the address will be incremented after every byte transfer
                        fn set_peripheral_address(&mut self, address: u32, inc: bool) {
                            self.st().paddr().write(|w| unsafe { w.addr().bits(address) } );
                            self.st().chcfg().modify(|_, w| w.pinc().bit(inc) );
                        }

                        fn clear_flag(&mut self, event : crate::dma::Event) { 
                            match event {
                                crate::dma::Event::HalfTransfer => {
                                    self.intclr().modify(|_,w| w.$chtxfX().clear_bit())
                                },
                                crate::dma::Event::TransferComplete => {
                                    self.intclr().modify(|_,w| w.$ctxcfX().clear_bit())
                                },
                                crate::dma::Event::TransferError => {
                                    self.intclr().modify(|_,w| w.$cerrfX().clear_bit())
                                }
                            }
                        }

                        fn set_transfer_direction(&mut self, direction: crate::dma::TransferDirection) {
                            match direction {
                                crate::dma::TransferDirection::MemoryToMemory => {
                                    self.st().chcfg().modify(|_,w| w.mem2mem().set_bit())
                                },
                                crate::dma::TransferDirection::MemoryToPeripheral => {
                                    self.st().chcfg().modify(|_,w| w.mem2mem().clear_bit().dir().set_bit())
                                },
                                crate::dma::TransferDirection::PeripheralToMemory => {
                                    self.st().chcfg().modify(|_,w| w.mem2mem().clear_bit().dir().clear_bit())
                                },
                            }
                        }

                        fn status(&self) -> crate::dma::ChannelStatus {
                            if self.intsts().$errfX().bit_is_set() {
                                return crate::dma::ChannelStatus::TransferError;
                            }
                            if self.intsts().$txcfX().bit_is_set() {
                                return crate::dma::ChannelStatus::TransferComplete;
                            }
                            return crate::dma::ChannelStatus::TransferInProgress;
                        } 

                        /// `address` where from/to data will be read/write
                        ///
                        /// `inc` indicates whether the address will be incremented after every byte transfer
                        fn set_memory_address(&mut self, address: u32, inc: bool) {
                            self.st().maddr().write(|w| unsafe { w.addr().bits(address) } );
                            self.st().chcfg().modify(|_, w| w.minc().bit(inc) );
                        }

                        /// Number of bytes to transfer
                        fn set_transfer_length(&mut self, len: usize) {
                            self.st().txnum().write(|w| unsafe { w.ndtx().bits(u16::try_from(len).unwrap()) });
                        }

                        /// Starts the DMA transfer
                        fn start(&mut self) {
                            self.st().paddr().modify(|r,w| unsafe { w.addr().bits(r.addr().bits()) });
                            self.st().maddr().modify(|r,w| unsafe { w.addr().bits(r.addr().bits()) });
                            self.st().chcfg().modify(|_, w| w.chen().set_bit() );
                        }

                        /// Stops the DMA transfer
                        fn stop(&mut self) {
                            self.intclr().write(|w| w.$cglbfX().set_bit());
                            self.st().chcfg().modify(|_, w| w.chen().clear_bit() );
                        }

                        /// Returns `true` if there's a transfer in progress
                        fn in_progress(&self) -> bool {
                            self.intsts().$txcfX().bit_is_clear()
                        }

                        fn listen(&mut self, event: Event) {
                            match event {
                                Event::HalfTransfer => self.st().chcfg().modify(|_, w| w.htxie().set_bit()),
                                Event::TransferComplete => {
                                    self.st().chcfg().modify(|_, w| w.txcie().set_bit())
                                },
                                Event::TransferError => {
                                    self.st().chcfg().modify(|_, w| w.errie().set_bit())
                                }
                            }
                        }

                        fn unlisten(&mut self, event: Event) {
                            match event {
                                Event::HalfTransfer => {
                                    self.st().chcfg().modify(|_, w| w.htxie().clear_bit())
                                },
                                Event::TransferComplete => {
                                    self.st().chcfg().modify(|_, w| w.txcie().clear_bit())
                                },
                                Event::TransferError => {
                                    self.st().chcfg().modify(|_, w| w.errie().clear_bit())
                                }
                            }
                        }

                        fn st(&mut self) -> &dma1::St {
                            unsafe { &(*$DMAX::ptr()).$chX() }
                        }

                        fn intsts(&self) -> n32g4::raw::R<dma1::intsts::IntstsSpec> {
                            // NOTE(unsafe) atomic read with no side effects
                            unsafe { (*$DMAX::ptr()).intsts().read() }
                        }

                        fn intclr(&self) -> &dma1::Intclr {
                            unsafe { &(*$DMAX::ptr()).intclr() }
                        }

                        fn get_txnum(&self) -> u32 {
                            // NOTE(unsafe) atomic read with no side effects
                            unsafe { &(*$DMAX::ptr())}.$chX().txnum().read().bits()
                        }
                    }
                    impl<B, PAYLOAD> CircBuffer<B, RxDma<PAYLOAD, $CX>>
                    where
                        RxDma<PAYLOAD, $CX>: TransferPayload,
                    {
                        
                        /// Peeks into the readable half of the buffer
                        pub fn peek<R, F>(&mut self, f: F) -> Result<R, Error>
                            where
                            F: FnOnce(&B, Half) -> R,
                        {
                            let half_being_read = self.readable_half()?;

                            let buf = match half_being_read {
                                Half::First => &self.buffer[0],
                                Half::Second => &self.buffer[1],
                            };

                            // XXX does this need a compiler barrier?
                            let ret = f(buf, half_being_read);


                            let isr = self.payload.channel.intsts();
                            let first_half_is_done = isr.$htxfX().bit_is_set();
                            let second_half_is_done = isr.$txcfX().bit_is_set();

                            if (half_being_read == Half::First && second_half_is_done) ||
                                (half_being_read == Half::Second && first_half_is_done) {
                                Err(Error::Overrun)
                            } else {
                                Ok(ret)
                            }
                        }

                        /// Returns the `Half` of the buffer that can be read
                        pub fn readable_half(&mut self) -> Result<Half, Error> {
                            let isr = self.payload.channel.intsts();
                            let first_half_is_done = isr.$htxfX().bit_is_set();
                            let second_half_is_done = isr.$txcfX().bit_is_set();

                            if first_half_is_done && second_half_is_done {
                                return Err(Error::Overrun);
                            }

                            let last_read_half = self.readable_half;

                            Ok(match last_read_half {
                                Half::First => {
                                    if second_half_is_done {
                                        self.payload.channel.intclr().write(|w| w.$ctxcfX().set_bit());

                                        self.readable_half = Half::Second;
                                        Half::Second
                                    } else {
                                        last_read_half
                                    }
                                }
                                Half::Second => {
                                    if first_half_is_done {
                                        self.payload.channel.intclr().write(|w| w.$chtxfX().set_bit());

                                        self.readable_half = Half::First;
                                        Half::First
                                    } else {
                                        last_read_half
                                    }
                                }
                            })
                        }

                        /// Stops the transfer and returns the underlying buffer and RxDma
                        pub fn stop(mut self) -> (&'static mut [B; 2], RxDma<PAYLOAD, $CX>) {
                            self.payload.stop();

                            (self.buffer, self.payload)
                        }
                    }

                    
                )+

                impl DmaExt for $DMAX {
                    type Channels = Channels;

                    fn split(self) -> Channels {
                        let rcc = unsafe { &(*Rcc::ptr()) };
                        $DMAX::enable(rcc);
                        unsafe { (*$DMAX::ptr()).chmapen().modify(|_,w| w.map_en().set_bit()); }
                        // reset the DMA control registers (stops all on-going transfers)
                        $(
                            self.$chX().chcfg().reset();
                        )+

                        Channels((), $($CX { _0: () }),+)
                    }
                }
            }
        )+
    }
}

dma! {
    Dma1: (dma1, {
        C1: (
            st1,
            htxf1, txcf1, errf1,
            chtxf1, ctxcf1, cglbf1, cerrf1
        ),
        C2: (
            st2,
            htxf2, txcf2, errf2,
            chtxf2, ctxcf2, cglbf2, cerrf2
        ),
        C3: (
            st3,
            htxf3, txcf3, errf3,
            chtxf3, ctxcf3, cglbf3, cerrf3
        ),
        C4: (
            st4,
            htxf4, txcf4, errf4,
            chtxf4, ctxcf4, cglbf4, cerrf4
        ),
        C5: (
            st5,
            htxf5, txcf5, errf5,
            chtxf5, ctxcf5, cglbf5, cerrf5
        ),
        C6: (
            st6,
            htxf6, txcf6, errf6,
            chtxf6, ctxcf6, cglbf6, cerrf6
        ),
        C7: (
            st7,
            htxf7, txcf7, errf7,
            chtxf7, ctxcf7, cglbf7, cerrf7
        ),
        C8: (
            st8,
            htxf8, txcf8, errf8,
            chtxf8, ctxcf8, cglbf8, cerrf8
        ),
    }),

    Dma2: (dma2, {
        C1: (
            st1,
            htxf1, txcf1, errf1,
            chtxf1, ctxcf1, cglbf1, cerrf1
        ),
        C2: (
            st2,
            htxf2, txcf2, errf2,
            chtxf2, ctxcf2, cglbf2, cerrf2
        ),
        C3: (
            st3,
            htxf3, txcf3, errf3,
            chtxf3, ctxcf3, cglbf3, cerrf3
        ),
        C4: (
            st4,
            htxf4, txcf4, errf4,
            chtxf4, ctxcf4, cglbf4, cerrf4
        ),
        C5: (
            st5,
            htxf5, txcf5, errf5,
            chtxf5, ctxcf5, cglbf5, cerrf5
        ),
        C6: (
            st6,
            htxf6, txcf6, errf6,
            chtxf6, ctxcf6, cglbf6, cerrf6
        ),
        C7: (
            st7,
            htxf7, txcf7, errf7,
            chtxf7, ctxcf7, cglbf7, cerrf7
        ),
        C8: (
            st8,
            htxf8, txcf8, errf8,
            chtxf8, ctxcf8, cglbf8, cerrf8
        ),
    }),
}

/// DMA Receiver
pub struct RxDma<PAYLOAD, RXCH> {
    pub(crate) payload: PAYLOAD,
    pub channel: RXCH,
}

/// DMA Transmitter
pub struct TxDma<PAYLOAD, TXCH> {
    pub(crate) payload: PAYLOAD,
    pub channel: TXCH,
}

/// DMA Receiver/Transmitter
pub struct RxTxDma<PAYLOAD, RXCH, TXCH> {
    pub(crate) payload: PAYLOAD,
    pub rxchannel: RXCH,
    pub txchannel: TXCH,
}

pub trait Receive {
    type RxChannel;
    type TransmittedWord;
}

pub trait Transmit {
    type TxChannel;
    type ReceivedWord;
}

/// Trait for circular DMA readings from peripheral to memory.
pub trait CircReadDma<B, RS>: Receive
where
    &'static mut [B; 2]: WriteBuffer<Word = RS>,
    B: 'static,
    Self: core::marker::Sized,
{
    fn circ_read(self, buffer: &'static mut [B; 2]) -> CircBuffer<B, Self>;
}

/// Trait for DMA readings from peripheral to memory.
pub trait ReadDma<B, RS>: Receive
where
    B: WriteBuffer<Word = RS>,
    Self: core::marker::Sized + TransferPayload,
{
    fn read(self, buffer: B) -> Transfer<W, B, Self>;
}

/// Trait for DMA writing from memory to peripheral.
pub trait WriteDma<B, TS>: Transmit
where
    B: ReadBuffer<Word = TS>,
    Self: core::marker::Sized + TransferPayload,
{
    fn write(self, buffer: B) -> Transfer<R, B, Self>;
}

/// Trait for DMA simultaneously reading and writing within one synchronous operation. Panics if both buffers are not of equal length.
pub trait ReadWriteDma<RXB, TXB, TS>: Transmit
where
    RXB: WriteBuffer<Word = TS>,
    TXB: ReadBuffer<Word = TS>,
    Self: core::marker::Sized + TransferPayload,
{
    fn read_write(self, rx_buffer: RXB, tx_buffer: TXB) -> Transfer<W, (RXB, TXB), Self>;
}

pub trait DMAMode {}
impl DMAMode for R {}
impl DMAMode for W {}
pub trait CompatibleChannel<PERIPH,MODE> : DMAChannel
where MODE : DMAMode {
    fn configure_channel(&mut self);
}

#[cfg(any(feature="n32g451",feature="n32g452",feature="n32g455",feature="n32g457",feature="n32g4fr"))]
pub mod chmap;