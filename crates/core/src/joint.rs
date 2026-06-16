#[cfg(not(feature = "verus"))]
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
use vstd::prelude::*;

use crate::fixed::q16;

verus! {

pub enum Joint {
    Home,
    Extended,
    Estopped,
}

pub enum Event {
    ButtonPressed,
    Estop,
}

pub struct Effect {
    pub led_high: bool,
    pub target: q16,
    pub enabled: bool,
}

pub open spec fn next_state(state: Joint, event: Event) -> Joint {
    match event {
        Event::Estop => Joint::Estopped,
        Event::ButtonPressed => match state {
            Joint::Home => Joint::Extended,
            Joint::Extended => Joint::Home,
            Joint::Estopped => Joint::Home,
        },
    }
}

pub open spec fn target_of(state: Joint) -> int {
    match state {
        Joint::Home => 0,
        Joint::Extended => 50 * crate::fixed::ONE,
        Joint::Estopped => 0,
    }
}

pub fn transition(state: Joint, event: Event) -> (r: (Joint, Effect))
    ensures
        r.0 == next_state(state, event),
        r.1.enabled == (r.0 != Joint::Estopped),
        r.1.led_high == (r.0 == Joint::Extended),
        r.1.target.val() == target_of(r.0),
{
    match (state, event) {
        (_, Event::Estop) => (
            Joint::Estopped,
            Effect { led_high: false, target: q16::from_int(0), enabled: false },
        ),
        (Joint::Home, Event::ButtonPressed) => (
            Joint::Extended,
            Effect { led_high: true, target: q16::from_int(50), enabled: true },
        ),
        (Joint::Extended, Event::ButtonPressed) => (
            Joint::Home,
            Effect { led_high: false, target: q16::from_int(0), enabled: true },
        ),
        (Joint::Estopped, Event::ButtonPressed) => (
            Joint::Home,
            Effect { led_high: false, target: q16::from_int(0), enabled: true },
        ),
    }
}

proof fn estop_always_disables(s: Joint)
    ensures
        next_state(s, Event::Estop) == Joint::Estopped,
{
}

proof fn button_clears_estop()
    ensures
        next_state(Joint::Estopped, Event::ButtonPressed) == Joint::Home,
{
}

} // verus!
