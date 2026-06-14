#[cfg(not(feature = "verus"))]
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::prelude::*;

verus! {

pub open spec fn trunc_div(a: int, b: int) -> int {
    if a >= 0 {
        a / b
    } else {
        -((-a) / b)
    }
}

pub fn div_trunc(a: i64, b: i64) -> (q: i64)
    requires
        b > 0,
    ensures
        q as int == trunc_div(a as int, b as int),
{
    a / b
}

} // verus!
