pub const REGULAR_FONT: ::embedded_graphics::mono_font::MonoFont = ::embedded_graphics::mono_font::MonoFont {
    image: ::embedded_graphics::image::ImageRaw::new(
        include_bytes!("regular_font.data"),
        368u32,
    ),
    glyph_mapping: &::embedded_graphics::mono_font::mapping::StrGlyphMapping::new(
        " %,\x000:C°度温湿",
        0usize,
    ),
    character_size: ::embedded_graphics::geometry::Size::new(23u32, 18u32),
    character_spacing: 1u32,
    baseline: 11u32,
    underline: ::embedded_graphics::mono_font::DecorationDimensions::new(13u32, 1u32),
    strikethrough: ::embedded_graphics::mono_font::DecorationDimensions::new(7u32, 1u32),
};
