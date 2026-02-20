#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use critical_section::Mutex;
use esp_alloc as _;
use esp_hal::{self as hal, delay::Delay, gpio::{Input, InputConfig, Io, Pull}, handler, ram};
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::main;
use defmt::println;
use defmt_rtt as _;
use core::cell::RefCell;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]

// Z按钮使用GPIO中断（摇杆按下）
static Z_BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

#[main]
fn main() -> ! {
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let cpu_clock = hal::clock::CpuClock::max();
    let config = hal::Config::default().with_cpu_clock(cpu_clock);
    let peripherals = hal::init(config);

    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(button_handler);
   
    let delay = Delay::new();

    // ========== ADC 配置（读取XY摇杆位置）==========
    // ESP32-S3 ADC1 可用引脚: GPIO1-GPIO10
    // 请将 X轴 连接到 GPIO1 (ADC1_CH0)
    // 请将 Y轴 连接到 GPIO2 (ADC1_CH1)
    // 
    // 注意：GPIO35/36/37 不支持 ADC！
    
    let mut adc1_config = AdcConfig::new();
    let mut x_adc = adc1_config.enable_pin(peripherals.GPIO1, Attenuation::_11dB);
    let mut y_adc = adc1_config.enable_pin(peripherals.GPIO2, Attenuation::_11dB);
    let mut adc1 = Adc::new(peripherals.ADC1, adc1_config);

    // ========== Z按钮配置（GPIO中断检测按下）==========
    // GPIO35 用于检测摇杆按钮按下
    let z_pin = peripherals.GPIO35;
    let config = InputConfig::default().with_pull(Pull::Up);
    let mut z_button = Input::new(z_pin, config);
    
    critical_section::with(|cs| {
        z_button.listen(esp_hal::gpio::Event::FallingEdge);
        Z_BUTTON.borrow_ref_mut(cs).replace(z_button);
    });

    println!("Joystick initialized!");
    println!("X-axis: GPIO1, Y-axis: GPIO2, Button: GPIO35");

    loop {
        // 读取 X 轴 ADC 值
        let x_value: u16 = nb::block!(adc1.read_oneshot(&mut x_adc)).unwrap();
        // 读取 Y 轴 ADC 值
        let y_value: u16 = nb::block!(adc1.read_oneshot(&mut y_adc)).unwrap();
        
        // ADC 值范围: 0-4095 (12位 ADC)
        // 中点值约为 2048（摇杆居中）
        println!("X: {}, Y: {}", x_value, y_value);
        
        delay.delay_millis(100);
    }
}

#[handler]
#[ram]
fn button_handler() {
    critical_section::with(|cs| {
        let mut z_btn = Z_BUTTON.borrow_ref_mut(cs);
        if let Some(ref mut btn) = *z_btn {
            if btn.is_interrupt_set() {
                println!("Button pressed!");
                btn.clear_interrupt();
            }
        }
    });
}
