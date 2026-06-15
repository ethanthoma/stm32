#[cfg(not(feature = "verus"))]
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::prelude::*;

use crate::fixed::q16;

verus! {

pub enum Joint {
    Home,
    Extended,
}

pub enum Event {
    ButtonPressed,
}

pub struct Effect {
    pub led_high: bool,
    pub target: q16,
}

pub open spec fn pressed(state: Joint) -> Joint {
    match state {
        Joint::Home => Joint::Extended,
        Joint::Extended => Joint::Home,
    }
}

pub open spec fn target_of(state: Joint) -> int {
    match state {
        Joint::Home => 0,
        Joint::Extended => 50 * crate::fixed::ONE,
    }
}

pub fn transition(state: Joint, event: Event) -> (r: (Joint, Effect))
    ensures
        r.0 == pressed(state),
        r.1.led_high == (r.0 == Joint::Extended),
        r.1.target.val() == target_of(r.0),
{
    match (state, event) {
        (Joint::Home, Event::ButtonPressed) => (
            Joint::Extended,
            Effect { led_high: true, target: q16::from_int(50) },
        ),
        (Joint::Extended, Event::ButtonPressed) => (
            Joint::Home,
            Effect { led_high: false, target: q16::from_int(0) },
        ),
    }
}

proof fn no_drift(s: Joint)
    ensures
        pressed(pressed(s)) == s,
{
}

proof fn home_reachable_within_two_presses(s: Joint)
    ensures
        pressed(s) == Joint::Home || pressed(pressed(s)) == Joint::Home,
{
}

} // verus!
