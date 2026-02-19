//! 7 segment digital clock display module
//! 
//! Using embedded_graphics to draw classic 7-segment digital clock
//! Segment style is hexagon (pointed ends, 90 degrees)

use embedded_graphics::{
    prelude::*,
    primitives::{Rectangle, PrimitiveStyle},
    pixelcolor::PixelColor,
    draw_target::DrawTarget,
};
use embedded_graphics_framebuf::FrameBuf;

/// 7 segment bit definitions
/// 
/// Segment layout:
/// ```text
///    AAAAA
///   F     B
///   F     B
///   F     B
///    GGGGG
///   E     C
///   E     C
///   E     C
///    DDDDD
/// ```
pub struct Segments(pub u8);

impl Segments {
    /// Segment A (top horizontal)
    pub const A: u8 = 0b01000000;
    /// Segment B (top right vertical)
    pub const B: u8 = 0b00100000;
    /// Segment C (bottom right vertical)
    pub const C: u8 = 0b00010000;
    /// Segment D (bottom horizontal)
    pub const D: u8 = 0b00001000;
    /// Segment E (bottom left vertical)
    pub const E: u8 = 0b00000100;
    /// Segment F (top left vertical)
    pub const F: u8 = 0b00000010;
    /// Segment G (middle horizontal)
    pub const G: u8 = 0b00000001;

    /// Create empty segments
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create segments from digit
    pub fn from_digit(digit: u8) -> Option<Self> {
        let bits = match digit {
            0 => Self::A | Self::B | Self::C | Self::D | Self::E | Self::F,
            1 => Self::B | Self::C,
            2 => Self::A | Self::B | Self::D | Self::E | Self::G,
            3 => Self::A | Self::B | Self::C | Self::D | Self::G,
            4 => Self::B | Self::C | Self::F | Self::G,
            5 => Self::A | Self::C | Self::D | Self::F | Self::G,
            6 => Self::A | Self::C | Self::D | Self::E | Self::F | Self::G,
            7 => Self::A | Self::B | Self::C,
            8 => Self::A | Self::B | Self::C | Self::D | Self::E | Self::F | Self::G,
            9 => Self::A | Self::B | Self::C | Self::D | Self::F | Self::G,
            _ => return None,
        };
        Some(Self(bits))
    }

    /// Check if contains segment
    pub fn contains(&self, segment: u8) -> bool {
        (self.0 & segment) != 0
    }
}

impl core::ops::BitOr for Segments {
    type Output = Segments;
    fn bitor(self, rhs: Segments) -> Self::Output {
        Segments(self.0 | rhs.0)
    }
}

impl core::ops::BitOr<u8> for Segments {
    type Output = Segments;
    fn bitor(self, rhs: u8) -> Self::Output {
        Segments(self.0 | rhs)
    }
}

/// 7 segment display configuration
#[derive(Debug, Clone, Copy)]
pub struct SevenSegmentConfig {
    /// Digit size
    pub digit_size: Size,
    /// Spacing between digits
    pub digit_spacing: u32,
    /// Segment width (thickness)
    pub segment_width: u32,
}

impl Default for SevenSegmentConfig {
    fn default() -> Self {
        Self {
            digit_size: Size::new(24, 48),
            digit_spacing: 8,
            segment_width: 4,
        }
    }
}

impl SevenSegmentConfig {
    /// Create new configuration
    pub fn new(digit_size: Size, digit_spacing: u32, segment_width: u32) -> Self {
        Self {
            digit_size,
            digit_spacing,
            segment_width,
        }
    }
}

/// 7 segment display drawer
pub struct SevenSegmentDisplay {
    config: SevenSegmentConfig,
}

impl SevenSegmentDisplay {
    /// Create new 7 segment display
    pub fn new(config: SevenSegmentConfig) -> Self {
        Self { config }
    }

    /// Draw horizontal segment (hexagon with pointed ends)
    /// 
    /// Draw by scanning each row, width decreases based on distance from center line
    fn draw_horizontal_segment<C, const N: usize>(
        &self,
        fbuf: &mut FrameBuf<C, &mut [C; N]>,
        rect: Rectangle,
        color: C,
    ) where
        C: PixelColor + Default,
    {
        if rect.is_zero_sized() {
            return;
        }

        let center_2x = rect.top_left * 2 + (rect.size - Size::new(1, 1));

        for y in rect.rows() {
            let offset = (y * 2 - center_2x.y).abs() / 2;

            let scanline = Rectangle::new(
                Point::new(rect.top_left.x + offset, y),
                Size::new(rect.size.width - offset as u32 * 2, 1),
            );

            let _ = scanline.into_styled(PrimitiveStyle::with_fill(color)).draw(fbuf);
        }
    }

    /// Draw vertical segment (hexagon with pointed ends)
    /// 
    /// Draw by scanning each column, height decreases based on distance from center line
    fn draw_vertical_segment<C, const N: usize>(
        &self,
        fbuf: &mut FrameBuf<C, &mut [C; N]>,
        rect: Rectangle,
        color: C,
    ) where
        C: PixelColor + Default,
    {
        if rect.is_zero_sized() {
            return;
        }

        let center_2x = rect.top_left * 2 + (rect.size - Size::new(1, 1));

        for x in rect.columns() {
            let offset = (x * 2 - center_2x.x).abs() / 2;

            let scanline = Rectangle::new(
                Point::new(x, rect.top_left.y + offset),
                Size::new(1, rect.size.height - offset as u32 * 2),
            );

            let _ = scanline.into_styled(PrimitiveStyle::with_fill(color)).draw(fbuf);
        }
    }

    /// Draw segment (auto select horizontal or vertical based on rectangle orientation)
    fn draw_segment<C, const N: usize>(
        &self,
        fbuf: &mut FrameBuf<C, &mut [C; N]>,
        rect: Rectangle,
        color: C,
    ) where
        C: PixelColor + Default,
    {
        if rect.size.width > rect.size.height {
            self.draw_horizontal_segment(fbuf, rect, color);
        } else {
            self.draw_vertical_segment(fbuf, rect, color);
        }
    }

    /// Create reduced size segment rectangle (avoid overlap between horizontal and vertical segments)
    fn reduced_rect(&self, mut rect: Rectangle) -> Rectangle {
        if rect.is_zero_sized() {
            return rect;
        }

        if rect.size.width > rect.size.height {
            // Horizontal segment: reduce width
            let size_offset = rect.size.height / 2 + 1;
            rect.top_left.x += size_offset as i32;
            rect.size.width = rect.size.width.saturating_sub(2 * size_offset);
        } else {
            // Vertical segment: reduce height
            let size_offset = rect.size.width / 2 + 1;
            rect.top_left.y += size_offset as i32;
            rect.size.height = rect.size.height.saturating_sub(2 * size_offset);
        }

        rect
    }

    /// Draw single digit to frame buffer
    pub fn draw_digit_to_fbuf<C, const N: usize>(
        &self,
        fbuf: &mut FrameBuf<C, &mut [C; N]>,
        digit: u8,
        position: Point,
        color: C,
        inactive_color: Option<C>,
    ) where
        C: PixelColor + Default,
    {
        let segments = Segments::from_digit(digit).unwrap_or(Segments::empty());
        self.draw_segments_to_fbuf(fbuf, &segments, position, color, inactive_color);
    }

    /// Draw segments to frame buffer
    /// 
    /// Key: horizontal and vertical segments share the same starting coordinates
    /// Overlap is avoided by reducing size during drawing
    /// 
    /// inactive_color: color for inactive segments (dim effect), None to hide completely
    pub fn draw_segments_to_fbuf<C, const N: usize>(
        &self,
        fbuf: &mut FrameBuf<C, &mut [C; N]>,
        segments: &Segments,
        position: Point,
        color: C,
        inactive_color: Option<C>,
    ) where
        C: PixelColor + Default,
    {
        let cfg = &self.config;
        let sw = cfg.segment_width;
        let w = cfg.digit_size.width;
        let h = cfg.digit_size.height;

        // Segment rectangles (full size, will be reduced during drawing)
        // Horizontal segment width = digit width
        // Vertical segment height = digit height / 2

        // Segment A (top horizontal)
        let rect_a = Rectangle::new(position, Size::new(w, sw));
        if segments.contains(Segments::A) {
            self.draw_segment(fbuf, self.reduced_rect(rect_a), color);
        } else if let Some(inactive) = inactive_color {
            self.draw_segment(fbuf, self.reduced_rect(rect_a), inactive);
        }

        // Segment F (top left vertical)
        let rect_f = Rectangle::new(position, Size::new(sw, h / 2));
        if segments.contains(Segments::F) {
            self.draw_segment(fbuf, self.reduced_rect(rect_f), color);
        } else if let Some(inactive) = inactive_color {
            self.draw_segment(fbuf, self.reduced_rect(rect_f), inactive);
        }

        // Segment B (top right vertical)
        let rect_b = Rectangle::new(position + Size::new(w - sw, 0), Size::new(sw, h / 2));
        if segments.contains(Segments::B) {
            self.draw_segment(fbuf, self.reduced_rect(rect_b), color);
        } else if let Some(inactive) = inactive_color {
            self.draw_segment(fbuf, self.reduced_rect(rect_b), inactive);
        }

        // Segment G (middle horizontal)
        let rect_g = Rectangle::new(position + Size::new(0, h / 2 - sw / 2), Size::new(w, sw));
        if segments.contains(Segments::G) {
            self.draw_segment(fbuf, self.reduced_rect(rect_g), color);
        } else if let Some(inactive) = inactive_color {
            self.draw_segment(fbuf, self.reduced_rect(rect_g), inactive);
        }

        // Segment E (bottom left vertical)
        let rect_e = Rectangle::new(position + Size::new(0, h / 2), Size::new(sw, h / 2));
        if segments.contains(Segments::E) {
            self.draw_segment(fbuf, self.reduced_rect(rect_e), color);
        } else if let Some(inactive) = inactive_color {
            self.draw_segment(fbuf, self.reduced_rect(rect_e), inactive);
        }

        // Segment C (bottom right vertical)
        let rect_c = Rectangle::new(position + Size::new(w - sw, h / 2), Size::new(sw, h / 2));
        if segments.contains(Segments::C) {
            self.draw_segment(fbuf, self.reduced_rect(rect_c), color);
        } else if let Some(inactive) = inactive_color {
            self.draw_segment(fbuf, self.reduced_rect(rect_c), inactive);
        }

        // Segment D (bottom horizontal)
        let rect_d = Rectangle::new(position + Size::new(0, h - sw), Size::new(w, sw));
        if segments.contains(Segments::D) {
            self.draw_segment(fbuf, self.reduced_rect(rect_d), color);
        } else if let Some(inactive) = inactive_color {
            self.draw_segment(fbuf, self.reduced_rect(rect_d), inactive);
        }
    }

    /// Draw colon to frame buffer
    pub fn draw_colon_to_fbuf<C, const N: usize>(
        &self,
        fbuf: &mut FrameBuf<C, &mut [C; N]>,
        position: Point,
        color: C,
    ) where
        C: PixelColor + Default,
    {
        let cfg = &self.config;
        let sw = cfg.segment_width;
        let h = cfg.digit_size.height;
        let dy = h / 3;

        // Top dot
        let rect1 = Rectangle::new(
            position + Size::new(0, dy - sw / 2),
            Size::new(sw, sw),
        );
        let _ = rect1.into_styled(PrimitiveStyle::with_fill(color)).draw(fbuf);

        // Bottom dot
        let rect2 = Rectangle::new(
            position + Size::new(0, dy * 2 - sw / 2),
            Size::new(sw, sw),
        );
        let _ = rect2.into_styled(PrimitiveStyle::with_fill(color)).draw(fbuf);
    }

    /// Draw time to frame buffer and display (format: HH:MM:SS)
    /// 
    /// inactive_color: color for inactive segments (dim effect), None to hide completely
    pub fn draw_time<T, C, const N: usize>(
        &self,
        display: &mut T,
        fbuf: &mut FrameBuf<C, &mut [C; N]>,
        hours: u8,
        minutes: u8,
        seconds: u8,
        color: C,
        inactive_color: Option<C>,
    ) -> Result<(), T::Error>
    where
        T: DrawTarget<Color = C>,
        C: PixelColor + Default,
    {
        let cfg = &self.config;
        let digit_width = cfg.digit_size.width;
        let digit_spacing = cfg.digit_spacing;
        let segment_width = cfg.segment_width;
        
        // Clear frame buffer
        for pixel in fbuf.data.iter_mut() {
            *pixel = C::default();
        }

        // Calculate starting position (centered)
        let fbuf_size = fbuf.size();
        let total_width = Self::time_display_width(cfg);
        let start_x = (fbuf_size.width.saturating_sub(total_width)) as i32 / 2;
        let start_y = (fbuf_size.height.saturating_sub(cfg.digit_size.height)) as i32 / 2;

        let mut position = Point::new(start_x, start_y);

        // Draw hours tens
        self.draw_digit_to_fbuf(fbuf, hours / 10, position, color, inactive_color);
        position.x += digit_width as i32 + digit_spacing as i32;

        // Draw hours units
        self.draw_digit_to_fbuf(fbuf, hours % 10, position, color, inactive_color);
        position.x += digit_width as i32 + digit_spacing as i32;

        // Draw first colon
        self.draw_colon_to_fbuf(fbuf, position, color);
        position.x += segment_width as i32 + digit_spacing as i32;

        // Draw minutes tens
        self.draw_digit_to_fbuf(fbuf, minutes / 10, position, color, inactive_color);
        position.x += digit_width as i32 + digit_spacing as i32;

        // Draw minutes units
        self.draw_digit_to_fbuf(fbuf, minutes % 10, position, color, inactive_color);
        position.x += digit_width as i32 + digit_spacing as i32;

        // Draw second colon
        self.draw_colon_to_fbuf(fbuf, position, color);
        position.x += segment_width as i32 + digit_spacing as i32;

        // Draw seconds tens
        self.draw_digit_to_fbuf(fbuf, seconds / 10, position, color, inactive_color);
        position.x += digit_width as i32 + digit_spacing as i32;

        // Draw seconds units
        self.draw_digit_to_fbuf(fbuf, seconds % 10, position, color, inactive_color);

        // Calculate display position (centered)
        let display_center = display.bounding_box().center();
        let target_point = Point::new(
            display_center.x - (fbuf_size.width as i32 / 2),
            display_center.y - (fbuf_size.height as i32 / 2),
        );
        
        // Use fill_contiguous to write to display in one operation, avoid flicker
        let area = Rectangle::new(target_point, fbuf_size);
        display.fill_contiguous(&area, fbuf.data.iter().copied())?;
        
        Ok(())
    }

    /// Calculate total width of time display
    pub fn time_display_width(config: &SevenSegmentConfig) -> u32 {
        let digit_width = config.digit_size.width;
        let digit_spacing = config.digit_spacing;
        let segment_width = config.segment_width;
        
        // 6 digits + 2 colons + spacing
        digit_width * 6 + segment_width * 2 + digit_spacing * 7
    }
}
