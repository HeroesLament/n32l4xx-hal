#![allow(unused)]

// TODO: unimplemented

use crate::pac::Sac;
use crate::rcc::Reset;
use crate::pac::Rcc;
use crate::rcc::Enable;
pub struct CryptoEngine {
    pub(crate) regs: Sac,
}

impl CryptoEngine {
    pub fn new(regs: Sac) -> Self {
        Self { regs }
    }
    pub fn enable(&self) {
        let rcc = unsafe { &(*Rcc::ptr()) };
        Sac::enable(&rcc);
    }

    pub fn init(&self) {
        let rcc = unsafe { &(*Rcc::ptr()) };
        Sac::disable(&rcc);
        Sac::enable(&rcc);
        Sac::reset(&rcc); 
    }

    pub fn init_with_trng(&self) {
        let rcc = unsafe { &(*Rcc::ptr()) };
        rcc.cfg3().modify(|_,w| w.trng1men().set_bit());
        rcc.ahbpclken().modify(|_,w| w.sacen().clear_bit().rngcen().clear_bit());
        rcc.ahbpclken().modify(|_,w| w.sacen().set_bit().rngcen().set_bit());
        rcc.ahbprst().modify(|_,w| w.sacrst().set_bit().rngcrst().set_bit());
        rcc.ahbprst().modify(|_,w| w.sacrst().clear_bit().rngcrst().clear_bit());

    }
    pub fn reset(&self) {
        let rcc = unsafe { &(*Rcc::ptr()) };
        Sac::reset(&rcc);
    }
}
pub mod hash;
pub mod trng;
pub mod aes;
// pub mod des;