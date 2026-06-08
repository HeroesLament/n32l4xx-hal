use super::CryptoEngine;

mod consts;

pub struct HashEngine {
    sac : CryptoEngine
}
#[derive(Clone, Copy)]
pub enum HashType {
    Sha1,
    Sha224,
    Sha256,
    Sm3,
    Md5,
}

pub struct Hkdf {
}

impl Hkdf {
    pub fn hkdf(salt: &[u8], ikm: &[u8], info: &[u8], out: &mut [u8], hashengine: HashEngine, hashtype: HashType) -> HashEngine {
        let hashengine = hashengine;
        let mut prk_buf = [0u8;0x20];
        let digest_size = match hashtype {
            HashType::Sha1 => 0x14,
            HashType::Sha224 => 0x1C,
            HashType::Sha256 | HashType::Sm3 => 0x20,
            HashType::Md5 => 0x10,
        };
        let mut hmac = if salt.len() == 0 {
            IncrementalHmac::new(&[0u8;0x20][0..digest_size], hashengine, hashtype)
        } else {
            IncrementalHmac::new(salt, hashengine, hashtype)
        };
        hmac.update(ikm);
        let hashengine = hmac.finish(&mut prk_buf);
        let out_len = out.len();
        let mut out_prog = 0;
        let mut hmac_temp_buf = [0u8;0x20];
        let mut hmac = IncrementalHmac::new(&prk_buf,hashengine,hashtype);
        let mut ctr : u8 = 1;
        hmac.update(info);
        hmac.update(&[ctr]);
        let copy_len = out_len.min(digest_size);
        let mut hashengine: HashEngine = hmac.finish(&mut hmac_temp_buf);
        out[0..copy_len].copy_from_slice(&hmac_temp_buf[0..copy_len]);
        out_prog += copy_len;
        ctr += 1;
        while out_prog < out_len {
            let copy_len = (out_len-out_prog).min(digest_size);
            hmac = IncrementalHmac::new(&prk_buf,hashengine,hashtype);
            hmac.update(&hmac_temp_buf[0..digest_size]);
            hmac.update(info);
            hmac.update(&[ctr]);
            ctr += 1;
            hashengine = hmac.finish(&mut hmac_temp_buf);
            out[out_prog..(out_prog+copy_len)].copy_from_slice(&hmac_temp_buf[0..copy_len]);
            out_prog += copy_len;
        }
        hashengine
    }

}
pub struct IncrementalHmac {
    outer_key: [u8;0x40],
    hasher : IncrementalHasher,
}

impl IncrementalHmac {
    pub fn new(key: &[u8], hashengine: HashEngine, hashtype: HashType) -> Self {
        let mut key_buf = [0u8;0x40];
        let mut key_len = key.len();
        let mut hasher = hashengine.hash_start(hashtype);
        if key.len() > 0x40 {
            let hashtype = hasher.hashtype;
            hasher.update(key);
            let hengine = hasher.finish(&mut key_buf);
            hasher = hengine.hash_start(hashtype);
            key_len = match hashtype {
                HashType::Sha1 => 0x14,
                HashType::Sha224 => 0x1C,
                HashType::Sha256 | HashType::Sm3 => 0x20,
                HashType::Md5 => 0x10,
            };
        } else {
            key_buf[0..key.len()].copy_from_slice(key);
        }
        let mut inner_key = [0x36u8;0x40];
        let mut outer_key = [0x5cu8;0x40];
        for i in 0..key_len {
            inner_key[i] ^= key_buf[i];
            outer_key[i] ^= key_buf[i];
        }
        hasher.update(&inner_key);

        Self {
            outer_key,
            hasher
        }
    }

    pub fn update(&mut self, data : &[u8]) {
        self.hasher.update(data);
    }

    pub fn finish(self, out : &mut [u8]) -> HashEngine {
        let hashtype = self.hasher.hashtype;
        let digest_len = match hashtype {
            HashType::Sha1 => 0x14,
            HashType::Sha224 => 0x1C,
            HashType::Sha256 | HashType::Sm3 => 0x20,
            HashType::Md5 => 0x10,
        };
        let mut out_buf = [0u8;0x20];
        let hengine = self.hasher.finish(&mut out_buf);
        let mut hasher = hengine.hash_start(hashtype);
        hasher.update(&self.outer_key);
        hasher.update(&out_buf[0..digest_len]);
        hasher.finish(out)
    }

}


pub struct IncrementalHasher {
    hashengine : HashEngine,
    hashtype : HashType,
    msg_len_buf : [usize;4],
    incr_buf : [u8;0x84],
    msg_idx : usize
}


impl IncrementalHasher {
    pub fn new(hashengine: HashEngine, hashtype: HashType) -> Self {
        Self {
            hashengine,
            hashtype,
            msg_len_buf: [0;4],
            incr_buf: [0;0x84],
            msg_idx : 0
        }
    }

    fn byte_len_plus(&mut self, in_len : usize) -> bool {
        self.msg_len_buf[1] = self.msg_len_buf[1] + in_len;
        if self.msg_len_buf[1] < in_len {
            self.msg_len_buf[0] = self.msg_len_buf[0] + 1;
        }
        if self.msg_len_buf[0] < 0x20000000 {
            return true
        } else {
            return false
        }
    }

    fn proc_incr_buf(&mut self) {
        let incr_buf_u32 : &[u32] = bytemuck::cast_slice(&self.incr_buf[0..0x40]);
        for data in  incr_buf_u32 {
            self.hashengine.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(*data)});
        }
        let hashctrl_data : u32 = match self.hashtype {
            HashType::Sha1 => 0x0,
            HashType::Sha224 | HashType::Sha256 => 0x2,
            HashType::Sm3 => 0xf,
            HashType::Md5 => 0x10,
        };
        self.hashengine.sac.regs.sac_op_ctrl().write(|w| unsafe { w.bits(hashctrl_data)});
        self.hashengine.sac.regs.sac_op_ctrl().modify(|_,w| w.run().set_bit());
        while self.hashengine.sac.regs.sac_op_ctrl().read().run().bit_is_set() {}
        self.msg_idx = 0;
    }

    pub fn update(&mut self, in_data : &[u8]) {
        let msg_idx = self.msg_idx;
        let end_idx = msg_idx + in_data.len();
        let mut cycle_cnt = end_idx >> 6;
        let block_len = 0x40;
        let mut in_progress: usize = 0;

        if !self.byte_len_plus(in_data.len()) {
            panic!("shitfuck")
        }
        if end_idx < block_len {
            self.incr_buf[self.msg_idx..end_idx].copy_from_slice(in_data);
            self.msg_idx = end_idx;
            return;
        }
        
        if msg_idx != 0 {
            self.incr_buf[self.msg_idx..block_len].copy_from_slice(&in_data[0..(block_len-self.msg_idx)]);
            in_progress += block_len-self.msg_idx;
            self.proc_incr_buf();
            cycle_cnt -= 1;
        }
        for _ in 0..cycle_cnt {
            self.incr_buf[0..block_len].copy_from_slice(&in_data[in_progress..(in_progress+block_len)]);
            self.proc_incr_buf();
            in_progress += block_len;
        }
        self.msg_idx = end_idx & (block_len - 1);
        self.incr_buf[0..self.msg_idx].copy_from_slice(&in_data[in_progress..(in_progress+self.msg_idx)]);
    }

    fn pad_msgbuf(&mut self) {
        let hashctrl_data : u32 = match self.hashtype {
            HashType::Sha1 => 0x0,
            HashType::Sha224 | HashType::Sha256 => 0x2,
            HashType::Sm3 => 0xf,
            HashType::Md5 => 0x10,
        };
        let mut final_update_size = (self.msg_idx + 4) >> 2;
        self.incr_buf[self.msg_idx] = 0x80;
        self.incr_buf[self.msg_idx + 1] = 0x0;
        self.incr_buf[self.msg_idx + 2] = 0x0;
        self.incr_buf[self.msg_idx + 3] = 0x0;
        self.msg_len_buf[0] = (self.msg_len_buf[0] << 3) | (self.msg_len_buf[1] >> 0x1d);
        self.msg_len_buf[1] <<= 3;
        match self.hashtype {
            HashType::Md5 => {
                let swap = self.msg_len_buf[0];
                self.msg_len_buf[0] = self.msg_len_buf[1];
                self.msg_len_buf[1] = swap;
            },
            _ => {
                self.msg_len_buf[0] = self.msg_len_buf[0].swap_bytes();
                self.msg_len_buf[1] = self.msg_len_buf[1].swap_bytes();

            }
        }
        if self.msg_idx > 0x37 {
            let incr_buf_u32 : &[u32] = bytemuck::cast_slice(&self.incr_buf[0..(final_update_size<<2)]);
            for data in incr_buf_u32 {
                self.hashengine.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(*data)});
            }
            for _ in final_update_size..0x10 {
                self.hashengine.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(0)});
            }

            self.hashengine.sac.regs.sac_op_ctrl().write(|w| unsafe { w.bits(hashctrl_data)});
            self.hashengine.sac.regs.sac_op_ctrl().modify(|_,w| w.run().set_bit());
            while self.hashengine.sac.regs.sac_op_ctrl().read().run().bit_is_set() {}
            final_update_size = 0;
            self.hashengine.sac.regs.sac_aram_ctrl().modify(|_,w|w.hash_done().set_bit());
        }
        let incr_buf_u32 : &[u32] = bytemuck::cast_slice(&self.incr_buf[0..(final_update_size<<2)]);
        for data in incr_buf_u32 {
            self.hashengine.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(*data)});
        }
        for _ in final_update_size..0xe {
            self.hashengine.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(0)});
        }
        self.hashengine.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(self.msg_len_buf[0] as u32)});
        self.hashengine.sac.regs.sac_in_fifo().write(|w| unsafe { w.bits(self.msg_len_buf[1] as u32)});
        self.hashengine.sac.regs.sac_op_ctrl().write(|w| unsafe { w.bits(hashctrl_data)});
        self.hashengine.sac.regs.sac_op_ctrl().modify(|_,w| w.run().set_bit());
        while self.hashengine.sac.regs.sac_op_ctrl().read().run().bit_is_set() {}
        self.hashengine.sac.regs.sac_aram_ctrl().modify(|_,w|w.hash_done().set_bit());
    }

    pub fn finish(mut self, out_buf: &mut [u8]) -> HashEngine {
        self.pad_msgbuf();
        let digest_len : usize = match self.hashtype {
            HashType::Sha1 => 0x14,
            HashType::Sha224 => 0x1C,
            HashType::Sha256 | HashType::Sm3 => 0x20,
            HashType::Md5 => 0x10,
        };
        let out_buf_u32: &mut [u32] = bytemuck::cast_slice_mut(out_buf);
        for i in 0..(digest_len/4) {
            out_buf_u32[i] = self.hashengine.sac.regs.sac_out_fifo().read().bits();
        }

        self.hashengine.sac.regs.sac_ctrl().modify(|r,w| unsafe { w.bits(r.bits() | 0x100)});
        self.hashengine.sac.regs.sac_ctrl().modify(|r,w| unsafe { w.bits(r.bits() & 0xffffffed)});
        while (self.hashengine.sac.regs.sac_ctrl().read().bits() & 0x80) != 0 {}
        self.hashengine.sac.regs.sac_aram_ctrl().modify(|_,w| w.low_bit().set_bit());
        self.hashengine.sac.regs.sac_ctrl().modify(|r,w| unsafe { w.bits(r.bits() & 0xffffbfff)});

        self.hashengine
    }
}


impl HashEngine {
    pub fn new(sac : CryptoEngine) -> Self {
        Self {
            sac 
        }
    }

    pub fn free(self) -> CryptoEngine {
        self.sac
    }

    pub fn hash_start(self, hashtype: HashType) -> IncrementalHasher {
        // HASH_INIT
        let saccr_data : u32 = match hashtype {
            HashType::Sha1 | HashType::Sha224 | HashType::Sha256 | HashType::Sm3 => 0xd2,
            HashType::Md5 => 0x92,
        };
        self.sac.reset();
        self.sac.regs.sac_ctrl().write(|w| unsafe { w.bits(saccr_data | 0x4080)});
        while (self.sac.regs.sac_ctrl().read().bits() & 0x80) != 0 {}
        self.sac.regs.sac_aram_ctrl().modify(|_,w| w.low_bit().set_bit());
        let hashctrl_data : u32 = match hashtype {
            HashType::Sha1 => 0x0,
            HashType::Sha224 | HashType::Sha256 => 0x2,
            HashType::Sm3 => 0xf,
            HashType::Md5 => 0x10,
        };
        self.sac.regs.sac_op_ctrl().write(|w| unsafe { w.bits(hashctrl_data) });

        match hashtype {
            HashType::Sha1 => {
                for k_val in consts::SHA1_K {
                    self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.bits(k_val)});
                }
            },
            HashType::Sha224 | HashType::Sha256 => {
                for k_val in consts::SHA256_K {
                    self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.bits(k_val)});
                }
            },
            HashType::Sm3 => {
                for k_val in consts::SM3_K {
                    self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.bits(k_val)});
                }   
            },
            HashType::Md5 => {
                for k_val in consts::MD5_K {
                    self.sac.regs.sac_key_reg_3().write(|w| unsafe { w.bits(k_val)});
                }
                for s_val in consts::MD5_S {
                    self.sac.regs.sac_key_reg_1().write(|w| unsafe { w.bits(s_val)});
                }

            },
        }

        //HASH_START
        let iv = match hashtype {
            HashType::Sha1 => &consts::SHA1_IV[..],
            HashType::Sha224 => &consts::SHA224_IV[..],
            HashType::Sha256 => &consts::SHA256_IV[..],
            HashType::Sm3 => &consts::SM3_IV[..],
            HashType::Md5 => &consts::MD5_IV[..],
        };
        for iv_val in iv {
            self.sac.regs.sac_iv_reg().write(|w| unsafe { w.bits(*iv_val) });
        }
        
        IncrementalHasher::new(self, hashtype)
    }
}