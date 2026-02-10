use crate::{BdfFont, BdfGlyph};


pub const REGULAR_FONT: BdfFont = {
    const fn rect(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> ::embedded_graphics::primitives::Rectangle {
        ::embedded_graphics::primitives::Rectangle::new(
            ::embedded_graphics::geometry::Point::new(x, y),
            ::embedded_graphics::geometry::Size::new(width, height),
        )
    }
    BdfFont {
        data: include_bytes!("regular_font.data"),
        replacement_character: 0usize,
        ascent: 12u32,
        descent: 3u32,
        glyphs: &[
            BdfGlyph {
                character: '%',
                bounding_box: rect(1i32, -9i32, 10u32, 10u32),
                device_width: 11u32,
                start_index: 0usize,
            },
            BdfGlyph {
                character: '0',
                bounding_box: rect(1i32, -9i32, 6u32, 10u32),
                device_width: 8u32,
                start_index: 100usize,
            },
            BdfGlyph {
                character: '1',
                bounding_box: rect(1i32, -9i32, 3u32, 10u32),
                device_width: 6u32,
                start_index: 160usize,
            },
            BdfGlyph {
                character: '2',
                bounding_box: rect(1i32, -9i32, 6u32, 10u32),
                device_width: 8u32,
                start_index: 190usize,
            },
            BdfGlyph {
                character: '3',
                bounding_box: rect(0i32, -9i32, 6u32, 10u32),
                device_width: 7u32,
                start_index: 250usize,
            },
            BdfGlyph {
                character: '4',
                bounding_box: rect(1i32, -9i32, 7u32, 10u32),
                device_width: 9u32,
                start_index: 310usize,
            },
            BdfGlyph {
                character: '5',
                bounding_box: rect(1i32, -9i32, 6u32, 10u32),
                device_width: 8u32,
                start_index: 380usize,
            },
            BdfGlyph {
                character: '6',
                bounding_box: rect(1i32, -9i32, 6u32, 10u32),
                device_width: 8u32,
                start_index: 440usize,
            },
            BdfGlyph {
                character: '7',
                bounding_box: rect(1i32, -9i32, 6u32, 10u32),
                device_width: 8u32,
                start_index: 500usize,
            },
            BdfGlyph {
                character: '8',
                bounding_box: rect(1i32, -9i32, 6u32, 10u32),
                device_width: 8u32,
                start_index: 560usize,
            },
            BdfGlyph {
                character: '9',
                bounding_box: rect(1i32, -9i32, 6u32, 10u32),
                device_width: 8u32,
                start_index: 620usize,
            },
            BdfGlyph {
                character: ':',
                bounding_box: rect(1i32, -6i32, 1u32, 7u32),
                device_width: 3u32,
                start_index: 680usize,
            },
            BdfGlyph {
                character: 'C',
                bounding_box: rect(1i32, -9i32, 7u32, 10u32),
                device_width: 8u32,
                start_index: 687usize,
            },
            BdfGlyph {
                character: '°',
                bounding_box: rect(1i32, -9i32, 4u32, 4u32),
                device_width: 5u32,
                start_index: 757usize,
            },
            BdfGlyph {
                character: '度',
                bounding_box: rect(1i32, -9i32, 11u32, 12u32),
                device_width: 13u32,
                start_index: 773usize,
            },
            BdfGlyph {
                character: '温',
                bounding_box: rect(1i32, -8i32, 11u32, 10u32),
                device_width: 13u32,
                start_index: 905usize,
            },
            BdfGlyph {
                character: '湿',
                bounding_box: rect(1i32, -8i32, 11u32, 10u32),
                device_width: 13u32,
                start_index: 1015usize,
            },
        ],
    }
};
