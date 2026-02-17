#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use alloc::{boxed::Box, rc::Rc};
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::prelude::{Point, Size};
use embedded_graphics::primitives::Rectangle;
use embedded_graphics_core::pixelcolor::raw::RawU16;
use embedded_graphics_core::pixelcolor::Rgb565;
use embedded_graphics_core::pixelcolor::RgbColor;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_alloc as _;
use esp_hal::main;
use esp_hal::{clock::CpuClock, delay::Delay, gpio, spi::master::Config, time::Rate};
use mipidsi::{Builder, interface::SpiInterface, models::ST7789};
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::WindowAdapter;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// ESP-IDF 应用描述符
esp_bootloader_esp_idf::esp_app_desc!();
slint::include_modules!();

// Slint 平台后端
struct EspBackend {
    window: Rc<MinimalSoftwareWindow>,
}

impl EspBackend {
    fn new(window: Rc<MinimalSoftwareWindow>) -> Self {
        Self { window }
    }
}

impl slint::platform::Platform for EspBackend {
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> core::time::Duration {
        core::time::Duration::from_millis(0)
    }

    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        rtt_target::rprintln!("Starting event loop...");
        
        // 初始化硬件
        let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
        let peripherals = esp_hal::init(config);

        // LCD 显示初始化
        let dc = gpio::Output::new(
            peripherals.GPIO15,
            gpio::Level::Low,
            Default::default(),
        );
        let mut rst = gpio::Output::new(
            peripherals.GPIO7,
            gpio::Level::Low,
            Default::default(),
        );
        rst.set_high();
        let cs = gpio::Output::new(
            peripherals.GPIO16,
            gpio::Level::High,
            Default::default(),
        );
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
            .color_order(mipidsi::options::ColorOrder::Rgb)
            .invert_colors(mipidsi::options::ColorInversion::Inverted)
            .init(&mut Delay::new())
            .unwrap();

        // 清空屏幕
        display.clear(Rgb565::BLACK).unwrap();

        // 设置窗口大小
        self.window.set_size(slint::PhysicalSize::new(240, 240));

        // 创建 DrawBuffer
        let mut line_buffer = [slint::platform::software_renderer::Rgb565Pixel::default(); 240];
        let mut buffer_provider = DrawBuffer {
            display: &mut display,
            buffer: &mut line_buffer,
        };

        // 事件循环
        loop {
            slint::platform::update_timers_and_animations();

            self.window.draw_if_needed(|renderer| {
                rtt_target::rprintln!("Rendering...");
                renderer.render_by_line(&mut buffer_provider);
            });

            if !self.window.has_active_animations() {
                if let Some(_duration) = slint::platform::duration_until_next_timer_update() {
                    // 可以在这里设置定时器中断
                }
            }
        }
    }

    fn debug_log(&self, arguments: core::fmt::Arguments) {
        rtt_target::rprintln!("{}", arguments);
    }
}

// 创建 Display 类型别名
type DisplayType = mipidsi::Display<
    SpiInterface<'static, ExclusiveDevice<esp_hal::spi::master::Spi<'static, esp_hal::Blocking>, gpio::Output<'static>, embedded_hal_bus::spi::NoDelay>, gpio::Output<'static>>,
    mipidsi::models::ST7789,
    gpio::Output<'static>,
>;

// DrawBuffer 用于将 Slint 渲染结果传输到 LCD
struct DrawBuffer<'a> {
    display: &'a mut DisplayType,
    buffer: &'a mut [slint::platform::software_renderer::Rgb565Pixel],
}

impl slint::platform::software_renderer::LineBufferProvider for &mut DrawBuffer<'_> {
    type TargetPixel = slint::platform::software_renderer::Rgb565Pixel;

    fn process_line(
        &mut self,
        line: usize,
        range: core::ops::Range<usize>,
        render_fn: impl FnOnce(&mut [slint::platform::software_renderer::Rgb565Pixel]),
    ) {
        let buffer = &mut self.buffer[range.clone()];

        render_fn(buffer);

        let area = Rectangle::new(
            Point::new(range.start as i32, line as i32),
            Size::new(range.len() as u32, 1),
        );
        let _ = self.display.fill_contiguous(
            &area,
            buffer.iter().map(|x| RawU16::new(x.0).into()),
        );
    }
}

#[allow(clippy::large_stack_frames, reason = "it's not unusual to allocate larger buffers etc. in main")]
#[main]
fn main() -> ! {
    // 初始化堆分配器
    esp_alloc::heap_allocator!(size: 64 * 1024);
    
    rtt_target::rprintln!("Starting main...");
    // 先创建窗口
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);

    // 设置 Slint 平台后端
    slint::platform::set_platform(Box::new(EspBackend::new(window.clone())))
        .expect("backend already initialized");
    rtt_target::rprintln!("Platform set, creating UI...");

    // 创建 UI 组件并运行
    let main_window = Recipe::new().unwrap();
    let strong = main_window.clone_strong();

    // 定时器模拟按钮点击
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        core::time::Duration::from_millis(1000),
        move || {
            if strong.get_counter() <= 0 {
                strong.set_counter(25);
            } else {
                strong.set_counter(0);
            }
        },
    );

    // 这会调用平台的 run_event_loop 方法
    rtt_target::rprintln!("Calling run()...");
    main_window.run().unwrap();

    loop {}
}
