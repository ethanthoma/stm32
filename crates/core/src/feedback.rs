#[cfg(not(feature = "verus"))]
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::prelude::*;

use crate::fixed::q16;

verus! {

pub const ADC_MAX: u16 = 4095;

pub const POSITION_MAX: i32 = 50;

pub fn pot_position(raw: u16) -> (p: q16)
    ensures
        0 <= p.val() <= POSITION_MAX * crate::fixed::ONE,
{
    let full: i64 = q16::from_int(POSITION_MAX).to_bits() as i64;
    let r: i64 = raw as i64;
    assert(r * full <= 65535 * full) by (nonlinear_arith)
        requires
            0 <= r <= 65535,
            0 <= full,
    ;
    let scaled = r * full / ADC_MAX as i64;
    let bits: i64 = if scaled < 0 {
        0
    } else if scaled > full {
        full
    } else {
        scaled
    };
    q16::from_bits(bits as i32)
}

} // verus!
