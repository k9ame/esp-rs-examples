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
use embedded_graphics::{
    prelude::RgbColor, 
    primitives::Rectangle,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    text::Text,
    Drawable,
    pixelcolor::Rgb565,
    prelude::{Point, Size},
    draw_target::DrawTarget,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::{Builder, interface::SpiInterface, models::ST7789};

use tetris::{TetrisGame, TetrominoType, BOARD_WIDTH, BOARD_HEIGHT, CELL_SIZE, set_random_seed};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

// Z按钮使用GPIO中断（摇杆按下）
static Z_BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

// 帧缓冲（静态分配）
static mut FRAMEBUF: [Rgb565; 240 * 240] = [Rgb565::BLACK; 240 * 240];

// 游戏板在屏幕上的偏移位置（居中显示）
const BOARD_OFFSET_X: i32 = 60;
const BOARD_OFFSET_Y: i32 = 0;

// 屏幕尺寸
const DISPLAY_WIDTH: usize = 240;
const DISPLAY_HEIGHT: usize = 240;

#[allow(clippy::large_stack_frames)]
#[main]
fn main() -> ! {
    esp_alloc::heap_allocator!(size: 150 * 1024);

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

    // 获取帧缓冲
    let framebuf: &mut [Rgb565] = unsafe {
        let ptr = core::ptr::addr_of_mut!(FRAMEBUF) as *mut [Rgb565; 240 * 240];
        &mut *ptr
    };

    // 方向判断参数
    const CENTER: u16 = 3900;       // 摇杆中点值
    const DEAD_ZONE: u16 = 100;     // 死区范围

    // 初始化游戏
    let mut game = TetrisGame::new();
    println!("俄罗斯方块游戏开始！");

    // 用于检测按钮按下（硬降）
    let mut hard_drop_triggered = false;
    
    // 用于方向防抖
    let mut last_move_time = 0u32;
    const MOVE_DELAY: u32 = 150; // 移动间隔（毫秒）

    loop {
        // 读取摇杆
        let x_value: u16 = nb::block!(adc1.read_oneshot(&mut x_adc)).unwrap();
        let y_value: u16 = nb::block!(adc1.read_oneshot(&mut y_adc)).unwrap();

        // 方向判断
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

        // 游戏结束状态 - 任意键重新开始
        if game.state == tetris::GameState::GameOver {
            // 绘制游戏结束画面
            draw_game_to_framebuf(framebuf, &game, true);
            let area = Rectangle::new(Point::new(0, 0), Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32));
            display.fill_contiguous(&area, framebuf.iter().copied()).unwrap();
            
            // 检测任意输入重新开始
            if x_dir != 0 || y_dir != 0 {
                println!("游戏重新开始！");
                game.reset();
                delay.delay_millis(300); // 防抖
            }
            
            // 检测Z按钮
            let z_pressed = critical_section::with(|cs| {
                let z_btn = Z_BUTTON.borrow_ref_mut(cs);
                if let Some(ref btn) = *z_btn {
                    btn.is_interrupt_set()
                } else {
                    false
                }
            });
            
            if z_pressed {
                critical_section::with(|cs| {
                    let mut z_btn = Z_BUTTON.borrow_ref_mut(cs);
                    if let Some(ref mut btn) = *z_btn {
                        btn.clear_interrupt();
                    }
                });
                println!("游戏重新开始！");
                game.reset();
                delay.delay_millis(300); // 防抖
            }
            
            delay.delay_millis(50);
            continue;
        }

        // 处理输入
        if game.state == tetris::GameState::Playing {
            last_move_time += 50; // 每次循环约50ms
            
            // 左右移动（带防抖）
            if last_move_time >= MOVE_DELAY {
                if x_dir == -1 {
                    game.try_move_left();
                    last_move_time = 0;
                } else if x_dir == 1 {
                    game.try_move_right();
                    last_move_time = 0;
                }
            }

            // 向下加速
            if y_dir == 1 {
                game.try_move_down();
            }

            // 旋转（上方向）
            if y_dir == -1 {
                game.try_rotate();
                // 等待释放，避免连续旋转
                delay.delay_millis(200);
            }
        }

        // 检测Z按钮按下（硬降）
        let z_pressed = critical_section::with(|cs| {
            let z_btn = Z_BUTTON.borrow_ref_mut(cs);
            if let Some(ref btn) = *z_btn {
                btn.is_interrupt_set()
            } else {
                false
            }
        });

        if z_pressed && !hard_drop_triggered && game.state == tetris::GameState::Playing {
            game.hard_drop();
            hard_drop_triggered = true;
            critical_section::with(|cs| {
                let mut z_btn = Z_BUTTON.borrow_ref_mut(cs);
                if let Some(ref mut btn) = *z_btn {
                    btn.clear_interrupt();
                }
            });
        } else if !z_pressed {
            hard_drop_triggered = false;
        }

        // 游戏更新
        game.update(50); // 约50ms的更新间隔

        // 游戏结束检测
        if game.state == tetris::GameState::GameOver {
            println!("游戏结束！最终得分: {} 消除行数: {}", game.score, game.lines);
        }

        // 绘制到帧缓冲
        draw_game_to_framebuf(framebuf, &game, false);
        
        // 将帧缓冲传输到显示
        let area = Rectangle::new(Point::new(0, 0), Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32));
        display.fill_contiguous(&area, framebuf.iter().copied()).unwrap();

        // 游戏速度控制
        delay.delay_millis(50);
    }
}

/// 绘制游戏画面到帧缓冲
fn draw_game_to_framebuf(framebuf: &mut [Rgb565], game: &TetrisGame, show_game_over: bool) {
    // 清空帧缓冲
    for pixel in framebuf.iter_mut() {
        *pixel = Rgb565::BLACK;
    }

    // 绘制游戏板边框
    let board_width_px = (BOARD_WIDTH as u16 * CELL_SIZE) as i32;
    let board_height_px = (BOARD_HEIGHT as u16 * CELL_SIZE) as i32;
    
    // 绘制边框
    for i in 0..=(board_width_px + 1) {
        let x = BOARD_OFFSET_X - 1 + i;
        if x >= 0 && x < DISPLAY_WIDTH as i32 {
            // 上边框
            let y_top = BOARD_OFFSET_Y - 1;
            if y_top >= 0 && y_top < DISPLAY_HEIGHT as i32 {
                framebuf[y_top as usize * DISPLAY_WIDTH + x as usize] = Rgb565::WHITE;
            }
            // 下边框
            let y_bottom = BOARD_OFFSET_Y + board_height_px;
            if y_bottom >= 0 && y_bottom < DISPLAY_HEIGHT as i32 {
                framebuf[y_bottom as usize * DISPLAY_WIDTH + x as usize] = Rgb565::WHITE;
            }
        }
    }
    
    for i in 0..=board_height_px {
        let y = BOARD_OFFSET_Y + i;
        if y >= 0 && y < DISPLAY_HEIGHT as i32 {
            // 左边框
            let x_left = BOARD_OFFSET_X - 1;
            if x_left >= 0 && x_left < DISPLAY_WIDTH as i32 {
                framebuf[y as usize * DISPLAY_WIDTH + x_left as usize] = Rgb565::WHITE;
            }
            // 右边框
            let x_right = BOARD_OFFSET_X + board_width_px;
            if x_right >= 0 && x_right < DISPLAY_WIDTH as i32 {
                framebuf[y as usize * DISPLAY_WIDTH + x_right as usize] = Rgb565::WHITE;
            }
        }
    }

    // 绘制已锁定的方块
    for y in 0..BOARD_HEIGHT {
        for x in 0..BOARD_WIDTH {
            if let Some(kind) = game.board[y][x] {
                draw_cell_to_framebuf(framebuf, x as i16, y as i16, kind);
            }
        }
    }

    // 绘制当前方块
    if let Some(ref piece) = game.current_piece {
        let shape = piece.shape();
        for y in 0..4 {
            for x in 0..4 {
                if shape[y][x] {
                    let board_x = piece.x + x as i16;
                    let board_y = piece.y + y as i16;
                    if board_y >= 0 {
                        draw_cell_to_framebuf(framebuf, board_x, board_y, piece.kind);
                    }
                }
            }
        }
    }

    // 绘制信息面板（左侧）
    let info_x = 5;
    let mut info_y = 10;
    
    // 绘制分数标题
    draw_text(framebuf, "SCORE", info_x, info_y, Rgb565::YELLOW);
    info_y += 12;
    
    // 绘制分数值
    let score_str = format_score(game.score);
    draw_text(framebuf, &score_str, info_x, info_y, Rgb565::WHITE);
    info_y += 20;
    
    // 绘制行数标题
    draw_text(framebuf, "LINES", info_x, info_y, Rgb565::YELLOW);
    info_y += 12;
    
    // 绘制行数值
    let lines_str = format_score(game.lines);
    draw_text(framebuf, &lines_str, info_x, info_y, Rgb565::WHITE);
    info_y += 20;
    
    // 绘制下一个方块预览标题
    draw_text(framebuf, "NEXT", info_x, info_y, Rgb565::YELLOW);
    info_y += 12;
    
    // 绘制下一个方块预览
    let preview_x = info_x;
    let preview_y = info_y;
    
    let next_kind = game.next_piece;
    let preview_shape = get_tetromino_shape(next_kind);
    for y in 0..2 {
        for x in 0..4 {
            if preview_shape[y][x] {
                let px = preview_x + (x as i16 * 8) as i32;
                let py = preview_y + (y as i16 * 8) as i32;
                let color = get_color(next_kind);
                fill_rect(framebuf, px, py, 7, 7, color);
            }
        }
    }

    // 游戏结束显示
    if show_game_over {
        // 绘制半透明背景
        let overlay_x = 30;
        let overlay_y = 90;
        let overlay_w = 180;
        let overlay_h = 60;
        
        for y in overlay_y..(overlay_y + overlay_h) {
            for x in overlay_x..(overlay_x + overlay_w) {
                if x >= 0 && x < DISPLAY_WIDTH as i32 && y >= 0 && y < DISPLAY_HEIGHT as i32 {
                    let idx = y as usize * DISPLAY_WIDTH + x as usize;
                    // 混合颜色（半透明效果）
                    let current = framebuf[idx];
                    framebuf[idx] = Rgb565::new(
                        (current.r() / 2),
                        (current.g() / 2),
                        (current.b() / 2),
                    );
                }
            }
        }
        
        // 绘制边框
        for x in overlay_x..(overlay_x + overlay_w) {
            if x >= 0 && x < DISPLAY_WIDTH as i32 {
                if overlay_y >= 0 && overlay_y < DISPLAY_HEIGHT as i32 {
                    framebuf[overlay_y as usize * DISPLAY_WIDTH + x as usize] = Rgb565::RED;
                }
                let bottom_y = overlay_y + overlay_h - 1;
                if bottom_y >= 0 && bottom_y < DISPLAY_HEIGHT as i32 {
                    framebuf[bottom_y as usize * DISPLAY_WIDTH + x as usize] = Rgb565::RED;
                }
            }
        }
        for y in overlay_y..(overlay_y + overlay_h) {
            if y >= 0 && y < DISPLAY_HEIGHT as i32 {
                if overlay_x >= 0 && overlay_x < DISPLAY_WIDTH as i32 {
                    framebuf[y as usize * DISPLAY_WIDTH + overlay_x as usize] = Rgb565::RED;
                }
                let right_x = overlay_x + overlay_w - 1;
                if right_x >= 0 && right_x < DISPLAY_WIDTH as i32 {
                    framebuf[y as usize * DISPLAY_WIDTH + right_x as usize] = Rgb565::RED;
                }
            }
        }
        
        // 绘制 GAME OVER 文字
        draw_text(framebuf, "GAME OVER", 70, 100, Rgb565::RED);
        draw_text(framebuf, "PRESS KEY", 70, 120, Rgb565::WHITE);
        draw_text(framebuf, "TO RESTART", 65, 132, Rgb565::WHITE);
    }
}

/// 绘制文本（使用帧缓冲）
fn draw_text(framebuf: &mut [Rgb565], text: &str, x: i32, y: i32, color: Rgb565) {
    // 使用 embedded-graphics 的文本渲染
    let style = MonoTextStyle::new(&FONT_6X10, color);
    let mut text_obj = Text::new(text, Point::new(x, y + 8), style);
    
    // 手动渲染到帧缓冲
    for (char_idx, c) in text.chars().enumerate() {
        let char_x = x + (char_idx as i32 * 6);
        if char_x < 0 || char_x >= DISPLAY_WIDTH as i32 - 5 {
            continue;
        }
        if y < 0 || y >= DISPLAY_HEIGHT as i32 - 9 {
            continue;
        }
        
        // 获取字符位图
        let char_code = c as usize;
        if char_code < 32 || char_code > 126 {
            continue;
        }
        
        // 简单的字符渲染（使用 FONT_6X10）
        // 这里我们直接使用 embedded-graphics 的渲染
        let char_str = alloc::string::String::from(c.encode_utf8(&mut [0; 4]));
        let char_text = Text::new(&char_str, Point::new(char_x, y + 8), MonoTextStyle::new(&FONT_6X10, color));
        
        // 渲染字符
        for py in 0..10 {
            for px in 0..6 {
                let pixel_x = char_x + px;
                let pixel_y = y + py;
                
                if pixel_x >= 0 && pixel_x < DISPLAY_WIDTH as i32 && pixel_y >= 0 && pixel_y < DISPLAY_HEIGHT as i32 {
                    // 简单的像素设置（实际字体渲染需要更复杂的逻辑）
                    // 这里我们使用一个简化的方法
                }
            }
        }
    }
    
    // 使用 embedded-graphics 的实际渲染
    let _ = text_obj.draw(&mut FramebufDrawTarget { framebuf, width: DISPLAY_WIDTH, height: DISPLAY_HEIGHT });
}

/// 格式化分数为字符串
fn format_score(score: u32) -> alloc::string::String {
    let mut result = alloc::string::String::new();
    let mut s = score;
    let mut digits = alloc::vec::Vec::new();
    
    if s == 0 {
        result.push('0');
    } else {
        while s > 0 {
            digits.push((s % 10) as u8 + b'0');
            s /= 10;
        }
        for d in digits.iter().rev() {
            result.push(*d as char);
        }
    }
    
    result
}

/// 帧缓冲绘制目标
struct FramebufDrawTarget<'a> {
    framebuf: &'a mut [Rgb565],
    width: usize,
    height: usize,
}

impl<'a> embedded_graphics::draw_target::DrawTarget for FramebufDrawTarget<'a> {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for pixel in pixels {
            let embedded_graphics::Pixel(point, color) = pixel;
            if point.x >= 0 && point.x < self.width as i32 && point.y >= 0 && point.y < self.height as i32 {
                let idx = point.y as usize * self.width + point.x as usize;
                self.framebuf[idx] = color;
            }
        }
        Ok(())
    }
}

impl<'a> embedded_graphics::geometry::OriginDimensions for FramebufDrawTarget<'a> {
    fn size(&self) -> Size {
        Size::new(self.width as u32, self.height as u32)
    }
}

/// 在帧缓冲中绘制单个格子
fn draw_cell_to_framebuf(framebuf: &mut [Rgb565], x: i16, y: i16, kind: TetrominoType) {
    let color = get_color(kind);
    let start_x = BOARD_OFFSET_X + (x as i16 * CELL_SIZE as i16) as i32;
    let start_y = BOARD_OFFSET_Y + (y as i16 * CELL_SIZE as i16) as i32;
    
    fill_rect(framebuf, start_x, start_y, CELL_SIZE - 1, CELL_SIZE - 1, color);
}

/// 在帧缓冲中填充矩形
fn fill_rect(framebuf: &mut [Rgb565], x: i32, y: i32, w: u16, h: u16, color: Rgb565) {
    for py in y..(y + h as i32) {
        if py < 0 || py >= DISPLAY_HEIGHT as i32 {
            continue;
        }
        for px in x..(x + w as i32) {
            if px < 0 || px >= DISPLAY_WIDTH as i32 {
                continue;
            }
            framebuf[py as usize * DISPLAY_WIDTH + px as usize] = color;
        }
    }
}

/// 获取方块颜色
fn get_color(kind: TetrominoType) -> Rgb565 {
    let (r, g, b) = kind.color();
    Rgb565::new(r, g, b)
}

/// 获取方块形状（初始状态，用于预览）
fn get_tetromino_shape(kind: TetrominoType) -> [[bool; 4]; 4] {
    match kind {
        TetrominoType::I => [
            [false, false, false, false],
            [true, true, true, true],
            [false, false, false, false],
            [false, false, false, false],
        ],
        TetrominoType::O => [
            [true, true, false, false],
            [true, true, false, false],
            [false, false, false, false],
            [false, false, false, false],
        ],
        TetrominoType::T => [
            [false, true, false, false],
            [true, true, true, false],
            [false, false, false, false],
            [false, false, false, false],
        ],
        TetrominoType::S => [
            [false, true, true, false],
            [true, true, false, false],
            [false, false, false, false],
            [false, false, false, false],
        ],
        TetrominoType::Z => [
            [true, true, false, false],
            [false, true, true, false],
            [false, false, false, false],
            [false, false, false, false],
        ],
        TetrominoType::J => [
            [true, false, false, false],
            [true, true, true, false],
            [false, false, false, false],
            [false, false, false, false],
        ],
        TetrominoType::L => [
            [false, false, true, false],
            [true, true, true, false],
            [false, false, false, false],
            [false, false, false, false],
        ],
    }
}

#[handler]
#[ram]
fn button_handler() {
    critical_section::with(|cs| {
        let mut z_btn = Z_BUTTON.borrow_ref_mut(cs);
        if let Some(ref mut btn) = *z_btn
            && btn.is_interrupt_set() {
                btn.clear_interrupt();
            }
    });
}
