use super::CryptoEngine;

pub enum DesMode {
    Ecb,
    Cbc{iv: [u8;8]}
}

pub enum DesKey {
    Des{key: [u8;7]},
    DoubleDes{key1: [u8;8], key2: [u8;8]},
    TripleDes{key1: [u8;8], key2: [u8;8], key3: [u8;8]},
}

pub struct DesEngine {
    sac : CryptoEngine
}

impl DesEngine {
    pub fn new(sac : CryptoEngine) -> Self {
        Self {
            sac 
        }
    }

    pub fn free(self) -> CryptoEngine {
        self.sac
    }

    pub fn encrypt(&mut self, ciphertext_in: &[u8], plaintext_out: &[u8], mode: DesMode, key: DesKey) {
        // self.sac.regs.sac_ctrl().write(|w| w.unk_low_bit().set_bit().init_bit().set_bit().symm_crypto_bit().set_bit().des_mode().set_bit().clear_aram_bit().set_bit() );
        // while self.sac.regs.sac_ctrl().read().unk_low_bit().bit_is_set() {}
        // self.sac.regs.sac_aram_ctrl().modify(|_,w| w.low_bit().set_bit());
        // self.sac.regs.sac_aram_ctrl().modify(|_,w| w.aram_unk_bit().set_bit());

        // match key {
        //     DesKey::Des { key } => {
        //         self.sac.regs.sac_op_ctrl().modify(|_,w| w.op_type().bits(0b10000).op_ctrl().bits(0b01));
        //     },
        // }
    }
}