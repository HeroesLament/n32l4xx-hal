use crate::pac::Rcc;

use super::CryptoEngine;

pub struct Trng {
    sac : CryptoEngine
}

impl Trng {
    pub fn new(sac : CryptoEngine) -> Self {
        Self {
            sac 
        }
    }

    pub fn free(self) -> CryptoEngine {
        self.sac
    }

    pub fn get_entropy(&mut self, entropy_buf : &mut [u8]) {
        self.sac.init_with_trng();
        let saccr = self.sac.regs.sac_ctrl().read().bits();
        let hashctrl = self.sac.regs.sac_op_ctrl().read().bits();
        self.sac.regs.sac_ctrl().modify(|r,w| unsafe { w.bits(0x16)});
        self.sac.regs.sac_ctrl().modify(|r,w| unsafe { w.bits(0x16)});
        self.sac.regs.sac_op_ctrl().modify(|r,w| unsafe { w.bits(0x2)  });
        self.sac.regs.sac_op_ctrl().modify(|r,w| unsafe { w.bits(0x102)  });
        self.sac.regs.sac_op_ctrl().modify(|r,w| unsafe { w.bits(0x182)  });
            
        self.sac.regs.sac_aram_ctrl().modify(|r,w| unsafe { w.bits( r.bits()|0x800)});
        while (self.sac.regs.sac_op_ctrl().read().bits() & 0x80) != 0 {}
        self.sac.regs.sac_aram_ctrl().modify(|r,w| unsafe { w.bits(r.bits()|0x80)});
        for i in 0..((entropy_buf.len() + 3)/4) {
            self.sac.regs.sac_op_ctrl().modify(|r,w| unsafe { w.bits(0x182)});
            while(self.sac.regs.sac_op_ctrl().read().bits() & 0x80) != 0 {}
            self.sac.regs.sac_aram_ctrl().modify(|r     ,w| unsafe { w.bits(r.bits()|0x80)});
            let rng_bytes = self.sac.regs.sac_trng_result().read().bits().to_ne_bytes();
            let rem_len = entropy_buf.len() - i*4;
            let cur_write_len = rem_len.min(4);
            for j in 0..cur_write_len {
                entropy_buf[i*4 + j] = rng_bytes[j];
            }               
        }
        self.sac.regs.sac_ctrl().write(|w| unsafe { w.bits(saccr)});
        self.sac.regs.sac_op_ctrl().write(|w| unsafe { w.bits(hashctrl)});

    }
}