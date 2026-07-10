#[cfg(not(feature = "verus"))]
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::prelude::*;

use crate::fixed::{q16, ONE};

verus! {

pub fn velocity_sq_frac(done: q16, ramp: q16, total: q16) -> (v: q16)
    ensures
        0 <= v.val() <= crate::fixed::ONE,
{
    let zero = q16::from_int(0);
    let full = q16::from_int(1);

    let accel = (done / ramp).clamp(zero, full);
    let decel = ((total - done) / ramp).clamp(zero, full);

    accel.min(decel)
}

pub fn velocity(done: q16, ramp: q16, total: q16, v_max: q16) -> (v: q16)
    requires
        v_max.val() >= 0,
    ensures
        0 <= v.val() <= v_max.val(),
{
    let frac = velocity_sq_frac(done, ramp, total);
    let scale = frac.sqrt();
    assert(0 <= scale.val() <= crate::fixed::ONE) by (nonlinear_arith)
        requires
            scale.val() >= 0,
            scale.val() * scale.val() <= frac.val() * crate::fixed::ONE,
            frac.val() <= crate::fixed::ONE,
    ;
    let v = v_max.saturating_mul(scale);
    proof {
        assert(v_max.val() * scale.val() >= 0) by (nonlinear_arith)
            requires
                v_max.val() >= 0,
                scale.val() >= 0,
        ;
        assert(v_max.val() * scale.val() <= crate::fixed::ONE * v_max.val()) by (nonlinear_arith)
            requires
                v_max.val() >= 0,
                scale.val() <= crate::fixed::ONE,
        ;
        vstd::arithmetic::div_mod::lemma_multiply_divide_le(
            v_max.val() * scale.val(),
            crate::fixed::ONE as int,
            v_max.val(),
        );
        vstd::arithmetic::div_mod::lemma_div_pos_is_pos(
            v_max.val() * scale.val(),
            crate::fixed::ONE as int,
        );
    }
    v
}

pub fn step_interval(velocity: q16, velocity_min: q16, timer_hz: u32) -> (ticks: u64)
    requires
        velocity_min.val() > 0,
        velocity.val() >= 0,
        velocity.val() <= timer_hz as int * ONE as int,
        velocity_min.val() <= timer_hz as int * ONE as int,
    ensures
        1 <= ticks,
        ticks <= timer_hz as int * ONE as int / velocity_min.val(),
{
    let denom: i32 = if velocity.to_bits() >= velocity_min.to_bits() {
        velocity.to_bits()
    } else {
        velocity_min.to_bits()
    };
    let num: u64 = timer_hz as u64 * ONE as u64;
    let ticks: u64 = num / denom as u64;
    proof {
        assert(num as int == timer_hz as int * ONE as int);
        vstd::arithmetic::div_mod::lemma_div_is_ordered_by_denominator(
            num as int,
            velocity_min.val(),
            denom as int,
        );
        vstd::arithmetic::div_mod::lemma_div_is_ordered(denom as int, num as int, denom as int);
        vstd::arithmetic::div_mod::lemma_div_by_self(denom as int);
    }
    ticks
}

} // verus!
