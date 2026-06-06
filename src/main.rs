#![no_main]
#![no_std]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_stm32::init(Default::default());
    info!("starting...");

    let mut green = Output::new(p.PD12, Level::Low, Speed::Low);

    let i0 = Input::new(p.PA0, Pull::Down);
    let mut user_button = ExtiInput::new(i0, p.EXTI0);

    loop {
        user_button.wait_for_rising_edge().await;
        info!("pressed!");
        green.set_high();

        user_button.wait_for_falling_edge().await;
        green.set_low();
    }
}
