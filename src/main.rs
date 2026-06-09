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
use embassy_sync::signal::Signal;
use embassy_time::Duration;
use embassy_time::Ticker;
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

static SETPOINT: Signal<ThreadModeRawMutex, f32> = Signal::new();
static TARGET: Signal<ThreadModeRawMutex, f32> = Signal::new();

enum Joint {
    Home,
    Extended,
}

#[embassy_executor::task]
async fn task_control() {
    const HZ: u64 = 100;
    let mut ticker = Ticker::every(Duration::from_hz(HZ));
    let mut ticks = 1;

    const K: u32 = 100;
    const K_P: f32 = 0.1;
    const K_I: f32 = 0.1;
    const K_D: f32 = 0.01;

    let dt: f32 = 1. / HZ as f32;
    let tau: f32 = 2.0;

    let mut v = 0.0;
    let mut u;
    let mut integral = 0.0;
    let mut sp = 0.0;
    let mut v_prev = 0.;
    loop {
        ticker.next().await;

        if let Some(new_sp) = SETPOINT.try_take() {
            sp = new_sp;
        }

        if ticks >= 5 {
            info!("sp = {}; v = {}", sp, v);
            ticks = 0;
        }

        let e = sp - v;
        let p = K_P * e;
        let i = K_I * integral;
        let d = -K_D * (v - v_prev) / dt;
        let u_raw = p + i + d;
        u = u_raw.clamp(-1.0, 1.0);

        if u == u_raw {
            integral += e * dt;
        }
        v_prev = v;

        v += (K as f32 * u - v) * (dt / tau);

        ticks += 1;
    }
}

#[embassy_executor::task]
async fn task_motion() {
    const HZ: u64 = 100; // hz
    let mut ticker = Ticker::every(Duration::from_hz(HZ));

    const A_MAX: f32 = 20.0;

    let dt: f32 = 1. / HZ as f32;

    let mut target = 0.;
    let mut sp = 0.;
    loop {
        ticker.next().await;

        if let Some(new_target) = TARGET.try_take() {
            target = new_target;
        }

        let step = A_MAX * dt;
        if sp < target {
            sp = (sp + step).min(target);
        } else if sp > target {
            sp = (sp - step).max(target);
        }
        SETPOINT.signal(sp);
    }
}

#[embassy_executor::task]
async fn task_supervisor(mut led: gpio::Output<'static>) {
    let mut state = Joint::Home;
    loop {
        CHANNEL.receive().await;

        match state {
            Joint::Home => {
                state = Joint::Extended;
                led.set_high();
                TARGET.signal(50.);
            }
            Joint::Extended => {
                led.set_low();
                state = Joint::Home;
                TARGET.signal(0.);
            }
        }
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
    spawner.spawn(unwrap!(task_blink(led_red)));
    spawner.spawn(unwrap!(task_breathe(pwm)));
    spawner.spawn(unwrap!(task_temp(adc)));
    spawner.spawn(unwrap!(task_control()));
    spawner.spawn(unwrap!(task_motion()));
    spawner.spawn(unwrap!(task_supervisor(led_green)));
}
