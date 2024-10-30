#![no_main]
#![no_std]

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f4::stm32f407;

#[entry]
fn main() -> ! {
    let _peripherals = stm32f407::Peripherals::take().unwrap();

    loop {}
}
