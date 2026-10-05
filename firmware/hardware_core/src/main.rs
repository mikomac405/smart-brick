#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, OutputType, Pull, Speed, SwjCfg};
use embassy_stm32::rcc::*;
use embassy_stm32::time::{hz, khz, Hertz};
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::{bind_interrupts, peripherals, usart};
use embedded_io_async::Read;
use panic_probe as _;

bind_interrupts!(struct Irqs {
    USART1 => usart::BufferedInterruptHandler<peripherals::USART1>;
});

struct Motors<'a> {
    m1: Output<'a>, // PB8 (Left wheel)
    m2: Output<'a>, // PB9 (Left wheel)
    m3: Output<'a>, // PA4 (Right wheel)
    m4: Output<'a>, // PB5 (Right wheel)
}

// Servo calibration constants (50Hz = 20,000 tick period)
const SERVO_PERIOD: u32 = 20000;

// Wheel steering servo (TIM3 CH1 - PA6)
const WHEEL_SERVO_CENTER: u32 = 1700;
const WHEEL_SERVO_LEFT: u32 = 1400;
const WHEEL_SERVO_RIGHT: u32 = 2000;

// Camera angle servo (TIM3 CH2 - PA7)
const CAMERA_SERVO_CENTER: u32 = 2000;
const CAMERA_SERVO_LEFT: u32 = 1700;
const CAMERA_SERVO_RIGHT: u32 = 2300;

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let mut config = embassy_stm32::Config::default();
    // Configure accurate 72 MHz clock using external 8 MHz crystal (HSE)
    config.rcc.hse = Some(Hse {
        freq: Hertz(8_000_000),
        mode: HseMode::Oscillator,
    });
    config.rcc.pll = Some(Pll {
        src: PllSource::HSE,
        prediv: PllPreDiv::DIV1,
        mul: PllMul::MUL9, // 8 MHz * 9 = 72 MHz
    });
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.ahb_pre = AHBPrescaler::DIV1;
    config.rcc.apb1_pre = APBPrescaler::DIV2; // 36 MHz max for APB1
    config.rcc.apb2_pre = APBPrescaler::DIV1; // 72 MHz max for APB2
    config.swj = SwjCfg::SwdOnly;
    let _p = embassy_stm32::init(config);

    info!("Init - Motors control");
    let mut _motors_control = Motors {
        m1: Output::new(_p.PB8, Level::Low, Speed::Low),
        m2: Output::new(_p.PB9, Level::Low, Speed::Low),
        m3: Output::new(_p.PA4, Level::Low, Speed::Low),
        m4: Output::new(_p.PB5, Level::Low, Speed::Low),
    };

    info!("Init - Motors PWM");
    let mut motors_pwm = SimplePwm::new(
        _p.TIM4,
        Some(PwmPin::new(_p.PB6, OutputType::PushPull)),
        Some(PwmPin::new(_p.PB7, OutputType::PushPull)),
        None,
        None,
        khz(10),
        CountingMode::EdgeAlignedUp,
    ).split();
    motors_pwm.ch1.set_duty_cycle_percent(60);
    motors_pwm.ch2.set_duty_cycle_percent(60);
    motors_pwm.ch1.enable();
    motors_pwm.ch2.enable();

    info!("Init - Servo PWM");
    let mut servo_pwm = SimplePwm::new(
        _p.TIM3,
        Some(PwmPin::new(_p.PA6, OutputType::PushPull)),
        Some(PwmPin::new(_p.PA7, OutputType::PushPull)),
        None,
        None,
        hz(50),
        CountingMode::EdgeAlignedUp,
    ).split();

    // Center both servos on startup
    servo_pwm.ch1.set_duty_cycle_fraction(WHEEL_SERVO_CENTER, SERVO_PERIOD);
    servo_pwm.ch2.set_duty_cycle_fraction(CAMERA_SERVO_CENTER, SERVO_PERIOD);
    servo_pwm.ch1.enable();
    servo_pwm.ch2.enable();
    
    info!("Init - Serial to ESP32 (USART1: PA9 TX, PA10 RX)");
    let mut uart_config = usart::Config::default();
    uart_config.baudrate = 115_200; // ESP32 configured for 115200 baud
    uart_config.rx_pull = Pull::Up; // Prevent floating line noise

    let mut tx_buffer = [0u8; 128];
    let mut rx_buffer = [0u8; 256];
    let serial_uart = usart::BufferedUart::new(
        _p.USART1,
        _p.PA10,
        _p.PA9,
        &mut tx_buffer,
        &mut rx_buffer,
        Irqs,
        uart_config,
    ).unwrap();

    let (_serial_tx, mut serial_rx) = serial_uart.split();

    let mut buf = [0u8; 64];
    info!("Listening for ESP32 serial data @ 115200 baud...");

    loop {
        match serial_rx.read(&mut buf).await {
            Ok(0) => {}
            Ok(n) => {
                let chunk = &buf[..n];
                if let Ok(s) = core::str::from_utf8(chunk) {
                    info!("ESP32 raw (len {}): {}", n, s);
                } else {
                    info!("ESP32 bytes (len {}): {=[u8]:#x}", n, chunk);
                }
            }
            Err(_e) => {
                warn!("UART receive error");
            }
        }
    }
}
