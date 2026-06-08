use crate::pac::Rcc;

pub struct MainPll {
    pub use_pll: bool,
    pub pllsysclk: Option<u32>,
}

impl MainPll {
    pub fn fast_setup(
        pllsrcclk: u32,
        use_hse: bool,
        pllsysclk: Option<u32>,
    ) -> MainPll {
        if pllsysclk.is_none() {
            return MainPll {
                use_pll: false,
                pllsysclk: None
            }
        }
        let target_freq = pllsysclk.unwrap();

        // Find the lowest pllm value that minimize the difference between
        // target frequency and the real vco_out frequency.
        let pll_presc = if use_hse {
            (1..=2)
            .max_by_key(|presc| {
                let vco_in = pllsrcclk / presc;
                let plln = target_freq / vco_in;
                target_freq - vco_in * plln
            })
            .unwrap()
        } else {
            2
        };
        let vco_in = pllsrcclk / pll_presc;
        let pll_mul = target_freq / vco_in;
        let (pllmulfct_h,pllmulfct) = if pll_mul > 16 {
            (true, pll_mul - 17)
        } else {
            (false, pll_mul - 1)
        };
        unsafe { &*Rcc::ptr() }.cfg().write(|w| {
            w.pllmulfct_h().bit(pllmulfct_h);
            unsafe { w.pllmulfct().bits(pllmulfct as u8); }
            w.pllhsepres().bit(use_hse && pll_presc == 2);
            w.pllsrc().bit(use_hse)
        });

        let real_pllsysclk = vco_in * pll_mul;
        MainPll {
            use_pll: true,
            pllsysclk: Some(real_pllsysclk),
        }
    }

}
