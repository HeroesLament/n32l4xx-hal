use crate::pac::{Pwr,Rcc};
use crate::rcc::{Enable,Reset};
pub trait PwrExt {
    fn constrain(self) -> Pwr;
}
#[cfg(any(feature="n32g451",feature="n32g452",feature="n32g455",feature="n32g457",feature="n32g4fr"))]
impl PwrExt for Pwr {
    fn constrain(self) -> Pwr {
        let rcc = unsafe { &(*Rcc::ptr()) };
        Pwr::enable(rcc);
        Pwr::reset(rcc);
        self
    }
}