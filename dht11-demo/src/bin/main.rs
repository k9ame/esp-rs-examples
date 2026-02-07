#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
use core::cell::RefCell;
use core::ptr::addr_of_mut;
use critical_section::Mutex;
use defmt::info;
use dht11_demo2::dht11::Dht11Manager;
use embedded_graphics::{
    Drawable as _,
    mono_font::{
        MonoTextStyle,
        iso_8859_1::{ FONT_10X20},
    },
    pixelcolor::Rgb565,
    prelude::{Dimensions, DrawTarget, Point, RgbColor, Size},
    primitives::Rectangle,
    text::{Alignment, Text},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio::Level;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, gpio::Flex, system::Stack};
use esp_hal::{delay::Delay, main};
use esp_hal::{gpio, time::Rate};
use esp_hal::{interrupt::software::SoftwareInterruptControl, spi::master::Config};
use mipidsi::{Builder, interface::SpiInterface, models::ST7789};
use panic_rtt_target as _;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

// DHT11 数据存储（使用 Mutex 保护）
static DHT11_DATA: Mutex<RefCell<Option<(u8, u8)>>> = Mutex::new(RefCell::new(None));

/// 获取 DHT11 数据
pub fn get_dht11_data() -> (u8, u8) {
    critical_section::with(|cs| {
        let data = DHT11_DATA.borrow(cs).borrow();
        if let Some((t, h)) = *data {
            (t, h)
        } else {
            (0, 0)
        }
    })
}

#[allow(clippy::large_stack_frames)]
#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();
    static mut APP_CORE_STACK: Stack<8192> = Stack::new();
    let mut delay = Delay::new();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let dht11_pin = Flex::new(peripherals.GPIO40);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0);

    let cpu1_task = move || cpu1_task(&delay, dht11_pin);

    let stack = unsafe { &mut *addr_of_mut!(APP_CORE_STACK) };
    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        software_interrupt.software_interrupt0,
        software_interrupt.software_interrupt1,
        stack,
        cpu1_task,
    );

    // LCD 显示初始化
    let dc = gpio::Output::new(peripherals.GPIO15, Level::Low, Default::default());
    let mut rst = gpio::Output::new(peripherals.GPIO7, Level::Low, Default::default());
    rst.set_high();
    let cs = gpio::Output::new(peripherals.GPIO16, Level::High, Default::default());
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
        .init(&mut delay)
        .unwrap();
    // 需要清空屏幕，否则会显示上一次的内容
    display.clear(Rgb565::BLACK).unwrap();

    let mut last_temp: u8 = 255;
    let mut last_hum: u8 = 255;
    loop {
        delay.delay_millis(2000);
        // 使用 get_dht11_data() 获取温度和湿度
        let (temp, hum) = get_dht11_data();
        if temp != last_temp || hum != last_hum {
            display.clear(Rgb565::BLACK).unwrap();
            draw_text(&mut display, temp, hum).unwrap();
            last_temp = temp;
            last_hum = hum;
        }
    }
}

fn draw_text<T: DrawTarget<Color = Rgb565>>(
    display: &mut T,
    temp: u8,
    hum: u8,
) -> Result<(), T::Error> {
    let temp_str = alloc::format!("Temperature: {} °C", temp);
    let hum_str = alloc::format!("Humidity: {} %", hum);
    let temp_character_style = MonoTextStyle::new(&FONT_10X20, Rgb565::new(31, 41, 0)); // White
    let hum_character_style = MonoTextStyle::new(&FONT_10X20, Rgb565::new(0, 63, 31)); // White

    Text::with_alignment(
        &temp_str,
        display.bounding_box().center() + Point::new(0, -20),
        temp_character_style,
        Alignment::Center,
    )
    .draw(display)?;
    Text::with_alignment(
        &hum_str,
        display.bounding_box().center() + Point::new(0, -40),
        hum_character_style,
        Alignment::Center,
    )
    .draw(display)?;
    Ok(())
}
fn cpu1_task(delay: &Delay, dht11_pin: Flex<'static>) -> ! {
    let mut dht11 = Dht11Manager::new(dht11_pin, *delay);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    loop {
        delay.delay_millis(2000);
        match dht11.read() {
            Ok((temp, hum)) => {
                info!("DHT11 - Temperature: {} °C, humidity: {} %", temp, hum);
                // 保存数据到共享存储
                critical_section::with(|cs| {
                    *DHT11_DATA.borrow(cs).borrow_mut() = Some((temp, hum));
                });
            }
            Err(_) => {
                defmt::dbg!("Failed to read DHT11 sensor");
            }
        }
    }
}
