use crate::dma::DMAChannel;
macro_rules! chmap_setup {
    (
        $(
        $PER:ty: (
            $dmaunit:tt::($($dmach:ident$(,)*)+) => (R => $rmp_rx:expr, W => $rmp_tx:expr)
        ),
        )+
    ) => {
        $(
            $(
                impl crate::dma::CompatibleChannel<$PER,crate::dma::R> for crate::dma::$dmaunit::$dmach {
                    fn configure_channel(&mut self) {
                        unsafe { self.st().chsel().modify(|_,w| w.ch_sel().bits($rmp_rx)) }
                    }
                }
    
                impl crate::dma::CompatibleChannel<$PER,crate::dma::W> for crate::dma::$dmaunit::$dmach {
                    fn configure_channel(&mut self) {
                        unsafe { self.st().chsel().modify(|_,w| w.ch_sel().bits($rmp_tx)) }
                    }
                }
            )+
        )+
    }
}

//ADCs
chmap_setup!(
    crate::pac::Adc1: (dma1::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 10, W => 15)),
    crate::pac::Adc2: (dma1::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 10, W => 15)),
    crate::pac::Adc3: (dma2::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 10, W => 15)),
    crate::pac::Adc4: (dma2::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 10, W => 15)),
);

//US?ARTs
chmap_setup!(
    crate::pac::Usart1: (dma1::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 23, W => 16)),
    crate::pac::Usart2: (dma1::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 29, W => 34)),
    crate::pac::Usart3: (dma1::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 11, W => 5)),
    crate::pac::Uart4: (dma2::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 14, W => 24)),
    crate::pac::Uart5: (dma1::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 40, W => 1)),
    crate::pac::Uart6: (dma2::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 14, W => 12)),
    crate::pac::Uart7: (dma2::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 27, W => 30)),
);

//I2Cs
chmap_setup!(
    crate::pac::I2c1: (dma1::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 38, W => 33)),
    crate::pac::I2c2: (dma1::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 28, W => 22)),
    crate::pac::I2c3: (dma2::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 6, W => 2)),
);

//SPIs
chmap_setup!(
    crate::pac::Spi1: (dma1::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 10, W => 15)),
    crate::pac::Spi2: (dma1::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 21, W => 25)),
    crate::pac::Spi3: (dma2::(C1,C2,C3,C4,C5,C6,C7,C8) => (R => 4, W => 11)),
);

