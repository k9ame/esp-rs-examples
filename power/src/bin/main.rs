#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_alloc as _;
use esp_hal::{self as hal, gpio::{Level, Output, OutputConfig}};
use esp_hal::main;

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
#[main]
fn main() -> ! {
    let cpu_clock = hal::clock::CpuClock::max();
    let config = hal::Config::default().with_cpu_clock(cpu_clock);
    let p = hal::init(config);

    // 配置GPIO38为继电器控制引脚
    let mut relay_pin = Output::new(p.GPIO38, Level::Low, OutputConfig::default());
    
    // 继电器导通（设置高电平）
    relay_pin.set_high();
    
    // 简单延时等待1秒（基于CPU时钟的简单循环延时）
    // ESP32-S3 at 240MHz, 大约需要240,000,000次循环
    for _ in 0..240_000_000 / 10 {
        core::hint::black_box(());
    }
    
    // 继电器断开（设置低电平）
    relay_pin.set_low();

    // 主循环保持运行
    loop {
        // 简单延时
        for _ in 0..240_000_000 / 10 {
            core::hint::black_box(());
        }
    }
}
