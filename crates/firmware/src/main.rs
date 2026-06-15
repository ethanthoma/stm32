#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![cfg_attr(flux, flux::opts(check_overflow = "strict"))]

use defmt::{info, unwrap, warn};
use embassy_executor::{InterruptExecutor, Spawner};
use embassy_stm32::adc::{Adc, SampleTime};
use embassy_stm32::bind_interrupts;
use embassy_stm32::exti;
use embassy_stm32::gpio;
use embassy_stm32::i2c::{I2c, Master};
use embassy_stm32::interrupt;
use embassy_stm32::interrupt::InterruptExt;
use embassy_stm32::mode::{Async, Blocking};
use embassy_stm32::peripherals::{ADC1, ADC2, PA1};
use embassy_stm32::wdg::IndependentWatchdog;
use embassy_stm32::Peri;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex};
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker, Timer};
use {defmt_rtt as _, panic_probe as _};

use stm32_core::feedback::pot_position;
use stm32_core::fixed::*;
use stm32_core::joint::{transition, Event, Joint};
use stm32_core::servo::servo_pulse;

use pwm_pca9685::{Address, Channel as ServoChannel, Pca9685};

bind_interrupts!(struct Irqs {
    EXTI0 => exti::InterruptHandler<interrupt::typelevel::EXTI0>;
});

static CHANNEL: Channel<ThreadModeRawMutex, (), 4> = Channel::new();
static SETPOINT: Signal<CriticalSectionRawMutex, q16> = Signal::new();
static TARGET: Signal<ThreadModeRawMutex, q16> = Signal::new();
static SERVO_SP: Signal<ThreadModeRawMutex, q16> = Signal::new();

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
async fn task_temp(mut adc: Adc<'static, ADC1>) {
    use stm32_core::temp_convert::{to_millicelsius, to_millivolts};

    let mut vrefint = adc.enable_vrefint();
    let mut temp = adc.enable_temperature();

    loop {
        let vsense = adc.blocking_read(&mut temp, SampleTime::CYCLES480);
        let vrefint = adc.blocking_read(&mut vrefint, SampleTime::CYCLES480);

        match to_millivolts(vsense, vrefint) {
            Some(mv) => info!("internal temp: {} C", to_millicelsius(mv) as f32 / 1000.0),
            None => warn!(
                "temp: adc out of range (vsense={}, vrefint={})",
                vsense, vrefint
            ),
        }

        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn task_control(
    mut led: gpio::Output<'static>,
    mut wdg: IndependentWatchdog<'static, embassy_stm32::peripherals::IWDG>,
    mut adc: Adc<'static, ADC2>,
    mut pot: Peri<'static, PA1>,
) {
    const HZ: u64 = 100;
    let mut ticker = Ticker::every(Duration::from_hz(HZ));
    let mut ticks: u32 = 1;

    let dt_s = 1.0 / HZ as f32; // seconds per tick
    let k_p = q16::from_f32(0.1);
    let k_i = q16::from_f32(0.1);
    let k_d_eff = q16::from_f32(0.01 / dt_s); // K_D / DT
    let dt = q16::from_f32(dt_s);
    let lo = q16::from_int(-1);
    let hi = q16::from_int(1);
    let i_lo = q16::from_int(-10);
    let i_hi = q16::from_int(10);

    let mut integral = q16::from_int(0);
    let mut sp = q16::from_int(0);
    let mut v = pot_position(adc.blocking_read(&mut pot, SampleTime::CYCLES480));

    loop {
        ticker.next().await;
        wdg.pet();

        if let Some(new_sp) = SETPOINT.try_take() {
            sp = new_sp;
        }

        let v_now = pot_position(adc.blocking_read(&mut pot, SampleTime::CYCLES480));

        let e = sp - v_now;
        integral = (integral + e * dt).clamp(i_lo, i_hi);

        let p = k_p * e;
        let i = k_i * integral;
        let d = k_d_eff * (v - v_now);
        let u = (p + i + d).clamp(lo, hi);

        if ticks.is_multiple_of(5) {
            info!("sp = {}; v = {}; u = {}", sp, v_now, u);
        }

        if ticks.is_multiple_of(50) {
            led.toggle();
        }

        v = v_now;
        ticks = ticks.wrapping_add(1);
    }
}

#[embassy_executor::task]
async fn task_motion() {
    const HZ: u64 = 100; // hz
    let mut ticker = Ticker::every(Duration::from_hz(HZ));

    const A_MAX: f32 = 20.0; // units per second
    let step = q16::from_f32(A_MAX / HZ as f32);

    let mut target = q16::from_int(0);
    let mut sp = q16::from_int(0);
    loop {
        ticker.next().await;

        if let Some(new_target) = TARGET.try_take() {
            target = new_target;
        }

        if sp < target {
            sp = (sp + step).min(target);
        } else if sp > target {
            sp = (sp - step).max(target);
        }
        SETPOINT.signal(sp);
        SERVO_SP.signal(sp);
    }
}

#[embassy_executor::task]
async fn task_supervisor(mut led: gpio::Output<'static>) {
    let mut state = Joint::Home;
    loop {
        CHANNEL.receive().await;

        let (next_state, effect) = transition(state, Event::ButtonPressed);
        state = next_state;

        if effect.led_high {
            led.set_high();
        } else {
            led.set_low();
        }
        TARGET.signal(effect.target);
    }
}

#[embassy_executor::task]
async fn task_servo(i2c: I2c<'static, Blocking, Master>) {
    const PRESCALE_50HZ: u8 = 121;

    let mut pwm = unwrap!(Pca9685::new(i2c, Address::default()).ok());
    let mut configured = false;

    loop {
        let sp = SERVO_SP.wait().await;

        if !configured {
            if pwm.set_prescale(PRESCALE_50HZ).is_ok() && pwm.enable().is_ok() {
                configured = true;
                info!("servo: pca9685 configured");
            } else {
                continue;
            }
        }

        if pwm
            .set_channel_on_off(ServoChannel::C15, 0, servo_pulse(sp))
            .is_err()
        {
            warn!("servo: i2c lost, will reconfigure");
            configured = false;
        }
    }
}

static EXECUTOR_H: InterruptExecutor = InterruptExecutor::new();

#[interrupt]
#[allow(unsafe_code)]
unsafe fn UART4() {
    unsafe {
        EXECUTOR_H.on_interrupt();
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("starting...");

    interrupt::UART4.set_priority(interrupt::Priority::P6); // high

    // pd12 = green, pd13 = orange, pd14 = red, pd15 = blue
    let button = exti::ExtiInput::new(p.PA0, p.EXTI0, gpio::Pull::Down, Irqs);
    let led_green = gpio::Output::new(p.PD12, gpio::Level::Low, gpio::Speed::Low);
    let led_blue = gpio::Output::new(p.PD15, gpio::Level::Low, gpio::Speed::Low);
    let adc = Adc::new(p.ADC1);
    let pot_adc = Adc::new(p.ADC2);

    let i2c = I2c::new_blocking(p.I2C1, p.PB6, p.PB7, Default::default());

    let mut wdg = IndependentWatchdog::new(p.IWDG, 50_000);
    wdg.unleash();

    EXECUTOR_H
        .start(interrupt::UART4)
        .spawn(unwrap!(task_control(led_blue, wdg, pot_adc, p.PA1)));

    spawner.spawn(unwrap!(task_supervisor(led_green)));
    spawner.spawn(unwrap!(task_button(button)));
    spawner.spawn(unwrap!(task_temp(adc)));
    spawner.spawn(unwrap!(task_motion()));
    spawner.spawn(unwrap!(task_servo(i2c)));
}
