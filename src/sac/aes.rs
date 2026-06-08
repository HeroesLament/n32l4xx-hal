
use super::CryptoEngine;

#[derive(Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]

pub enum AesMode {
    Ecb{},
    Cbc{iv: [u32;4]},
    Ctr{iv: [u32;4]},
}


#[derive(Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]

pub enum AesKey {
    Aes128Key{key : [u32;4]},
    Aes192Key{key : [u32;6]},
    Aes256Key{key : [u32;8]},
}

pub struct AesEngine {
    sac : CryptoEngine
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AesError {
    LengthError,
}
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AesDir {
    Encrypt,
    Decrypt,
}

impl AesEngine {
    pub fn new(sac : CryptoEngine) -> Self {
        Self {
            sac 
        }
    }

    pub fn free(self) -> CryptoEngine {
        self.sac
    }

    pub fn execute(&mut self, data_in: &[u32], data_out: &mut [u32], dir : AesDir, mode: AesMode, key: AesKey) -> Result<(),AesError> {
        let in_len: usize = data_in.len();
        let out_len = data_out.len();
        let len_in_blocks = in_len >> 2;
        let sub_block_remainder =  in_len & 3;

        if (in_len != out_len) || len_in_blocks == 0 {
            return Err(AesError::LengthError)
        }
        match mode {
            AesMode::Ecb { .. } | AesMode::Cbc { .. } => {
                if sub_block_remainder != 0 {
                    return Err(AesError::LengthError)
                }
            },
            _ => ()
        }
        // AES INIT
        self.sac.reset();
        self.sac.regs.sac_ctrl().write(|w| unsafe { w.bits(0x2d0)});
        while (self.sac.regs.sac_ctrl().read().bits() & 0x80) != 0 {}
        self.sac.regs.sac_aram_ctrl().modify(|_,w| w.low_bit().set_bit());
        cortex_m::asm::dsb();


        match key {
            AesKey::Aes128Key { key } => {
                self.sac.regs.sac_op_ctrl().write(|w| unsafe { w.bits(0x0)});
                cortex_m::asm::dsb();
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[0]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[1]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[2]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[3]) });
            },
            AesKey::Aes192Key { key } => {
                self.sac.regs.sac_op_ctrl().write(|w| unsafe { w.bits(0x8)});
                cortex_m::asm::dsb();
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[0]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[1]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[2]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[3]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[4]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[5]) });
            },
            AesKey::Aes256Key { key } => {
                self.sac.regs.sac_op_ctrl().write(|w| unsafe { w.bits(0x10)});
                cortex_m::asm::dsb();
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[0]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[1]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[2]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[3]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[4]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[5]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[6]) });
                self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.key().bits(key[7]) });
            }

        }
        cortex_m::asm::dsb();
        self.sac.regs.sac_op_ctrl().modify(|r,w| unsafe { w.bits(r.bits() | 0x80)});
        while (self.sac.regs.sac_op_ctrl().read().bits() & 0x80) != 0x0 {}
        cortex_m::asm::dsb();
        self.sac.regs.sac_aram_ctrl().modify(|_,w| w.aes_done().set_bit());
        cortex_m::asm::dsb();
        match (dir,mode) {
            (AesDir::Encrypt, _) | (_, AesMode::Ctr { .. }) => {
                self.sac.regs.sac_op_ctrl().modify(|r,w| unsafe { w.bits((r.bits() & 0xfc) + 1)})
            },
            (AesDir::Decrypt, AesMode::Cbc { .. } | AesMode::Ecb { .. }) => {
                self.sac.regs.sac_op_ctrl().modify(|r,w| unsafe { w.bits((r.bits() & 0xfd) + 2)})
            }
        }
        cortex_m::asm::dsb();
        match mode {
            AesMode::Cbc { iv } => {
                self.sac.regs.sac_op_ctrl().modify(|r,w| unsafe { w.bits((r.bits() & 0xdf) | 0x20) });
                self.sac.regs.sac_iv_reg().write(|w| unsafe { w.iv().bits(iv[0])} );
                self.sac.regs.sac_iv_reg().write(|w| unsafe { w.iv().bits(iv[1])} );
                self.sac.regs.sac_iv_reg().write(|w| unsafe { w.iv().bits(iv[2])} );
                self.sac.regs.sac_iv_reg().write(|w| unsafe { w.iv().bits(iv[3])} );

            },
            AesMode::Ctr { .. } |  AesMode::Ecb { .. } => self.sac.regs.sac_op_ctrl().modify(|r,w| unsafe { w.bits(r.bits() & 0xdf) }),
        };
        cortex_m::asm::dsb();

        //AES RUN
        for i in 0..len_in_blocks {
            match mode {
                AesMode::Ctr { iv } => {
                    let iv = u128::from_be_bytes(bytemuck::cast(iv)).wrapping_add(i as u128);    
                    let swapped_iv : [u32;4] = bytemuck::cast(iv.to_be_bytes());
                    self.sac.regs.sac_in_fifo().write(|w| unsafe { w.data().bits(swapped_iv[0]) });
                    self.sac.regs.sac_in_fifo().write(|w| unsafe { w.data().bits(swapped_iv[1]) });
                    self.sac.regs.sac_in_fifo().write(|w| unsafe { w.data().bits(swapped_iv[2]) });
                    self.sac.regs.sac_in_fifo().write(|w| unsafe { w.data().bits(swapped_iv[3]) });                    
                },
                _ => {
                    self.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(data_in[0 + (i * 4)]) });
                    self.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(data_in[1 + (i * 4)]) });
                    self.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(data_in[2 + (i * 4)]) });
                    self.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(data_in[3 + (i * 4)]) });
                }
            }
            cortex_m::asm::dsb();
            self.sac.regs.sac_op_ctrl().modify(|r,w| unsafe{ w.bits((r.bits() & 0x7f) | 0x80)});
            while (self.sac.regs.sac_op_ctrl().read().bits() & 0x80) != 0x0 {}
            cortex_m::asm::dsb();
            self.sac.regs.sac_aram_ctrl().modify(|_,w| w.aes_done().set_bit());
            cortex_m::asm::dsb();
            match mode {
                AesMode::Ecb { .. } | AesMode::Cbc { .. } => {
                    data_out[0 + (i * 4)] = self.sac.regs.sac_out_fifo().read().data().bits();
                    data_out[1 + (i * 4)] = self.sac.regs.sac_out_fifo().read().data().bits();
                    data_out[2 + (i * 4)] = self.sac.regs.sac_out_fifo().read().data().bits();
                    data_out[3 + (i * 4)] = self.sac.regs.sac_out_fifo().read().data().bits();
                },
                AesMode::Ctr { .. } => {
                    data_out[0 + (i * 4)] = data_in[0 + (i * 4)] ^ self.sac.regs.sac_out_fifo().read().bits();
                    data_out[1 + (i * 4)] = data_in[1 + (i * 4)] ^ self.sac.regs.sac_out_fifo().read().bits();
                    data_out[2 + (i * 4)] = data_in[2 + (i * 4)] ^ self.sac.regs.sac_out_fifo().read().bits();
                    data_out[3 + (i * 4)] = data_in[3 + (i * 4)] ^ self.sac.regs.sac_out_fifo().read().bits();

                },
            }
        }
        // if sub_block_remainder != 0 {
        //     self.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(bytemuck::cast_slice(&iv[0..3])[0]) });
        //     self.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(bytemuck::cast_slice(&iv[4..7])[0]) });
        //     self.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(bytemuck::cast_slice(&iv[8..11])[0]) });
        //     self.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(bytemuck::cast_slice(&iv[12..15])[0]) });
        //     self.sac.regs.sac_op_ctrl().modify(|r,w| unsafe{ w.bits((r.bits() & 0x7f) | 0x80)});
        //     while (self.sac.regs.sac_op_ctrl().read().bits() & 0x80) != 0x0 {}
        //     self.sac.regs.sac_aram_ctrl().modify(|_,w| w.aes_done().set_bit());
        //     cortex_m::asm::dsb();
        //     for i in 0..sub_block_remainder {
        //         data_out[i + (len_in_blocks << 2)] = data_in[i + (len_in_blocks << 2)] ^ self.sac.regs.sac_out_fifo().read().data().bits();
        //     }
        // }

        self.sac.regs.sac_ctrl().modify(|r,w| unsafe { w.bits(r.bits() | 0x100) });
        self.sac.regs.sac_ctrl().modify(|r,w| unsafe { w.bits(r.bits() & 0xffffffef) });
        while self.sac.regs.sac_ctrl().read().clear_aram().bit_is_set() {}
        self.sac.regs.sac_aram_ctrl().modify(|_,w| w.low_bit().set_bit());
        self.sac.regs.sac_ctrl().modify(|r,w| unsafe { w.bits(r.bits() & 0xfffffdff) });
        
        Ok(())
    }
}