#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embedded_graphics::prelude::RgbColor;
use embedded_graphics::{
    Drawable,
    image::Image,
    pixelcolor::{Rgb565, Bgr565},
    prelude::{DrawTarget, DrawTargetExt, OriginDimensions, Point, Size},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_alloc as _;
use esp_hal::main;
use esp_hal::{clock::CpuClock, delay::Delay, gpio, spi::master::Config, time::Rate};
use mipidsi::{Builder, interface::SpiInterface, models::ST7789, options::Orientation};
use tinybmp::Bmp;
use tinytga::Tga;
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
    // static mut APP_CORE_STACK: Stack<8192> = Stack::new();
    let mut delay = Delay::new();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

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
        .color_order(mipidsi::options::ColorOrder::Rgb)
        .invert_colors(mipidsi::options::ColorInversion::Inverted)
        .init(&mut delay)
        .unwrap();
    // 需要清空屏幕，否则会显示上一次的内容
    display.clear(Rgb565::BLACK).unwrap();

    let data = include_bytes!("../../jing.tga");
    let img: Tga<Rgb565> = Tga::from_slice(data).unwrap();

    // let data = include_bytes!("../../jing.bmp");
    // let img: Bmp<Bgr565> = Bmp::from_slice(data).unwrap();


    let image = Image::new(&img, Point::new(64, 64));
    image.draw(&mut display.color_converted()).unwrap();

    loop {
        delay.delay_millis(500);
        
    }
}
