//! # Controller Area Network (CAN) Interface
//!
//! TX: Alternate Push-Pull Output
//! RX: Alternate (input direction selected internally)
//!
//! The N32L40x has a single CAN controller (`pac::Can`). Pin options come
//! from the verified GPIO AF table (see `tools/gpio_af/af_table_um.tsv`):
//!
//! | Function | Option A | Option B |
//! |----------|----------|----------|
//! | TX       | PA12 (AF1) | PB9 (AF5) |
//! | RX       | PA11 (AF1) | PB8 (AF5) |
//!
//! Pins are selected by their alternate function (the STM32-F4/L4 model);
//! there is no AFIO remap mux on this family.

use crate::gpio::alt::CanCommon;
use crate::pac::{self, Rcc};

/// Interface to the CAN peripheral.
pub struct Can<Instance> {
    _peripheral: Instance,
}

impl<Instance> Can<Instance>
where
    Instance: crate::rcc::Enable + CanCommon,
{
    pub fn new(can: Instance) -> Can<Instance> {
        let rcc = unsafe { &(*Rcc::ptr()) };
        Instance::enable(rcc);

        Can { _peripheral: can }
    }

    /// Routes CAN TX and RX signals to the given pins.
    ///
    /// The pins must be configured for the CAN alternate function; this is
    /// enforced by the `Into` bounds against the peripheral's `Rx`/`Tx`
    /// pin types from the GPIO altmap.
    pub fn assign_pins<RX, TX>(&self, _pins: (RX, TX))
    where
        RX: Into<Instance::Rx>,
        TX: Into<Instance::Tx>,
    {
        let _rx: Instance::Rx = _pins.0.into();
        let _tx: Instance::Tx = _pins.1.into();
    }
}

unsafe impl bxcan::Instance for Can<pac::Can> {
    const REGISTERS: *mut bxcan::RegisterBlock = pac::Can::ptr() as *mut bxcan::RegisterBlock;
}

unsafe impl bxcan::FilterOwner for Can<pac::Can> {
    const NUM_FILTER_BANKS: u8 = 14;
}
