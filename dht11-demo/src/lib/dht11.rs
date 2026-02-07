#![no_std]

use embedded_dht_rs::dht11::Dht11;
use esp_hal::{
    delay::Delay,
    gpio::{DriveMode, Flex, OutputConfig, Pull},
};

/// DHT11 传感器管理器
pub struct Dht11Manager<'a> {
    dht11: Dht11<Flex<'a>, Delay>,
}

impl<'a> Dht11Manager<'a> {
    /// 创建新的 DHT11 传感器管理器
    pub fn new(pin: Flex<'static>, delay: Delay) -> Self {
        let mut dht11_pin = pin;
        let config = OutputConfig::default()
            .with_drive_mode(DriveMode::OpenDrain)
            .with_pull(Pull::None);
        dht11_pin.apply_output_config(&config);
        dht11_pin.set_output_enable(true);
        dht11_pin.set_input_enable(true);
        dht11_pin.set_high();

        let dht11 = Dht11::new(dht11_pin, delay);
        Self { dht11 }
    }

    /// 读取传感器数据
    pub fn read(&mut self) -> Result<(u8, u8), DhtError> {
        let reading = self.dht11.read().map_err(|_| DhtError)?;
        Ok((reading.temperature, reading.humidity))
    }
}

/// DHT11 错误
#[derive(Debug)]
pub struct DhtError;
