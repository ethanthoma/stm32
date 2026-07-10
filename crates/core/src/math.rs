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

pub fn isqrt(n: u64) -> (r: u64)
    ensures
        r * r <= n,
        n < (r + 1) * (r + 1),
        r <= 0xFFFF_FFFF,
{
    let mut lo: u64 = 0;
    let mut hi: u64 = 0xFFFF_FFFF;
    while lo < hi
        invariant
            lo <= hi <= 0xFFFF_FFFF,
            lo * lo <= n,
            n < (hi + 1) * (hi + 1),
        decreases hi - lo,
    {
        let mid: u64 = (lo + hi + 1) / 2;
        assert(lo < mid <= hi);
        assert(mid * mid <= u64::MAX) by (nonlinear_arith)
            requires
                mid <= 0xFFFF_FFFF,
        ;
        if mid * mid <= n {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

} // verus!
