# ESP-Rust Examples

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.88-orange.svg)](https://www.rust-lang.org/)
[![ESP32-S3](https://img.shields.io/badge/Chip-ESP32--S3-blue.svg)](https://www.espressif.com/en/products/socs/esp32-s3)

[中文文档](./README_CN.md)

This repository contains a collection of Rust examples designed to run on ESP microcontrollers. These examples demonstrate how to use the [esp-hal](https://github.com/esp-rs/esp-hal) ecosystem to develop embedded applications for ESP32 chips.

## 📋 Projects Included

| Project | Description | Features |
|---------|-------------|----------|
| **[clk](./clk)** | Digital Clock | 7-segment display, real-time clock |
| **[dht11-demo](./dht11-demo)** | Temperature & Humidity Sensor | DHT11 sensor, LCD display |
| **[joystick](./joystick)** | Joystick Controller | ADC reading, analog input |
| **[led](./led)** | LED Control | GPIO operations, LED strip control with blinksy |
| **[power](./power)** | Power Management | Power modes, sleep functions |
| **[showimg](./showimg)** | Image Display | TGA/BMP image rendering on LCD |
| **[slintui](./slintui)** | Slint UI Demo | GUI framework, touch interface |
| **[snake](./snake)** | Snake Game | Classic game with joystick control |
| **[sounds](./sounds)** | Audio Playback | PCM audio, speaker output |
| **[tetris](./tetris)** | Tetris Game | Classic Tetris with embedded-graphics |
| **[tui](./tui)** | Terminal UI | Text-based UI with ratatui/mousefood |
| **[xiaoai-led](./xiaoai-led)** | Smart LED Control | WiFi, MQTT, smart home integration |
| **[xiaoai-led-c3](./xiaoai-led-c3)** | Smart LED (C3) | ESP32-C3 variant of xiaoai-led |

## 🛠️ Prerequisites

- **Rust** 1.88 or later (edition 2024)
- **ESP development environment** setup:
  - [espup](https://github.com/esp-rs/espup) for toolchain installation
  - [probe-rs](https://probe.rs/) for flashing and debugging
- **Hardware**: ESP32-S3 development board (some examples support ESP32-C3)

## 🚀 Getting Started

1. **Install ESP Rust toolchain:**
   ```bash
   cargo install espup
   espup install
   ```

2. **Clone the repository:**
   ```bash
   git clone https://github.com/your-username/esp-rs-examples.git
   cd esp-rs-examples
   ```

3. **Build and flash an example:**
   ```bash
   cd led  # or any other example
   cargo build --release
   cargo run --release
   ```

## 📦 Key Dependencies

- [esp-hal](https://github.com/esp-rs/esp-hal) - Hardware Abstraction Layer for ESP chips
- [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics) - 2D graphics library
- [mipidsi](https://github.com/almindor/mipidsi) - MIPI Display Serial Interface driver
- [slint](https://slint-ui.com/) - GUI framework for embedded systems
- [blinksy](https://github.com/blinksy/blinksy) - LED strip control library
- [embassy](https://embassy.dev/) - Async framework for embedded

## 🔧 Hardware Support

| Feature | ESP32-S3 | ESP32-C3 |
|---------|----------|----------|
| Basic GPIO | ✅ | ✅ |
| WiFi | ✅ | ✅ |
| LCD Display | ✅ | ✅ |
| LED Strip | ✅ | ✅ |
| Touch | ✅ | ❌ |

## 📖 Documentation

Each subdirectory contains a complete Rust project with its own configuration. Navigate to the desired example for more details.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.

---

<p align="center">
  Made with ❤️ by <a href="https://github.com/kkch">KKCH</a>
</p>