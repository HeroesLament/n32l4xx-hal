//! # Alternate Function I/Os

use crate::pac::{afio, Afio, Rcc};

use crate::rcc::{Enable, Reset};


pub trait AfioExt {
    fn constrain(self) -> Afio;
}
#[cfg(any(feature="n32g451",feature="n32g452",feature="n32g455",feature="n32g457",feature="n32g4fr"))]
impl AfioExt for Afio {
    fn constrain(self) -> Afio {
        let rcc = unsafe { &(*Rcc::ptr()) };
        Afio::enable(rcc);
        Afio::reset(rcc);
        self
        // Parts {
        //     ectrl: ECTRL { _0: () },
        //     rmp_cfg: RMP_CFG { _0: () },
        //     exticfg1: EXTI_CFG1 { _0: () },
        //     exticfg2: EXTI_CFG2 { _0: () },
        //     exticfg3: EXTI_CFG3 { _0: () },
        //     exticfg4: EXTI_CFG4 { _0: () },
        //     rmp_cfg3: RMP_CFG3 { _0: () },
        //     rmp_cfg4: RMP_CFG4 { _0: () },
        //     rmp_cfg5: RMP_CFG5 { _0: () },
        // }
    }
}

#[cfg(any(feature="n32g432",feature="n32g435"))]
impl AfioExt for Afio {
    fn constrain(self) -> Parts {
        let rcc = unsafe { &(*Rcc::ptr()) };
        Afio::enable(rcc);
        Afio::reset(rcc);

        Parts {
            ectrl: ECTRL { _0: () },
            exticfg1: EXTI_CFG1 { _0: () },
            exticfg2: EXTI_CFG2 { _0: () },
            exticfg3: EXTI_CFG3 { _0: () },
            exticfg4: EXTI_CFG4 { _0: () },
        }
    }
}


#[cfg(any(feature="n32g401",feature="n32g430"))]
impl AfioExt for Afio {
    fn constrain(self) -> Parts {
        let rcc = unsafe { &(*Rcc::ptr()) };
        Afio::enable(rcc);
        Afio::reset(rcc);

        Parts {
            ectrl: ECTRL { _0: () },
            exticfg1: EXTI_CFG1 { _0: () },
            exticfg2: EXTI_CFG2 { _0: () },
            exticfg3: EXTI_CFG3 { _0: () },
            exticfg4: EXTI_CFG4 { _0: () },
            tol5vcfg: TOL5V_CFG { _0: () },
            eftcfg1: EFT_CFG1 { _0: () },
            eftcfg2: EFT_CFG2 { _0: () },
            filtcfg: FILT_CFG { _0: () },
            digeftcfg1: DIGEFT_CFG1 { _0: () },
            digeftcfg2: DIGEFT_CFG2 { _0: () },
        }
    }
}

/// HAL wrapper around the Afio registers
///
/// Aquired by calling [constrain](trait.AfioExt.html#constrain) on the [Afio
/// registers](../pac/struct.Afio.html)
///
/// ```rust
/// let p = pac::Peripherals::take().unwrap();
/// let mut rcc = p.Rcc.constrain();
/// let mut afio = p.Afio.constrain();
/// 
#[cfg(any(feature="n32g451",feature="n32g452",feature="n32g455",feature="n32g457",feature="n32g4fr"))]
pub struct Parts {
    pub ectrl: ECTRL,
    pub rmp_cfg : RMP_CFG,
    pub exticfg1: EXTI_CFG1,
    pub exticfg2: EXTI_CFG2,
    pub exticfg3: EXTI_CFG3,
    pub exticfg4: EXTI_CFG4,
    pub rmp_cfg3 : RMP_CFG3,
    pub rmp_cfg4 : RMP_CFG4,
    pub rmp_cfg5 : RMP_CFG5,
}

#[cfg(any(feature="n32g451",feature="n32g452",feature="n32g455",feature="n32g457",feature="n32g4fr"))]
pub struct ECTRL {
    _0: (),
}
#[cfg(any(feature="n32g451",feature="n32g452",feature="n32g455",feature="n32g457",feature="n32g4fr"))]
impl ECTRL {
    pub fn ec(&mut self) -> &afio::Ectrl {
        unsafe { &(*Afio::ptr()).ectrl() }
    }
}

#[cfg(any(feature="n32g401",feature="n32g430"))]
pub struct Parts {
    pub rmp_cfg: RMP_CFG,
    pub exticfg1: EXTI_CFG1,
    pub exticfg2: EXTI_CFG2,
    pub exticfg3: EXTI_CFG3,
    pub exticfg4: EXTI_CFG4,
    pub tol5vcfg: TOL5V_CFG,
    pub eftcfg1: EFT_CFG1,
    pub eftcfg2: EFT_CFG2,
    pub filtcfg: FILT_CFG,
    pub digeftcfg1: DIGEFT_CFG1,
    pub digeftcfg2: DIGEFT_CFG2,
}

#[cfg(any(feature="n32g432",feature="n32g435"))]
pub struct Parts {
    pub rmp_cfg: RMP_CFG,
    pub exticfg1: EXTI_CFG1,
    pub exticfg2: EXTI_CFG2,
    pub exticfg3: EXTI_CFG3,
    pub exticfg4: EXTI_CFG4,
}

pub enum DebugState {
    FullyEnabled,
    JtagNoTrstEnabled,
    SwdEnabled,
    DebugDisabled
}

/// AF remap and debug I/O configuration register (MAPR)
///
/// Aquired through the [Parts](struct.Parts.html) struct.
///
/// ```rust
/// let dp = pac::Peripherals::take().unwrap();
/// let mut rcc = dp.Rcc.constrain();
/// let mut afio = dp.Afio.constrain();
/// function_using_mapr(&mut afio.mapr);
/// ```

pub struct EXTI_CFG1 {
    _0: (),
}

impl EXTI_CFG1 {
    pub fn exti_cfg1(&mut self) -> &afio::ExtiCfg1 {
        unsafe { &(*Afio::ptr()).exti_cfg1() }
    }
}

pub struct EXTI_CFG2 {
    _0: (),
}

impl EXTI_CFG2 {
    pub fn exti_cfg2(&mut self) -> &afio::ExtiCfg2 {
        unsafe { &(*Afio::ptr()).exti_cfg2() }
    }
}

pub struct EXTI_CFG3 {
    _0: (),
}

impl EXTI_CFG3 {
    pub fn exti_cfg3(&mut self) -> &afio::ExtiCfg3 {
        unsafe { &(*Afio::ptr()).exti_cfg3() }
    }
}

pub struct EXTI_CFG4 {
    _0: (),
}

impl EXTI_CFG4 {
    pub fn exti_cfg3(&mut self) -> &afio::ExtiCfg4 {
        unsafe { &(*Afio::ptr()).exti_cfg4() }
    }
}

pub struct RMP_CFG {
    _0: (),
}

impl RMP_CFG {
    pub fn rmp_cfg(&mut self) -> &afio::RmpCfg {
        unsafe { &(*Afio::ptr()).rmp_cfg() }
    }
}

#[cfg(any(feature="n32g451",feature="n32g452",feature="n32g455",feature="n32g457",feature="n32g4fr"))]
pub struct RMP_CFG3 {
    _0: (),
}
#[cfg(any(feature="n32g451",feature="n32g452",feature="n32g455",feature="n32g457",feature="n32g4fr"))]
impl RMP_CFG3 {
    pub fn rmp_cfg3(&mut self) -> &afio::RmpCfg3 {
        unsafe { &(*Afio::ptr()).rmp_cfg3() }
    }
}

#[cfg(any(feature="n32g451",feature="n32g452",feature="n32g455",feature="n32g457",feature="n32g4fr"))]
pub struct RMP_CFG4 {
    _0: (),
}
#[cfg(any(feature="n32g451",feature="n32g452",feature="n32g455",feature="n32g457",feature="n32g4fr"))]
impl RMP_CFG4 {
    pub fn rmp_cfg4(&mut self) -> &afio::RmpCfg4 {
        unsafe { &(*Afio::ptr()).rmp_cfg4() }
    }
}

#[cfg(any(feature="n32g451",feature="n32g452",feature="n32g455",feature="n32g457",feature="n32g4fr"))]
pub struct RMP_CFG5 {
    _0: (),
}
#[cfg(any(feature="n32g451",feature="n32g452",feature="n32g455",feature="n32g457",feature="n32g4fr"))]
impl RMP_CFG5 {
    pub fn rmp_cfg5(&mut self) -> &afio::RmpCfg5 {
        unsafe { &(*Afio::ptr()).rmp_cfg5() }
    }
}

#[cfg(any(feature="n32g401",feature="n32g430"))]
pub struct TOL5V_CFG {
        _0: (),
    }

#[cfg(any(feature="n32g401",feature="n32g430"))]
impl TOL5V_CFG {
    pub fn tol5v_cfg(&mut self) -> &afio::TOL5V_CFG {
        unsafe { &(*Afio::ptr()).tol5v_cfg() }
    }
}

#[cfg(any(feature="n32g401",feature="n32g430"))]
pub struct EFT_CFG1 {
        _0: (),
    }

#[cfg(any(feature="n32g401",feature="n32g430"))]
impl EFT_CFG1 {
    pub fn eft_cfg1(&mut self) -> &afio::EFT_CFG1 {
        unsafe { &(*Afio::ptr()).eft_cfg1() }
    }
}

#[cfg(any(feature="n32g401",feature="n32g430"))]
pub struct EFT_CFG2 {
        _0: (),
    }

#[cfg(any(feature="n32g401",feature="n32g430"))]
impl EFT_CFG2 {
    pub fn eft_cfg2(&mut self) -> &afio::EFT_CFG2 {
        unsafe { &(*Afio::ptr()).eft_cfg2() }
    }
}

#[cfg(any(feature="n32g401",feature="n32g430"))]
pub struct FILT_CFG {
        _0: (),
    }

#[cfg(any(feature="n32g401",feature="n32g430"))]
impl FILT_CFG {
    pub fn filt_cfg(&mut self) -> &afio::FILT_CFG {
        unsafe { &(*Afio::ptr()).filt_cfg() }
    }
}

#[cfg(any(feature="n32g401",feature="n32g430"))]
pub struct DIGEFT_CFG1 {
        _0: (),
    }

#[cfg(any(feature="n32g401",feature="n32g430"))]
impl DIGEFT_CFG1 {
    pub fn digeft_cfg1(&mut self) -> &afio::DIGEFT_CFG1 {
        unsafe { &(*Afio::ptr()).digeft_cfg1() }
    }
}

#[cfg(any(feature="n32g401",feature="n32g430"))]
pub struct DIGEFT_CFG2 {
        _0: (),
    }

#[cfg(any(feature="n32g401",feature="n32g430"))]
impl DIGEFT_CFG2 {
    pub fn digeft_cfg2(&mut self) -> &afio::DIGEFT_CFG2 {
        unsafe { &(*Afio::ptr()).digeft_cfg2() }
    }
}