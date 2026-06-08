use super::*;
use crate::bb;

macro_rules! bus_enable {
    ($PER:ident => $bit:literal) => {
        impl Enable for crate::pac::$PER {
            #[inline(always)]
            fn enable(rcc: &RccRB) {
                unsafe {
                    bb::set(Self::Bus::pclken(rcc), $bit);
                }
                // Stall the pipeline to work around erratum 2.1.13 (DM00037591)
                cortex_m::asm::dsb();
            }
            #[inline(always)]
            fn disable(rcc: &RccRB) {
                unsafe {
                    bb::clear(Self::Bus::pclken(rcc), $bit);
                }
            }
            #[inline(always)]
            fn is_enabled() -> bool {
                let rcc = pac::Rcc::ptr();
                (Self::Bus::pclken(unsafe { &*rcc }).read().bits() >> $bit) & 0x1 != 0
            }
        }
    };
}

macro_rules! bus_reset {
    ($PER:ident => $bit:literal) => {
        impl Reset for crate::pac::$PER {
            #[inline(always)]
            fn reset(rcc: &RccRB) {
                unsafe {
                    bb::set(Self::Bus::prst(rcc), $bit);
                    bb::clear(Self::Bus::prst(rcc), $bit);
                }
            }
        }
    };
}

macro_rules! bus {
    ($($PER:ident => ($busX:ty, $bit:literal),)+) => {
        $(
            impl crate::Sealed for crate::pac::$PER {}
            impl RccBus for crate::pac::$PER {
                type Bus = $busX;
            }
            bus_enable!($PER => $bit);
            bus_reset!($PER => $bit);
        )+
    }
}



#[cfg(any(feature = "n32g401", feature = "n32g430"))]
bus! {
    GPIOA => (AHB, 7),
    GPIOB => (AHB, 8),
    GPIOC => (AHB, 9),
    GPIOD => (AHB, 10),
}

#[cfg(not(any(feature = "n32g401", feature = "n32g430")))]
bus! {
    Gpioa => (APB2, 2),
    Gpiob => (APB2, 3),
    Gpioc => (APB2, 4),
    Gpiod => (APB2, 5),
}

#[cfg(any(feature = "n32g451", feature = "n32g452", feature = "n32g455", feature = "n32g457", feature = "n32g4fr"))]
bus! {
    Gpioe => (APB2, 6),
    Gpiof => (APB2, 7),
    Gpiog => (APB2, 8),
}

// TODO: RNG/SAC Abstraction.
// #[cfg(not(any(feature = "n32g401", feature = "n32g430")))]
// bus! {
//     Sac => (AHB, 11),
//     Rng => (AHB, 9),
// }


bus! {
    Crc => (AHB, 6),
    Adc1 => (AHB, 12),
    Dma1 => (AHB, 0),
}

#[cfg(any(feature = "n32g435",feature = "n32g455",feature = "n32g457"))]
bus! {
    Opamp => (APB1, 31),
}

#[cfg(not(feature = "n32g4fr"))]
bus! {
    Comp => (APB1, 6),
}

bus! {
    Pwr => (APB1, 28),
    I2c2 => (APB1, 22),
    I2c1 => (APB1, 21),
    Usart2 => (APB1, 17),
    Wwdg => (APB1, 11),
    Tim6 => (APB1, 4),
    Tim5 => (APB1, 3),
    Tim4 => (APB1, 2),
    Tim3 => (APB1, 1),
    Tim2 => (APB1, 0),
    
}

bus! {
    Usart1 => (APB2, 14),
    Tim8 => (APB2, 13),
    Spi1 => (APB2, 12),
    Tim1 => (APB2, 11),
    Afio => (APB2, 0),
}


#[cfg(any(feature = "n32g452",feature = "n32g455",feature = "n32g457",feature = "n32g4fr"))]
bus! {
    Qspi => (AHB, 17),

}

#[cfg(feature = "n32g457")]
bus! {
    Eth => (AHB, 16),
}

#[cfg(any(feature = "n32g451",feature = "n32g452",feature = "n32g455",feature = "n32g457",feature = "n32g4fr"))]
bus! {
    Adc4 => (AHB, 15),
    Adc3 => (AHB, 14),
    Adc2 => (AHB, 13),
    Sac => (AHB,11),
    Sdio => (AHB, 10),
    Dma2 => (AHB, 1),
}


#[cfg(any(feature = "n32g451",feature = "n32g452",feature = "n32g455",feature = "n32g457",feature = "n32g4fr"))]
bus! {
    I2c4 => (APB2, 20),
    I2c3 => (APB2, 19),
    Uart7 => (APB2, 18),
    Uart6 => (APB2, 17),
}

#[cfg(any(feature = "n32g401",feature = "n32g430",feature = "n32g432",feature = "n32g435"))]
bus! {
    Spi2 => (APB2, 19),
}

#[cfg(any(feature = "n32g401",feature = "n32g430"))]
bus! {
    Uart4 => (APB2, 18),
    Uart3 => (APB2, 17),
    Beep => (APB2, 1),
}

#[cfg(any(feature = "n32g432",feature = "n32g435"))]
bus! {
    Spi2 => (APB2, 19),
    Uart5 => (APB2, 18),
    Uart4 => (APB2, 17),
}

#[cfg(not(any(feature = "n32g401",feature = "n32g430")))]
bus! {
    Dac => (APB1, 29),
    Usb => (APB1, 23),
    Usart3 => (APB1, 18),
    Tsc => (APB1, 10),
    Tim7 => (APB1, 5),
}

#[cfg(any(feature = "n32g451",feature = "n32g452",feature = "n32g455",feature = "n32g457",feature = "n32g4fr"))]
bus! {
    Bkp => (APB1, 27),
    Uart5 => (APB1, 20),
    Uart4 => (APB1, 19),
    Spi3 => (APB1, 15),
    Spi2 => (APB1, 14),
}

#[cfg(any(feature = "n32g452",feature = "n32g455",feature = "n32g457",feature = "n32g4fr"))]
bus! {
    Can2 => (APB1, 26),
}

#[cfg(not(any(feature = "n32g401")))]
bus! {
    Can1 => (APB1, 25),
}

#[cfg(any(feature = "n32g401",feature = "n32g430"))]
bus! {
    Tim9 => (APB1, 9),
    Afec => (APB1, 8),
}