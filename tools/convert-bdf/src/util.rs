pub use conv::conv_bdf;

mod conv {
    use embedded_fonts::{BdfFont as MyBdfFont, BdfGlyph as MyBdfGlyph};
    use embedded_graphics::{prelude::*, primitives::Rectangle};
    use std::{collections::hash_set::HashSet, convert::TryFrom, fs, path::Path};

    /// convert bdf to const rust code
    pub fn conv_bdf(
        path: &Path,
        characters: Option<String>,
    ) -> Option<(String, String /*left chars*/)> {
        // TODO: handle errors
        let bdf = fs::read(&path).expect("couldn't open BDF file");
        let font = bdf_parser::BdfFont::parse(&bdf).expect("BDF file is bad format");

        let mut data = Vec::new();
        let mut glyphs = Vec::new();
        let mut replacement_character = None;

        let mut chars_range_set = HashSet::<char>::new();
        if let Some(cs) = &characters {
            for c in cs.chars() {
                chars_range_set.insert(c.clone());
            }
        }
        let mut left_chars_range_set = chars_range_set.clone();

        // ////////////////////
        //TODO: sort glyphs to make it possible to use binary search
        for glyph in font.glyphs.iter() {
            if let Some(c) = glyph.encoding {
                // if None, should output all fonts
                if chars_range_set.is_empty() == false {
                    if false == chars_range_set.contains(&c) {
                        continue;
                    }
                    left_chars_range_set.remove(&c);
                }

                if c == std::char::REPLACEMENT_CHARACTER
                    || (c == ' ' && replacement_character.is_none())
                {
                    replacement_character = Some(glyphs.len());
                }

                let (glyph_data, literal) = glyph_literal(glyph, data.len());
                glyphs.push(literal);
                data.extend_from_slice(&glyph_data);
            }
        }

        // TODO: try to use DEFAULT_CHAR
        let replacement_character = replacement_character.unwrap_or_default();

        let data = bits_to_bytes(&data);

        // TODO: report error or calculate fallback value
        let line_height = font
            .properties
            .try_get::<i32>(bdf_parser::Property::PixelSize)
            .unwrap() as u32;

        let output = MyBdfFont {
            glyphs: glyphs.as_slice(),
            data: data.as_slice(),
            line_height,
            replacement_character,
        };

        if 0 == output.glyphs.len() {
            return None;
        }

        let glyphs = format!("{:?}", output.glyphs);
        let data = format!("{:?}", output.data);
        let file_stem = path.file_stem().unwrap().to_owned();
        let constant = format!(
            "FONT_{}",
            file_stem
                .to_string_lossy()
                .to_ascii_uppercase()
                .replace("O", "_ITALIC")
                .replace("B", "_BOLD")
        );

        // format!("{:?}", output)
        let o = format!(
            r#"
// GENERATED CODE by convert-bdf in tools
            
pub use  unformatted::{name};
#[rustfmt::skip]
mod unformatted {{
    use embedded_fonts::{{BdfGlyph,BdfFont}};
    use embedded_graphics::{{
        prelude::*,
        primitives::Rectangle,
    }};
    
    /// include {glyphs_count} glyphs. characters: "{list}"
    /// orig bdf file is {bdffile} 
    pub const  {name}: BdfFont = BdfFont{{
        glyphs: &{g},
        data : &{d},
        line_height: {height},
        replacement_character:{replace},
    }};
}}    
"#,
            glyphs_count = output.glyphs.len(),
            list = charset_to_string(&chars_range_set),
            bdffile = path.to_str().unwrap(),
            name = constant,
            g = glyphs,
            d = data,
            height = output.line_height,
            replace = output.replacement_character,
        );

        Some((o, charset_to_string(&left_chars_range_set)))
    }

    fn charset_to_string(set: &HashSet<char>) -> String {
        let mut string = String::new();
        for c in set {
            string.push(c.clone())
        }
        string
    }

    fn bits_to_bytes(bits: &[bool]) -> Vec<u8> {
        bits.chunks(8)
            .map(|bits| {
                bits.iter()
                    .enumerate()
                    .filter(|(_, b)| **b)
                    .map(|(i, _)| 0x80 >> i)
                    .sum()
            })
            .collect()
    }

    fn glyph_literal(glyph: &bdf_parser::Glyph, start_index: usize) -> (Vec<bool>, MyBdfGlyph) {
        /// Converts a BDF bounding box into an embedded-graphics rectangle.
        fn bounding_box_to_rectangle(bounding_box: &bdf_parser::BoundingBox) -> Rectangle {
            Rectangle::new(
                Point::new(
                    bounding_box.offset.x,
                    -bounding_box.offset.y - (bounding_box.size.y as i32 - 1),
                ),
                // TODO: check for negative values
                Size::new(bounding_box.size.x as u32, bounding_box.size.y as u32),
            )
        }

        let character = glyph.encoding.unwrap();

        let rectangle = bounding_box_to_rectangle(&glyph.bounding_box);
        let bounding_box = rectangle.clone();

        // TODO: handle height != 0
        // TODO: check for negative values
        let device_width = glyph.device_width.x as u32;

        let mut data = Vec::new();

        for y in 0..usize::try_from(glyph.bounding_box.size.y).unwrap() {
            for x in 0..usize::try_from(glyph.bounding_box.size.x).unwrap() {
                data.push(glyph.pixel(x, y))
            }
        }

        (
            data,
            MyBdfGlyph {
                character,
                bounding_box,
                device_width,
                start_index,
            },
        )
    }
}
