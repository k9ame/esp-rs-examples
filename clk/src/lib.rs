//! 时钟项目库
//! 
//! 包含7段数码管显示模块

#![no_std]

pub mod seven_segment;

pub use seven_segment::{Segments, SevenSegmentConfig, SevenSegmentDisplay};
