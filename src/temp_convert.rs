#![allow(unexpected_cfgs)]

#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::arithmetic::div_mod::{lemma_div_is_ordered, lemma_div_is_ordered_by_denominator};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

// internal reference voltage, mV
pub const V_REFINT_MV: u32 = 1210;

// 12-bit ADC full scale
// (1 << 12) - 1
// TODO: compute this nicely
#[allow(dead_code)]
pub const ADC_MAX: u16 = 4095;

// smallest vrefint keeping the result within u16 (and != 0)
// TODO: compute this nicely
#[allow(dead_code)]
pub const V_REFINT_MIN: u16 = 76;

pub fn to_millivolts(vsense: u16, vrefint: u16) -> (mv: u16)
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

} // verus!
