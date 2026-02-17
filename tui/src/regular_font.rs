pub const REGULAR_FONT: crate::BdfFont = {
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
    crate::BdfFont {
        data: include_bytes!("regular_font.data"),
        replacement_character: 0usize,
        ascent: 12u32,
        descent: 3u32,
        glyphs: &[
            crate::BdfGlyph {
                character: '!',
                bounding_box: rect(1i32, -9i32, 1u32, 10u32),
                device_width: 3u32,
                start_index: 0usize,
            },
            crate::BdfGlyph {
                character: ',',
                bounding_box: rect(0i32, 0i32, 2u32, 4u32),
                device_width: 3u32,
                start_index: 10usize,
            },
            crate::BdfGlyph {
                character: '交',
                bounding_box: rect(1i32, -9i32, 11u32, 12u32),
                device_width: 13u32,
                start_index: 18usize,
            },
            crate::BdfGlyph {
                character: '内',
                bounding_box: rect(2i32, -9i32, 9u32, 11u32),
                device_width: 13u32,
                start_index: 150usize,
            },
            crate::BdfGlyph {
                character: '告',
                bounding_box: rect(1i32, -9i32, 11u32, 11u32),
                device_width: 13u32,
                start_index: 249usize,
            },
            crate::BdfGlyph {
                character: '易',
                bounding_box: rect(1i32, -8i32, 10u32, 10u32),
                device_width: 13u32,
                start_index: 370usize,
            },
            crate::BdfGlyph {
                character: '有',
                bounding_box: rect(1i32, -9i32, 11u32, 11u32),
                device_width: 13u32,
                start_index: 470usize,
            },
            crate::BdfGlyph {
                character: '止',
                bounding_box: rect(1i32, -9i32, 11u32, 10u32),
                device_width: 13u32,
                start_index: 591usize,
            },
            crate::BdfGlyph {
                character: '终',
                bounding_box: rect(1i32, -9i32, 11u32, 11u32),
                device_width: 13u32,
                start_index: 701usize,
            },
            crate::BdfGlyph {
                character: '警',
                bounding_box: rect(1i32, -9i32, 11u32, 11u32),
                device_width: 13u32,
                start_index: 822usize,
            },
            crate::BdfGlyph {
                character: '鬼',
                bounding_box: rect(1i32, -9i32, 11u32, 11u32),
                device_width: 13u32,
                start_index: 943usize,
            },
        ],
    }
};
