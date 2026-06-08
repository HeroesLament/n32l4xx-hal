//! Multi device hardware abstraction on top of the peripheral access API for the Nations Technologies N32G4 series microcontrollers.
//!
//! ## Feature flags
// #![doc = document_features::document_features!()]
#![no_std]
#![allow(non_camel_case_types)]
#![feature(associated_type_defaults)]
#![feature(impl_trait_in_assoc_type)]
#![feature(negative_impls)]
#![feature(min_specialization)]
#![feature(macro_metavar_expr)]
#![feature(more_qualified_paths)]
#![feature(adt_const_params)]
#![feature(trivial_bounds)]

use enumflags2::{BitFlag, BitFlags};

pub use embedded_hal as hal;
pub use embedded_hal_02 as hal_02;

pub use nb;
pub use nb::block;

#[cfg(feature = "n32l403")]
/// Re-export of the svd2rust-generated API for the n32l403 peripherals.
pub use n32l4::n32l403 as pac;

#[cfg(feature = "n32l406")]
/// Re-export of the svd2rust-generated API for the n32l406 peripherals.
pub use n32l4::n32l406 as pac;

pub mod adc;
pub mod afio;
pub mod bb;
// bkp module disabled pending N32L4 port
pub mod can;
pub mod crc;
pub mod delay;
pub mod dma;
pub mod fmc;
pub mod gpio;
pub mod i2c;
pub mod pwm;
pub mod sac;
pub mod serial;
pub mod spi;
pub mod rcc;
pub mod time;
pub mod timer;
pub mod prelude;
pub mod pwr;
pub mod usb;
mod sealed {
pub trait Sealed {}
}
pub(crate) use sealed::Sealed;

fn stripped_type_name<T>() -> &'static str {
    let s = core::any::type_name::<T>();
    let p = s.split("::");
    p.last().unwrap()
}

pub trait ReadFlags {
    /// Enum of bit flags
    type Flag: BitFlag;

    /// Get all interrupts flags a once.
    fn flags(&self) -> BitFlags<Self::Flag>;
}

pub trait ClearFlags {
    /// Enum of manually clearable flags
    type Flag: BitFlag;

    /// Clear interrupts flags with `Self::Flags`s
    ///
    /// If event flag is not cleared, it will immediately retrigger interrupt
    /// after interrupt handler has finished.
    fn clear_flags(&mut self, flags: impl Into<BitFlags<Self::Flag>>);

    /// Clears all interrupts flags
    #[inline(always)]
    fn clear_all_flags(&mut self) {
        self.clear_flags(BitFlags::ALL)
    }
}

pub trait Listen {
    /// Enum of bit flags associated with events
    type Event: BitFlag;

    /// Start listening for `Event`s
    ///
    /// Note, you will also have to enable the appropriate interrupt in the NVIC to start
    /// receiving events.
    fn listen(&mut self, event: impl Into<BitFlags<Self::Event>>);

    /// Start listening for `Event`s, stop all other
    ///
    /// Note, you will also have to enable the appropriate interrupt in the NVIC to start
    /// receiving events.
    fn listen_only(&mut self, event: impl Into<BitFlags<Self::Event>>);

    /// Stop listening for `Event`s
    fn unlisten(&mut self, event: impl Into<BitFlags<Self::Event>>);

    /// Start listening all `Event`s
    #[inline(always)]
    fn listen_all(&mut self) {
        self.listen(BitFlags::ALL)
    }

    /// Stop listening all `Event`s
    #[inline(always)]
    fn unlisten_all(&mut self) {
        self.unlisten(BitFlags::ALL)
    }
}

