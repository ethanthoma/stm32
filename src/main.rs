#![no_main]
#![no_std]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_stm32::init(Default::default());
    info!("starting...");

    let mut green = Output::new(p.PD12, Level::High, Speed::Low);
    let mut red = Output::new(p.PD14, Level::High, Speed::Low);

    loop {
        red.set_low();
        info!("green");
        green.set_high();
        Timer::after_secs(1).await;

        green.set_low();
        info!("red");
        red.set_high();
        Timer::after_secs(1).await;
    }
}
