#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_alloc as _;
use esp_hal::{self as hal, main};
use esp_hal::delay::Delay;
use esp_hal::i2s::master::{I2s, Channels, DataFormat, Config};
use esp_hal::dma_buffers;
use esp_hal::time::Rate;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

// 包含 PCM 音频数据 (从 MP3 转换而来)
// 使用命令转换: ffmpeg -i hello.mp3 -f s16le -acodec pcm_s16le -ar 16000 -ac 2 hello.pcm
static AUDIO_DATA: &[u8] = include_bytes!("../../hello.pcm");

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// 播放 PCM 音频数据
fn play_pcm(
    i2s_tx: &mut esp_hal::i2s::master::I2sTx<'static, esp_hal::Blocking>,
    _delay: &Delay,
) {
    // 分块播放 PCM 数据
    const CHUNK_SIZE: usize = 1024;
    let mut offset = 0;
    
    while offset < AUDIO_DATA.len() {
        let end = (offset + CHUNK_SIZE).min(AUDIO_DATA.len());
        let chunk = &AUDIO_DATA[offset..end];
        
        // 创建固定大小的缓冲区并复制数据
        let mut buffer = [0i16; CHUNK_SIZE / 2];
        let samples = chunk.len() / 2;
        
        // 将字节数据转换为 i16 样本
        for i in 0..samples {
            let byte_offset = i * 2;
            let sample = i16::from_le_bytes([chunk[byte_offset], chunk[byte_offset + 1]]);
            buffer[i] = sample;
        }
        
        // 写入 I2S
        let _ = i2s_tx.write_words(&buffer[..samples]);
        
        offset = end;
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    let cpu_clock = hal::clock::CpuClock::max();
    let config = hal::Config::default().with_cpu_clock(cpu_clock);
    let peripherals = hal::init(config);

    // 初始化堆内存分配器
    esp_alloc::heap_allocator!(size: 72 * 1024);

    // 配置 I2S
    // GPIO 配置:
    // LRC (Word Select) = GPIO 38
    // BCLK (Bit Clock) = GPIO 37  
    // DIN (Data In) = GPIO 36 - 注意：这是 ESP32 发送数据到功放的引脚
    
    // 创建 DMA 缓冲区 (TX only)
    let (_, _, _tx_buffer, tx_descriptors) = dma_buffers!(0, 4092);
    
    // 配置 I2S
    // ESP32-S3 使用 DMA_CH0
    // 使用 16kHz 采样率匹配音频文件
    let i2s = I2s::new(
        peripherals.I2S0,
        peripherals.DMA_CH0,
        Config::default()
            .with_sample_rate(Rate::from_hz(16000))
            .with_data_format(DataFormat::Data16Channel16)
            .with_channels(Channels::STEREO),
    ).unwrap();
    
    // 配置 I2S TX (发送)
    let mut i2s_tx = i2s
        .i2s_tx
        .with_bclk(peripherals.GPIO37)      // BCLK = GPIO 37
        .with_ws(peripherals.GPIO38)        // LRC/WS = GPIO 38
        .with_dout(peripherals.GPIO36)      // DOUT (数据输出到功放) = GPIO 36
        .build(tx_descriptors);
    
    let delay = Delay::new();

    // 播放一次 PCM 音频
    play_pcm(&mut i2s_tx, &delay);

    loop {
        // 空循环，保持程序运行
        delay.delay_millis(1000);
    }
}
