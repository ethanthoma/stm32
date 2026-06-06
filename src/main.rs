#![no_main]
#![no_std]

use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_stm32::bind_interrupts;
use embassy_stm32::exti;
use embassy_stm32::gpio;
use embassy_stm32::interrupt;
use embassy_stm32::mode::Async;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    EXTI0 => exti::InterruptHandler<interrupt::typelevel::EXTI0>;
});

static CHANNEL: Channel<ThreadModeRawMutex, (), 4> = Channel::new();

#[embassy_executor::task]
async fn task_button(mut button: exti::ExtiInput<'static, Async>) {
    loop {
        button.wait_for_rising_edge().await;
        info!("pressed!");
        CHANNEL.send(()).await;
        Timer::after_millis(20).await;
        button.wait_for_falling_edge().await;
        Timer::after_millis(20).await;
    }
}

#[embassy_executor::task]
async fn task_led(mut led: gpio::Output<'static>) {
    loop {
        CHANNEL.receive().await;
        led.toggle();
        info!("led = {}", led.get_output_level())
    }
}

#[embassy_executor::task]
async fn task_blink(mut led: gpio::Output<'static>) {
    loop {
        led.toggle();
        info!("blink = {}", led.get_output_level());
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p = embassy_stm32::init(Default::default());
    info!("starting...");

    let button = exti::ExtiInput::new(p.PA0, p.EXTI0, gpio::Pull::Down, Irqs);
    let led_green = gpio::Output::new(p.PD12, gpio::Level::Low, gpio::Speed::Low);
    let led_red = gpio::Output::new(p.PD14, gpio::Level::Low, gpio::Speed::Low);

    spawner.spawn(unwrap!(task_button(button)));
    spawner.spawn(unwrap!(task_led(led_green)));
    spawner.spawn(unwrap!(task_blink(led_red)));

    loop {
        Timer::after_secs(1).await;
    }
}
