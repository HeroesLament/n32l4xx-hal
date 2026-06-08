/*
CRC registers.

*/

use core::mem::MaybeUninit;
use core::ptr::copy_nonoverlapping;

use crate::pac::{Crc,Rcc};
use crate::rcc::{Enable,Reset};

pub trait CrcExt {
    fn constrain(self) -> CrcEngine;
}

pub struct CrcEngine {
    pub(crate) regs: Crc,
}

pub struct Crc16Engine {
    pub(crate) regs: Crc,
}

pub struct Crc32Engine {
    pub(crate) regs: Crc,
}

pub struct Crc32Stream {
    engine: Crc32Engine
}

#[derive(Clone, Copy)]
pub struct Crc16State {
    pub value: u16,
    pub endianness: CrcEndianness
}

#[derive(Clone, Copy)]
pub enum CrcEndianness {
    StartFromMsb,
    StartFromLsb
}

impl Crc16State {
    pub fn new(endianness: CrcEndianness) -> Self {
        Self {
            value: 0,
            endianness: endianness
        }
    }
    pub fn new_le() -> Self {
        Self {
            value: 0,
            endianness: CrcEndianness::StartFromLsb,
        }
    }
    pub fn new_be() -> Self {
        Self {
            value: 0,
            endianness: CrcEndianness::StartFromMsb,
        }
    }
}

impl CrcExt for Crc {
    fn constrain(self) -> CrcEngine {
        let rcc = {unsafe {&(*Rcc::ptr())}};
        Crc::enable(rcc);
        Crc::reset(rcc);
        CrcEngine{regs: self}
    }
}


impl CrcEngine {
    /// Splits this CrcEngine into a Crc16 and Crc32 engine that can act independently.
    /// two of them.
    pub fn split(self) -> (Crc16Engine, Crc32Engine) {
        let crc = {unsafe {Crc::steal()}};
        (
            Crc16Engine {regs: crc},
            Crc32Engine {regs: self.regs}
        )
    }

    /// Computes a CRC32 on the given u32 slice.
    /// This produces a big-endian CRC.
    pub fn crc32(&mut self, data: &[u32]) -> u32 {
        self.regs.crc32ctrl().write(|w| w.reset().set_bit());
        for word in data {
            self.regs.crc32dat().write(|w| unsafe {w.crc32dat().bits(*word)});
        }
        self.regs.crc32dat().read().crc32dat().bits()
    }

    /// Compute a CRC16 on the given u16 slice.
    /// iv: initial value
    /// endianness: which way to consume the data
    /// data: data to consume
    pub fn crc16(&mut self, state: Crc16State, data: &[u8]) -> Crc16State {
        match state.endianness {
            CrcEndianness::StartFromMsb => self.regs.crc16ctrl().write(|w| w.endhl().clear_bit()),
            CrcEndianness::StartFromLsb =>  self.regs.crc16ctrl().write(|w| w.endhl().set_bit())
        };
        self.regs.crc16d().write(|w| unsafe {w.crc16d().bits(state.value)});
        for word in data {
            self.regs.crc16dat().write(|w| unsafe {w.crc16dat().bits(*word)});
        }        
        Crc16State {
            value: self.regs.crc16d().read().crc16d().bits(),
            endianness: state.endianness
        }
    }
}

impl Crc32Engine {
    /// stream a crc32 so you don't have to compute it all at once
    pub fn stream(self) -> Crc32Stream {
        self.regs.crc32ctrl().write(|w| w.reset().set_bit());
        Crc32Stream { engine: self }
    }

    /// Computes a CRC32 on the given u32 slice.
    /// This produces a big-endian CRC.
    pub fn crc32(&mut self, data: &[u32]) -> u32 {
        self.regs.crc32ctrl().write(|w| w.reset().set_bit());
        for word in data {
            self.regs.crc32dat().write(|w| unsafe {w.crc32dat().bits(*word)});
        }
        self.regs.crc32dat().read().crc32dat().bits()
    }

    pub fn init(&mut self) {
        self.regs.crc32ctrl().write(|w| w.reset().set_bit());
    }

    pub fn update(&mut self, data: &[u32]) -> u32 {
        for word in data {
            self.regs.crc32dat().write(|w| unsafe {
                w.crc32dat().bits(*word)
            });
        }
        self.regs.crc32dat().read().bits()
    }

    pub fn update_bytes(&mut self, data: &[u8]) -> u32 {

        let chunks = data.chunks_exact(4);
        let remainder = chunks.remainder();

                // For each full chunk of four bytes...
        chunks.for_each(|chunk| unsafe {
            // Create an uninitialized scratch buffer. We make it uninitialized
            // to avoid re-zeroing this data inside of the loop.
            let mut scratch: MaybeUninit<[u8; 4]> = MaybeUninit::uninit();

            // Copy the (potentially unaligned) bytes from the input chunk to
            // our scratch bytes. We cast the `scratch` buffer from a `*mut [u8; 4]`
            // to a `*mut u8`.
            let src: *const u8 = chunk.as_ptr();
            let dst: *mut u8 = scratch.as_mut_ptr().cast::<u8>();
            copy_nonoverlapping(src, dst, 4);

            // Mark the scratch bytes as initialized, and then convert it to a
            // native-endian u32. Feed this into the CRC peripheral
            self.regs.crc32dat().write(|w| w.bits(u32::from_be_bytes(scratch.assume_init())));
        });
        // If we had a non-multiple of four bytes...
        if !remainder.is_empty() {
            // Create a zero-filled scratch buffer, and copy the data in
            let mut scratch = [0u8; 4];

            // NOTE: We are on a little-endian processor. This means that copying
            // the 0..len range fills the LEAST significant bytes, leaving the
            // MOST significant bytes as zeroes
            scratch[..remainder.len()].copy_from_slice(remainder);
            self.regs.crc32dat().write(|w| unsafe {w.bits(u32::from_be_bytes(scratch))});
        }

        self.regs.crc32dat().read().bits()

    }

}

impl Crc32Stream {
    /// update the crc32 hardware register with new data
    pub fn update(&mut self, data: &[u32]) {
        for word in data {
            self.engine.regs.crc32dat().write(|w| unsafe {w.crc32dat().bits(*word)});
        }
    }
    /// read the current crc32 hash value
    pub fn value(&self) -> u32 {
        self.engine.regs.crc32dat().read().crc32dat().bits()
    }
    /// release the engine for use elsewhere
    pub fn finalize(self) -> Crc32Engine {
        self.engine
    }
}

impl Crc16Engine {
    /// Compute a CRC16 on the given u16 slice.
    /// States allow one to multiplex the periph between different subsystems.
    /// state: a Crc16State with an iv and endianness
    /// data: data to consume
    pub fn crc16(&mut self, state: Crc16State, data: &[u8]) -> Crc16State {
        match state.endianness {
            CrcEndianness::StartFromMsb => self.regs.crc16ctrl().write(|w| w.endhl().clear_bit()),
            CrcEndianness::StartFromLsb =>  self.regs.crc16ctrl().write(|w| w.endhl().set_bit())
        };
        self.regs.crc16d().write(|w| unsafe {w.crc16d().bits(state.value)});
        for word in data {
            self.regs.crc16dat().write(|w| unsafe {w.crc16dat().bits(*word)});
        }        
        Crc16State {
            value: self.regs.crc16d().read().crc16d().bits(),
            endianness: state.endianness
        }
    }
}