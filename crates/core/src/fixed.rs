#[cfg(not(feature = "verus"))]
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::prelude::*;

use crate::math::*;

verus! {

pub const ONE: i32 = 65536;

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct q16(i32);

impl q16 {
    pub closed spec fn val(self) -> int {
        self.0 as int
    }

    pub exec fn from_bits(bits: i32) -> (q: q16)
        ensures
            q.val() == bits as int,
    {
        q16(bits)
    }

    pub const MAX_INT: i32 = 32767;

    pub const MIN_INT: i32 = -32768;

    pub fn from_int(val: i32) -> (q: q16)
        ensures
            val < Self::MIN_INT ==> q.val() == Self::MIN_INT * ONE,
            val > Self::MAX_INT ==> q.val() == Self::MAX_INT * ONE,
            Self::MIN_INT <= val <= Self::MAX_INT ==> q.val() == val * ONE,
    {
        if val < Self::MIN_INT {
            q16(Self::MIN_INT * ONE)
        } else if val > Self::MAX_INT {
            q16(Self::MAX_INT * ONE)
        } else {
            q16(val * ONE)
        }
    }

    pub open spec fn mul_bits(self, other: q16) -> int {
        let p = trunc_div(self.val() * other.val(), ONE as int);
        if p > i32::MAX {
            i32::MAX as int
        } else if p < i32::MIN {
            i32::MIN as int
        } else {
            p
        }
    }

    #[verifier::nonlinear]
    pub fn saturating_mul(self, other: q16) -> (q: q16)
        ensures
            q.val() == self.mul_bits(other),
    {
        let prod: i64 = self.0 as i64 * other.0 as i64;
        let scaled: i64 = div_trunc(prod, ONE as i64);

        if scaled > i32::MAX as i64 {
            q16(i32::MAX)
        } else if scaled < i32::MIN as i64 {
            q16(i32::MIN)
        } else {
            q16(scaled as i32)
        }
    }
}

#[cfg(feature = "verus")]
impl vstd::std_specs::ops::MulSpecImpl for q16 {
    open spec fn obeys_mul_spec() -> bool {
        true
    }

    open spec fn mul_req(self, rhs: q16) -> bool {
        true
    }

    closed spec fn mul_spec(self, rhs: q16) -> q16 {
        q16(self.mul_bits(rhs) as i32)
    }
}

impl core::ops::Mul for q16 {
    type Output = q16;

    fn mul(self, rhs: q16) -> q16 {
        self.saturating_mul(rhs)
    }
}

} // verus!
