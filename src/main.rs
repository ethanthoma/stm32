#![no_main]
#![no_std]

use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};
use stm32f4::stm32f407;

#[entry]
fn main() -> ! {
    let _peripherals = stm32f407::Peripherals::take().unwrap();

    rtt_init_print!();
    loop {
        rprintln!("Hello, world!");
    }
}
