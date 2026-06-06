#![no_main]
#![no_std]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::bind_interrupts;
use embassy_stm32::exti;
use embassy_stm32::gpio;
use embassy_stm32::interrupt;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    EXTI0 => exti::InterruptHandler<interrupt::typelevel::EXTI0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_stm32::init(Default::default());
    info!("starting...");

    let mut green = gpio::Output::new(p.PD12, gpio::Level::Low, gpio::Speed::Low);

    let mut user_button = exti::ExtiInput::new(p.PA0, p.EXTI0, gpio::Pull::Down, Irqs);
    let mut on = false;

    loop {
        user_button.wait_for_rising_edge().await;
        on = !on;
        let level = if on {
            gpio::Level::High
        } else {
            gpio::Level::Low
        };
        green.set_level(level);
        info!("led = {}", on);
        Timer::after_millis(20).await;
        user_button.wait_for_falling_edge().await;
        Timer::after_millis(20).await;
    }
}
