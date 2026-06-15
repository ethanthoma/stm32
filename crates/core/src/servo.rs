#[cfg(not(feature = "verus"))]
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::prelude::*;

use crate::fixed::q16;

verus! {

pub const PULSE_MIN: u16 = 205;

pub const PULSE_MAX: u16 = 410;

pub const SETPOINT_MAX: i32 = 50;

pub fn servo_pulse(setpoint: q16) -> (p: u16)
    ensures
        PULSE_MIN <= p <= PULSE_MAX,
{
    let span = (PULSE_MAX - PULSE_MIN) as i32;
    let clamped = setpoint.clamp(q16::from_int(0), q16::from_int(SETPOINT_MAX));
    let raw = clamped.to_bits() * span / (SETPOINT_MAX * crate::fixed::ONE);
    let frac = if raw < 0 {
        0
    } else if raw > span {
        span
    } else {
        raw
    };
    PULSE_MIN + frac as u16
}

} // verus!
