#![no_main]
#![no_std]

use embassy_executor::Spawner;
use embassy_stm32::Config;
use embassy_time::Timer;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let config = Config::default();
    let _p = embassy_stm32::init(config);
    rtt_init_print!();

    rprintln!("starting...");

    loop {
        rprintln!("ping");
        Timer::after_secs(1).await;
    }
}
