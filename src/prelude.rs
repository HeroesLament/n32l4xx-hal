pub use embedded_hal::delay::DelayNs as _;
pub use embedded_hal_02::adc::OneShot as _embedded_hal_adc_OneShot;
pub use embedded_hal_02::blocking::serial::Write as _embedded_hal_blocking_serial_Write;
pub use embedded_hal_02::Capture as _embedded_hal_Capture;
pub use embedded_hal_02::Pwm as _embedded_hal_Pwm;
pub use embedded_hal_02::Qei as _embedded_hal_Qei;
pub use embedded_hal_nb::serial::Read as _embedded_hal_serial_nb_Read;
pub use embedded_hal_nb::serial::Write as _embedded_hal_serial_nb_Write;
pub use fugit::ExtU32 as _fugit_ExtU32;
pub use fugit::RateExtU32 as _fugit_RateExtU32;

pub use crate::dma::DmaExt as _;
pub use crate::serial::SerialDma as _;
pub use crate::gpio::ExtiPin as _n32g4xx_hal_gpio_ExtiPin;
pub use crate::gpio::GpioExt as _n32g4xx_hal_gpio_GpioExt;
pub use crate::i2c::dma::I2CMasterHandleIT as _n32g4xx_hal_i2c_dma_I2CMasterHandleIT;
pub use crate::i2c::dma::I2CMasterReadDMA as _n32g4xx_hal_i2c_dma_I2CMasterReadDMA;
pub use crate::i2c::dma::I2CMasterWriteDMA as _n32g4xx_hal_i2c_dma_I2CMasterWriteDMA;
pub use crate::i2c::dma::I2CMasterWriteReadDMA as _n32g4xx_hal_i2c_dma_I2CMasterWriteReadDMA;
pub use crate::rcc::RccExt as _n32g4xx_hal_rcc_RccExt;
pub use crate::pwr::PwrExt as _n32g4xx_hal_pwr_PwrExt;
pub use crate::serial::RxISR as _n32g4xx_hal_serial_RxISR;
pub use crate::serial::RxListen as _n32g4xx_hal_serial_RxListen;
pub use crate::serial::SerialExt as _n32g4xx_hal_serial_SerialExt;
pub use crate::serial::TxISR as _n32g4xx_hal_serial_TxISR;
pub use crate::serial::TxListen as _n32g4xx_hal_serial_TxListen;
pub use crate::spi::SpiExt as _n32g4xx_hal_spi_SpiExt;
pub use crate::afio::AfioExt as _n32g4xx_hal_afio_AfioExt;
pub use crate::time::U32Ext as _n32g4xx_hal_time_U32Ext;
#[cfg(feature = "rtic1")]
pub use crate::timer::MonoTimer64Ext as _;
#[cfg(feature = "rtic1")]
pub use crate::timer::MonoTimerExt as _;
#[cfg(feature = "rtic1")]
pub use crate::timer::SysMonoTimerExt as _n32g4xx_hal_timer_SysMonoTimerExt;

pub use crate::ClearFlags as _;
pub use crate::Listen as _;
pub use crate::ReadFlags as _;