#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use clk::{SevenSegmentConfig, SevenSegmentDisplay};
use defmt::println;
use embedded_graphics::prelude::{RgbColor, Size};
use embedded_graphics::{
    pixelcolor::Rgb565,
    draw_target::DrawTarget,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_alloc as _;
use esp_hal::{clock::CpuClock, delay::Delay, gpio, main, rtc_cntl::Rtc};
use esp_hal::{spi::master::Config, time::Rate};
use defmt_rtt as _;
use mipidsi::{Builder, interface::SpiInterface, models::ST7789};
use embedded_graphics_framebuf::FrameBuf;

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
    // 初始化堆内存分配器
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let mut delay = Delay::new();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // 初始化RTC
    let _rtc = Rtc::new(peripherals.LPWR);

    // LCD 显示初始化
    let dc = gpio::Output::new(peripherals.GPIO15, gpio::Level::Low, Default::default());
    let mut rst = gpio::Output::new(peripherals.GPIO7, gpio::Level::Low, Default::default());
    rst.set_high();
    let cs = gpio::Output::new(peripherals.GPIO16, gpio::Level::High, Default::default());
    let spi = esp_hal::spi::master::Spi::new(
        peripherals.SPI2,
        Config::default().with_frequency(Rate::from_mhz(30)),
    )
    .unwrap()
    .with_sck(peripherals.GPIO5)
    .with_mosi(peripherals.GPIO6);

    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let mut buffer = [0_u8; 512];

    let di = SpiInterface::new(spi_device, dc, &mut buffer);
    let mut display = Builder::new(ST7789, di)
        .reset_pin(rst)
        .display_size(240, 240)
        .color_order(mipidsi::options::ColorOrder::Rgb)
        .invert_colors(mipidsi::options::ColorInversion::Inverted)
        .init(&mut delay)
        .unwrap();

    // 清空屏幕
    display.clear(Rgb565::BLACK).unwrap();

    // 时间变量
    let mut hours: u8 = 23;
    let mut minutes: u8 = 20;
    let mut seconds: u8 = 0;

    // 创建7段数码管显示配置
    // digit_size: 数字尺寸, digit_spacing: 数字间距, segment_width: 段宽度
    let ss_config = SevenSegmentConfig::new(Size::new(25, 45), 4, 5);
    let ss_display = SevenSegmentDisplay::new(ss_config);

    // 创建帧缓冲区用于绘制7段数码管
    let fbuf_width: usize = 240;
    let fbuf_height: usize = 80;
    let mut data = [Rgb565::BLACK; 240 * 80];
    let mut fbuf: FrameBuf<Rgb565, &mut [Rgb565; 240 * 80]> = FrameBuf::new(&mut data, fbuf_width, fbuf_height);

    loop {
        // 延时 1 秒
        delay.delay_millis(1000);

        // 更新时间
        seconds += 1;
        if seconds >= 60 {
            seconds = 0;
            minutes += 1;
            if minutes >= 60 {
                minutes = 0;
                hours += 1;
                if hours >= 24 {
                    hours = 0;
                }
            }
        }

        // Debug 打印
        println!("time: {:02}:{:02}:{:02}", hours, minutes, seconds);

        // 使用7段数码管显示时间
        let color = Rgb565::GREEN;
        // 未点亮的段显示为暗淡的绿色（约1/4亮度）
        let inactive_color = Rgb565::new(0, 4, 0);
        ss_display.draw_time(&mut display, &mut fbuf, hours, minutes, seconds, color, Some(inactive_color)).unwrap();
    }
}

