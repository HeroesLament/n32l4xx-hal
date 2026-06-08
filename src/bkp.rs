/*!
  Registers that are not reset as long as Vbat or Vdd has power.

  The registers retain their values during wakes from standby mode or system resets. They also
  retain their value when Vdd is switched off as long as V_BAT is powered.

  The backup domain also contains tamper protection and writes to it must be enabled in order
  to use the real time clock (RTC).

  Write access to the backup domain is enabled in Rcc using the `rcc::Rcc::BKP::constrain()`
  function.

  Only the RTC functionality is currently implemented.
*/

use crate::{pac::{Bkp, Rcc}, rcc::Enable};

/**
  The existence of this struct indicates that writing to the the backup
  domain has been enabled. It is acquired by calling `constrain` on `rcc::Rcc::BKP`
*/
pub struct BackupDomain {
    pub(crate) _regs: Bkp,
}

macro_rules! write_datax {
    ($self:ident, $datax:ident, $idx:expr, $new:expr) => {
        $self._regs.$datax($idx).write(|w| unsafe { w.dat().bits($new) })
    };
}

macro_rules! read_datax {
    ($self:ident, $datax:ident, $idx:expr) => {
        $self._regs.$datax($idx).read().dat().bits()
    };
}

impl BackupDomain {
    pub unsafe fn conjure() -> Self {
        unsafe { Self { _regs: Bkp::steal() } }
    }


    /// Read a 16-bit value from one of the DR1 to DR10 registers part of the
    /// Backup Data Register. The register argument is a zero based index to the
    /// DRx registers: 0 is DR1, up to 41 for DR42. Providing a number above 41
    /// will panic.
    pub fn read_data_register(&self, register: usize) -> u16 {
        if register < 10 {
            read_datax!(self, datl, register)
        } else {
            read_datax!(self, dath, register-10)
        }
    }

    /// Write a 16-bit value to one of the DR1 to DR10 registers part of the
    /// Backup Data Register. The register argument is a zero based index to the
    /// DRx registers: 0 is DR1, up to 41 for DR42. Providing a number above 41
    /// will panic.
    pub fn write_data_register_low(&self, register: usize, data: u16) {
        if register < 10 {
            write_datax!(self, datl, register, data)
        } else {
            write_datax!(self, dath, register-10, data)
        }
    }
}

pub trait BkpExt {
    fn constrain(self, pwr: &mut crate::pac::Pwr) -> BackupDomain;
}

impl BkpExt for Bkp {
    fn constrain(self, pwr: &mut crate::pac::Pwr) -> BackupDomain {
        let rcc = {unsafe {&(*Rcc::ptr())}};
        Bkp::enable(rcc);
        pwr.pwr_ctrl1().modify(|_, w| w.dbkp().set_bit());
        BackupDomain{_regs: self}
    }
}