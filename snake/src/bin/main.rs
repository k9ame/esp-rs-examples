#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::cell::RefCell;
use critical_section::Mutex;
use defmt::println;
use defmt_rtt as _;
use esp_alloc as _;
use esp_hal::{analog::adc::{Adc, AdcConfig, Attenuation}, gpio, time::Rate};
use esp_hal::main;
use esp_hal::{
    self as hal,
    delay::Delay,
    gpio::{Input, InputConfig, Io, Pull},
    handler, ram,
    spi::master::Config, 
};
use embedded_graphics_framebuf::FrameBuf;
use embedded_graphics::{prelude::{Primitive, RgbColor}, primitives::{PrimitiveStyle, Rectangle}};
use embedded_graphics::{
    Drawable,
    pixelcolor::{Rgb565},
    prelude::{DrawTarget, Point, Size},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::{Builder, interface::SpiInterface, models::ST7789};

use joystick::snake::{Game, Direction, GRID_SIZE, SCREEN_SIZE, set_random_seed};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

// Z按钮使用GPIO中断（摇杆按下）
static Z_BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

#[allow(clippy::large_stack_frames)]
#[main]
fn main() -> ! {
    esp_alloc::heap_allocator!(size: 128 * 1024);

    let cpu_clock = hal::clock::CpuClock::max();
    let config = hal::Config::default().with_cpu_clock(cpu_clock);
    let peripherals = hal::init(config);

    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(button_handler);

    let mut delay = Delay::new();

    // 初始化随机数种子（使用 ESP32-S3 硬件 RNG）
    let mut rng = esp_hal::rng::Rng::new();
    let random_seed = rng.random();
    set_random_seed(random_seed as u16);
    println!("随机种子: {}", random_seed);

    // ========== ADC 配置（读取XY摇杆位置）==========
    let mut adc1_config = AdcConfig::new();
    let mut x_adc = adc1_config.enable_pin(peripherals.GPIO1, Attenuation::_11dB);
    let mut y_adc = adc1_config.enable_pin(peripherals.GPIO2, Attenuation::_11dB);
    let mut adc1 = Adc::new(peripherals.ADC1, adc1_config);

    // ========== Z按钮配置（GPIO中断检测按下）==========
    let z_pin = peripherals.GPIO35;
    let config = InputConfig::default().with_pull(Pull::Up);
    let mut z_button = Input::new(z_pin, config);

    critical_section::with(|cs| {
        z_button.listen(esp_hal::gpio::Event::FallingEdge);
        Z_BUTTON.borrow_ref_mut(cs).replace(z_button);
    });

    // ========== LCD 显示初始化 ==========
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
    display.clear(Rgb565::BLACK).unwrap();

    // 创建帧缓冲区（全屏 240x240，使用 Box 分配到堆上）
    let data_boxed = alloc::boxed::Box::new([Rgb565::BLACK; 240 * 240]);
    let data: &'static mut [Rgb565; 240 * 240] = alloc::boxed::Box::leak(data_boxed);
    let mut fbuf: FrameBuf<Rgb565, &'static mut [Rgb565; 240 * 240]> = FrameBuf::new(data, 240, 240);

    // 方向判断参数
    const CENTER: u16 = 3900;       // 摇杆中点值
    const DEAD_ZONE: u16 = 100;     // 死区范围

    // 初始化游戏
    let mut game = Game::new();
    println!("贪吃蛇游戏开始！");

    loop {
        // 读取摇杆
        let x_value: u16 = nb::block!(adc1.read_oneshot(&mut x_adc)).unwrap();
        let y_value: u16 = nb::block!(adc1.read_oneshot(&mut y_adc)).unwrap();

        // 打印摇杆值（调试用）
        // println!("X: {}, Y: {}", x_value, y_value);

        // 方向判断（使用原始代码的逻辑）
        let x_dir = if x_value < CENTER - DEAD_ZONE {
            -1  // 左
        } else if x_value > CENTER + DEAD_ZONE {
            1   // 右
        } else {
            0   // 居中
        };

        let y_dir = if y_value < CENTER - DEAD_ZONE {
            -1  // 上
        } else if y_value > CENTER + DEAD_ZONE {
            1   // 下
        } else {
            0   // 居中
        };

        // 根据方向组合确定最终方向
        let current_dir = match (x_dir, y_dir) {
            (-1, 0) => Some(Direction::Left),
            (1, 0) => Some(Direction::Right),
            (0, -1) => Some(Direction::Up),
            (0, 1) => Some(Direction::Down),
            (-1, -1) => Some(Direction::Left),
            (-1, 1) => Some(Direction::Left),
            (1, -1) => Some(Direction::Right),
            (1, 1) => Some(Direction::Right),
            _ => None,
        };

        // 直接设置方向
        if let Some(dir) = current_dir {
            println!("检测到方向: {:?}", dir);
            game.set_direction(dir);
        }

        // 游戏更新
        game.update();

        // 游戏结束检测
        if game.game_over {
            println!("游戏结束！最终得分: {}", game.score);
            delay.delay_millis(2000);
            game.reset();
            println!("游戏重新开始！");
            continue;
        }

        // 绘制游戏到帧缓冲区
        draw_game_to_buffer(&mut fbuf, &game);

        // 将帧缓冲区绘制到屏幕
        let area = Rectangle::new(Point::new(0, 0), Size::new(240, 240));
        display.fill_contiguous(&area, fbuf.data.iter().copied()).unwrap();

        // 游戏速度控制
        delay.delay_millis(150);
    }
}

/// 绘制游戏画面到帧缓冲区
fn draw_game_to_buffer(fbuf: &mut FrameBuf<Rgb565, &mut [Rgb565; 240 * 240]>, game: &Game) {
    // 清空缓冲区
    for pixel in fbuf.data.iter_mut() {
        *pixel = Rgb565::BLACK;
    }

    // 绘制蛇
    for (i, pos) in game.snake.iter().enumerate() {
        let color = if i == 0 {
            Rgb565::GREEN  // 蛇头
        } else {
            Rgb565::new(0, 200 >> 3, 0)  // 蛇身（深绿色）
        };
        
        let area = Rectangle::new(
            Point::new((pos.x * GRID_SIZE) as i32, (pos.y * GRID_SIZE) as i32),
            Size::new(GRID_SIZE as u32, GRID_SIZE as u32)
        );
        area.into_styled(PrimitiveStyle::with_fill(color)).draw(fbuf).unwrap();
    }

    // 绘制食物
    let food_area = Rectangle::new(
        Point::new((game.food.x * GRID_SIZE) as i32, (game.food.y * GRID_SIZE) as i32),
        Size::new(GRID_SIZE as u32, GRID_SIZE as u32)
    );
    food_area.into_styled(PrimitiveStyle::with_fill(Rgb565::RED)).draw(fbuf).unwrap();

    // 绘制边框
    let border_style = PrimitiveStyle::with_stroke(Rgb565::WHITE, 1);
    let border = Rectangle::new(Point::new(0, 0), Size::new(SCREEN_SIZE as u32, SCREEN_SIZE as u32));
    border.into_styled(border_style).draw(fbuf).unwrap();
}

#[handler]
#[ram]
fn button_handler() {
    critical_section::with(|cs| {
        let mut z_btn = Z_BUTTON.borrow_ref_mut(cs);
        if let Some(ref mut btn) = *z_btn
            && btn.is_interrupt_set() {
                println!("Button pressed!");
                btn.clear_interrupt();
            }
    });
}
