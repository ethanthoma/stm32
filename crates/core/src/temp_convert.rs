#[cfg(not(feature = "verus"))]
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::arithmetic::div_mod::{lemma_div_is_ordered, lemma_div_is_ordered_by_denominator};
#[cfg(feature = "verus")]
use vstd::prelude::*;

verus! {

// internal reference voltage, mV
pub const V_REFINT_MV: u32 = 1210;

// 12-bit ADC full scale
// (1 << 12) - 1
// TODO: compute this nicely
pub const ADC_MAX: u16 = 4095;

// smallest vrefint keeping the result within u16 (and != 0)
// TODO: compute this nicely
pub const V_REFINT_MIN: u16 = 76;

pub fn to_millivolts(vsense: u16, vrefint: u16) -> (mv: Option<u16>)
    ensures
        vsense <= ADC_MAX && vrefint >= V_REFINT_MIN ==> (mv matches Some(v)
            && v == (vsense as int * V_REFINT_MV as int) / (vrefint as int)),
        !(vsense <= ADC_MAX && vrefint >= V_REFINT_MIN) ==> mv is None,
{
    if vsense <= ADC_MAX && vrefint >= V_REFINT_MIN {
        Some(to_millivolts_unchecked(vsense, vrefint))
    } else {
        None
    }
}

fn to_millivolts_unchecked(vsense: u16, vrefint: u16) -> (mv: u16)
    requires
        vsense <= ADC_MAX,
        vrefint >= V_REFINT_MIN,
    ensures
        mv == (vsense as int * V_REFINT_MV as int) / (vrefint as int),
{
    let num: u32 = (vsense as u32) * V_REFINT_MV;
    let den: u32 = vrefint as u32;
    let v: u32 = num / den;
    proof {
        lemma_div_is_ordered_by_denominator(num as int, V_REFINT_MIN as int, den as int);
        lemma_div_is_ordered(num as int, ADC_MAX as int * V_REFINT_MV as int, V_REFINT_MIN as int);
    }
    v as u16
}

// sensor output at 25 C, mV
pub const V_25_MV: i32 = 760;

// 1000 / 2.5 mV-per-C, mC * mV^-1
pub const MC_PER_MV: i32 = 400;

// 25 C, mC
pub const OFFSET_MC: i32 = 25_000;

pub fn to_millicelsius(mv: u16) -> (mc: i32)
    ensures
        mc == (mv as int - V_25_MV as int) * MC_PER_MV as int + OFFSET_MC as int,
{
    (mv as i32 - V_25_MV) * MC_PER_MV + OFFSET_MC
}

} // verus!

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn to_millivolts_is_total() {
        let vsense: u16 = kani::any();
        let vrefint: u16 = kani::any();
        match to_millivolts(vsense, vrefint) {
            Some(_) => assert!(vsense <= ADC_MAX && vrefint >= V_REFINT_MIN),
            None => assert!(vsense > ADC_MAX || vrefint < V_REFINT_MIN),
        }
    }
}
