# ESP-Rust 示例项目

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.88-orange.svg)](https://www.rust-lang.org/)
[![ESP32-S3](https://img.shields.io/badge/Chip-ESP32--S3-blue.svg)](https://www.espressif.com/en/products/socs/esp32-s3)

[English](./README.md)

本仓库包含一系列运行在 ESP 微控制器上的 Rust 示例程序。这些示例展示了如何使用 [esp-hal](https://github.com/esp-rs/esp-hal) 生态系统为 ESP32 芯片开发嵌入式应用程序。

## 📋 包含的项目

| 项目 | 描述 | 特性 |
|------|------|------|
| **[clk](./clk)** | 数字时钟 | 七段数码管显示、实时时钟 |
| **[dht11-demo](./dht11-demo)** | 温湿度传感器 | DHT11传感器、LCD显示 |
| **[joystick](./joystick)** | 摇杆控制器 | ADC读取、模拟输入 |
| **[led](./led)** | LED控制 | GPIO操作、使用blinksy控制LED灯带 |
| **[power](./power)** | 电源管理 | 电源模式、睡眠功能 |
| **[showimg](./showimg)** | 图片显示 | 在LCD上渲染TGA/BMP图片 |
| **[slintui](./slintui)** | Slint UI演示 | GUI框架、触摸界面 |
| **[snake](./snake)** | 贪吃蛇游戏 | 经典游戏、摇杆控制 |
| **[sounds](./sounds)** | 音频播放 | PCM音频、扬声器输出 |
| **[tetris](./tetris)** | 俄罗斯方块 | 经典俄罗斯方块、embedded-graphics |
| **[tui](./tui)** | 终端界面 | 基于文本的UI、ratatui/mousefood |
| **[xiaoai-led](./xiaoai-led)** | 智能LED控制 | WiFi、MQTT、智能家居集成 |
| **[xiaoai-led-c3](./xiaoai-led-c3)** | 智能LED(C3版) | xiaoai-led的ESP32-C3变体 |

## 🛠️ 环境要求

- **Rust** 1.88 或更高版本（edition 2024）
- **ESP 开发环境**配置：
  - [espup](https://github.com/esp-rs/espup) 用于工具链安装
  - [probe-rs](https://probe.rs/) 用于烧录和调试
- **硬件**：ESP32-S3 开发板（部分示例支持 ESP32-C3）

## 🚀 快速开始

1. **安装 ESP Rust 工具链：**
   ```bash
   cargo install espup
   espup install
   ```

2. **克隆仓库：**
   ```bash
   git clone https://github.com/your-username/esp-rs-examples.git
   cd esp-rs-examples
   ```

3. **编译并烧录示例：**
   ```bash
   cd led  # 或其他示例目录
   cargo build --release
   cargo run --release
   ```

## 📦 主要依赖

- [esp-hal](https://github.com/esp-rs/esp-hal) - ESP芯片硬件抽象层
- [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics) - 2D图形库
- [mipidsi](https://github.com/almindor/mipidsi) - MIPI显示串行接口驱动
- [slint](https://slint-ui.com/) - 嵌入式系统GUI框架
- [blinksy](https://github.com/blinksy/blinksy) - LED灯带控制库
- [embassy](https://embassy.dev/) - 嵌入式异步框架

## 🔧 硬件支持

| 功能 | ESP32-S3 | ESP32-C3 |
|------|----------|----------|
| 基本GPIO | ✅ | ✅ |
| WiFi | ✅ | ✅ |
| LCD显示 | ✅ | ✅ |
| LED灯带 | ✅ | ✅ |
| 触摸 | ✅ | ❌ |

## 📖 文档

每个子目录都包含一个完整的 Rust 项目及其配置。请导航到感兴趣的示例查看更多详情。

## 🤝 贡献

欢迎贡献代码！请随时提交 Pull Request。

## 📄 许可证

本项目采用 MIT 许可证 - 详情请查看 [LICENSE](./LICENSE) 文件。

---

<p align="center">
  Made with ❤️ by <a href="https://github.com/kkch">KKCH</a>
</p>