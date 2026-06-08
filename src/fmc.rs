use crate::pac::{flash, Flash as Fmc};
use embedded_storage::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};

pub trait FMCExt {
    /// Constrains the FLASH peripheral to play nicely with the other abstractions
    fn constrain(self) -> Flash;
}

impl FMCExt for Fmc {
    fn constrain(self) -> Flash {
        Flash
    }
}

pub struct Flash;

impl Flash {
    const FLASH_BASE: u32 = 0x0800_0000;
    fn max_addr() -> u32 {
        Flash::FLASH_BASE + (Flash::capacity() as u32) - 1
    }

    // calculates the capacity from the Dbg register
    fn capacity() -> usize {
        let dbg = unsafe { crate::pac::Dbg::steal() };
        (dbg.dbg_id().read().flash().bits() as usize) << 16
    }



    const READ_SIZE: usize = 0x1;
    const WRITE_SIZE: usize = 0x4;
    const ERASE_SIZE: usize = 2048;

    fn unlock(&mut self) {
        let fmc: &flash::RegisterBlock = unsafe { &(*Fmc::ptr()) };
        if fmc.ctrl().read().lock().bit_is_set() {
            fmc.key().write(|w| unsafe{w.bits(0x45670123)});
            fmc.key().write(|w| unsafe{w.bits(0xCDEF89AB)});
        }
    }

    fn lock(&mut self) {
        let fmc: &flash::RegisterBlock = unsafe { &(*Fmc::ptr()) };
        if fmc.ctrl().read().lock().bit_is_clear() {
            fmc.ctrl().modify(|_, w| w.lock().set_bit());
        }
    }

    fn program_word(&mut self, offset: u32, word: u32) {
        let fmc: &flash::RegisterBlock = unsafe { &(*Fmc::ptr()) };
        while fmc.sts().read().busy().bit_is_set() {}
        fmc.ctrl().modify(|_, w| w.pg().set_bit());
        let write_ptr = unsafe { core::mem::transmute::<usize,*mut u32>((Flash::FLASH_BASE + offset) as usize) };
        unsafe { core::ptr::write_volatile(write_ptr, word); }
        while fmc.sts().read().busy().bit_is_set() {}
        fmc.ctrl().modify(|_, w| w.pg().clear_bit());
    }

    fn erase_page(&mut self, offset: u32) {
        let fmc: &flash::RegisterBlock = unsafe { &(*Fmc::ptr()) };
        while fmc.sts().read().busy().bit_is_set() {}
        let erase_addr = Flash::FLASH_BASE + offset;

        fmc.ctrl().modify(|_, w| w.per().set_bit());
        unsafe { fmc.addr().write(|w| w.fadd().bits(erase_addr)); }
        fmc.ctrl().modify(|_, w| w.start().set_bit());
        cortex_m::asm::dsb();
        cortex_m::asm::isb();
        while fmc.sts().read().busy().bit_is_set() {}
        fmc.ctrl().modify(|_, w| w.per().clear_bit());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlashError {
    WriteProtected,
    ProgramError,
    OutOfBounds,
    NotAligned,
}

impl NorFlashError for FlashError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            FlashError::OutOfBounds => NorFlashErrorKind::OutOfBounds,
            FlashError::NotAligned => NorFlashErrorKind::NotAligned,
            FlashError::WriteProtected => NorFlashErrorKind::Other,
            FlashError::ProgramError => NorFlashErrorKind::Other,
        }
    }
}

impl ErrorType for Flash {
    type Error = FlashError;
}

impl ReadNorFlash for Flash {
    const READ_SIZE: usize = Flash::READ_SIZE;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let mut addr = Flash::FLASH_BASE + offset;
        if addr > Flash::max_addr() || (addr + bytes.len() as u32) > (Flash::max_addr() + 1) {
            return Err(FlashError::OutOfBounds);
        }

        // this actually produces Reasonable Assembly somehow.
        // we use read_volatile because eliding these calls can lead to Odd Behavior when dealing with pointers/offsets
        // derived from consts/statics that happen to live in flash; the compiler may assume that they are immutable
        // even if the underlying data is mutated.

        let mut biter = bytes.chunks_exact_mut(4);
        for b4 in &mut biter {
            let bbuf = unsafe { core::ptr::read_volatile(addr as *const u32) }.to_ne_bytes();
            b4[0] = bbuf[0];
            b4[1] = bbuf[1];
            b4[2] = bbuf[2];
            b4[3] = bbuf[3];
            addr += 4;
        }
        for b in biter.into_remainder() {
            *b = unsafe { core::ptr::read_volatile(addr as *const u8) };
            addr += 1;
        }

        Ok(())
    }

    fn capacity(&self) -> usize {
        Flash::capacity()
    }
}

impl NorFlash for Flash
{
    const WRITE_SIZE: usize = Flash::WRITE_SIZE;
    const ERASE_SIZE: usize = Flash::ERASE_SIZE;


    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        if from >= Flash::max_addr() {
            return Err(Self::Error::OutOfBounds);
        }

        if to > (Flash::max_addr() + 1) {
            return Err(Self::Error::OutOfBounds);
        }

        if from % Self::ERASE_SIZE as u32 != 0 || to % Self::ERASE_SIZE as u32 != 0 {
            return Err(Self::Error::NotAligned);
        }
        self.unlock();

        let range = (from / Self::ERASE_SIZE as u32)..(to / Self::ERASE_SIZE as u32);
        for page in range {
            self.erase_page(page * (Self::ERASE_SIZE as u32));
        }
        self.lock();
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        if bytes.len() % Self::WRITE_SIZE != 0 {
            return Err(Self::Error::NotAligned);
        }

        if offset as usize % Self::WRITE_SIZE != 0 {
            return Err(Self::Error::NotAligned);
        }

        if (offset as usize) + bytes.len() > Flash::capacity() {
            return Err(Self::Error::OutOfBounds);
        }

        self.unlock();

        // WRITE_SIZE is always 4 so the chunks will never have a remainder.
        let mut byte_chunks = bytes.chunks_exact(4);
        let mut i = 0u32;
        for b in byte_chunks.by_ref() {
            self.program_word(offset + i, u32::from_ne_bytes(b.try_into().unwrap()));
            i += 4;
        }

        self.lock();
        Ok(())
    }
}

impl embedded_storage_async::nor_flash::NorFlash for Flash {

    // while theoretically possible to async wait on the fmc.stat() register combined with the fmc interrupt,
    // it's unknown if it's worth doing.
    // so for now we just provide the sync impls.

    const WRITE_SIZE: usize = Flash::WRITE_SIZE;
    const ERASE_SIZE: usize = Flash::ERASE_SIZE;


    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        embedded_storage::nor_flash::NorFlash::erase(self, from, to)

    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        embedded_storage::nor_flash::NorFlash::write(self, offset, bytes)
    }

}

impl embedded_storage_async::nor_flash::ReadNorFlash for Flash {
    const READ_SIZE: usize = Flash::READ_SIZE;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        embedded_storage::nor_flash::ReadNorFlash::read(self, offset, bytes)
    }

    fn capacity(&self) -> usize {
        Flash::capacity()
    }
}