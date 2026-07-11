#[cfg(not(feature = "verus"))]
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::prelude::*;

use crate::fixed::{q16, ONE};

verus! {

pub fn velocity_sq_frac(done: u32, total: u32, ramp: u32) -> (v: q16)
    requires
        ramp > 0,
        done <= total,
    ensures
        0 <= v.val() <= ONE,
{
    let accel: i64 = done as i64 * ONE as i64 / ramp as i64;
    let decel: i64 = (total - done) as i64 * ONE as i64 / ramp as i64;
    let m: i64 = if accel <= decel {
        accel
    } else {
        decel
    };
    let bits: i64 = if m < 0 {
        0
    } else if m > ONE as i64 {
        ONE as i64
    } else {
        m
    };
    q16::from_bits(bits as i32)
}

pub fn velocity(done: u32, total: u32, ramp: u32, v_max: q16) -> (v: q16)
    requires
        ramp > 0,
        done <= total,
        v_max.val() >= 0,
    ensures
        0 <= v.val() <= v_max.val(),
{
    let frac = velocity_sq_frac(done, total, ramp);
    let scale = frac.sqrt();
    assert(0 <= scale.val() <= ONE) by (nonlinear_arith)
        requires
            scale.val() >= 0,
            scale.val() * scale.val() <= frac.val() * ONE,
            frac.val() <= ONE,
    ;
    let v = v_max.saturating_mul(scale);
    proof {
        assert(v_max.val() * scale.val() >= 0) by (nonlinear_arith)
            requires
                v_max.val() >= 0,
                scale.val() >= 0,
        ;
        assert(v_max.val() * scale.val() <= ONE * v_max.val()) by (nonlinear_arith)
            requires
                v_max.val() >= 0,
                scale.val() <= ONE,
        ;
        vstd::arithmetic::div_mod::lemma_multiply_divide_le(
            v_max.val() * scale.val(),
            ONE as int,
            v_max.val(),
        );
        vstd::arithmetic::div_mod::lemma_div_pos_is_pos(v_max.val() * scale.val(), ONE as int);
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

#[derive(Clone, Copy)]
pub struct MoveParams {
    pub total: u32,
    pub ramp: u32,
    pub v_max: q16,
    pub v_min: q16,
    pub timer_hz: u32,
}

impl MoveParams {
    pub open spec fn wf(self) -> bool {
        &&& self.ramp > 0
        &&& self.v_min.val() > 0
        &&& self.v_max.val() >= 0
        &&& self.v_max.val() <= self.timer_hz as int * ONE as int
        &&& self.v_min.val() <= self.timer_hz as int * ONE as int
    }
}

pub fn step_at(params: MoveParams, done: u32) -> (ticks: u64)
    requires
        params.wf(),
        done <= params.total,
    ensures
        1 <= ticks,
        ticks <= params.timer_hz as int * ONE as int / params.v_min.val(),
{
    let v = velocity(done, params.total, params.ramp, params.v_max);
    step_interval(v, params.v_min, params.timer_hz)
}

} // verus!
