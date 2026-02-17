#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use alloc::boxed::Box;
use defmt::println;
use defmt_rtt as _;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{DrawTarget, Point, RgbColor, Size};
use embedded_graphics::primitives::Rectangle;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_alloc as _;
use esp_hal::main;
use esp_hal::{clock::CpuClock, delay::Delay, gpio, spi::master::Config, time::Rate};
use tui::regular_font::REGULAR_FONT;
use mipidsi::{Builder, interface::SpiInterface, models::ST7789};
use mousefood::fonts::MONO_10X20;
use mousefood::prelude::*;
use ratatui::{
    Frame, Terminal,
    layout::Rect,
    style::Style,
    widgets::{Block, Paragraph},
};



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
    let mut delay = Delay::new();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);

    // LCD init
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
    let buffer = Box::leak(Box::new([0_u8; 512]));

    let di = SpiInterface::new(spi_device, dc, buffer);
    let mut display = Builder::new(ST7789, di)
        .reset_pin(rst)
        .display_size(240, 240)
        .color_order(mipidsi::options::ColorOrder::Rgb)
        .invert_colors(mipidsi::options::ColorInversion::Inverted)
        .init(&mut delay)
        .unwrap();

    let config = EmbeddedBackendConfig {
        font_regular: MONO_10X20,
        // font_bold: Some(REGULAR_FONT),
        // font_italic:  Some(REGULAR_FONT),
        // color_theme:  theme,
        ..Default::default()
    };

    let backend = EmbeddedBackend::new(&mut display, config);

    let mut terminal = Terminal::new(backend).expect("failed to create terminal");

    terminal.draw(draw).unwrap();
    loop {
        // terminal
        //     .draw(|f| {
        //         let area = f.area();
        //         f.render_widget(Block::bordered().title("Test"), area);
        //     })
        //     .unwrap();

        // println!("Here is loop.");
        delay.delay_millis(2000);
    }
}

fn draw(frame: &mut Frame) {
    let block = Block::bordered()
        .title("Mousefood")
        .style(Style::new().fg(ratatui::style::Color::Green));
    let paragraph = Paragraph::new("Hello from Mousefood!")
        .style(Style::new().fg(ratatui::style::Color::Red))
        .block(block);
    frame.render_widget(paragraph, frame.area());
}
