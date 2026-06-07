#![no_main]
#![no_std]

use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_stm32::adc::Adc;
use embassy_stm32::adc::SampleTime;
use embassy_stm32::bind_interrupts;
use embassy_stm32::exti;
use embassy_stm32::gpio;
use embassy_stm32::interrupt;
use embassy_stm32::mode::Async;
use embassy_stm32::peripherals;
use embassy_stm32::peripherals::ADC1;
use embassy_stm32::time;
use embassy_stm32::timer::simple_pwm;
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

#[embassy_executor::task]
async fn task_breathe(mut pwm: simple_pwm::SimplePwm<'static, peripherals::TIM4>) {
    let mut ch = pwm.ch4();
    ch.enable();
    let steps = 100;

    loop {
        for i in (0..=steps).chain((1..steps).rev()) {
            ch.set_duty_cycle_fraction(i * i, steps * steps);
            Timer::after_millis(10).await;
        }
    }
}

#[embassy_executor::task]
async fn task_temp(mut adc: Adc<'static, ADC1>) {
    let mut vrefint = adc.enable_vrefint();
    let convert_to_millivolts = |vsense: u16, vrefint: u16| {
        const V_REFINT: u32 = 1210; // mv
        (u32::from(vsense) * V_REFINT / u32::from(vrefint)) as u16
    };

    let mut temp = adc.enable_temperature();
    let convert_to_celcius = |vsense: u16, vrefint: u16| {
        const V_25: i32 = 760; // mv
        const AVG_SLOPE: f32 = 2.5; // mv/C

        ((convert_to_millivolts(vsense, vrefint) as i32 - V_25) as f32 / AVG_SLOPE) + 25.
    };

    loop {
        let vsense = adc.blocking_read(&mut temp, SampleTime::CYCLES480);
        let vrefint = adc.blocking_read(&mut vrefint, SampleTime::CYCLES480);
        info!("internal temp: {} C", convert_to_celcius(vsense, vrefint));
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("starting...");

    // pd12 = green, pd13 = orange, pd14 = red, pd15 = blue
    let button = exti::ExtiInput::new(p.PA0, p.EXTI0, gpio::Pull::Down, Irqs);
    let led_red = gpio::Output::new(p.PD14, gpio::Level::Low, gpio::Speed::Low);
    let led_green = gpio::Output::new(p.PD12, gpio::Level::Low, gpio::Speed::Low);

    let pin = simple_pwm::PwmPin::new(p.PD15, gpio::OutputType::PushPull);
    let pwm = simple_pwm::SimplePwm::new(
        p.TIM4,
        None,
        None,
        None,
        Some(pin),
        time::khz(10),
        Default::default(),
    );
    let adc = Adc::new(p.ADC1);

    spawner.spawn(unwrap!(task_button(button)));
    spawner.spawn(unwrap!(task_led(led_green)));
    spawner.spawn(unwrap!(task_blink(led_red)));
    spawner.spawn(unwrap!(task_breathe(pwm)));
    spawner.spawn(unwrap!(task_temp(adc)));
}
