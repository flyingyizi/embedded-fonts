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

        let final_scope:HashSet<_> = chars_range_set.difference(&left_chars_range_set).map(|c| c.clone()) .collect();
        let r_code = to_rust_code(&output, path, &charset_to_string(&final_scope));
        if r_code.is_none() {
            return None;
        }

        Some((r_code.unwrap(), charset_to_string(&left_chars_range_set)))
    }

    fn charset_to_string(set: &HashSet<char>) -> String {
        let mut string = String::new();
        for c in set {
            string.push(c.clone())
        }
        string
    }

    fn to_rust_code(
        mf: &MyBdfFont,
        related_bdf_path: &Path,
        code_scope: &String,
    ) -> Option<String> {
        if 0 == mf.glyphs.len() {
            return None;
        }

        let glyphs = format!("{:?}", mf.glyphs);
        let data = format!("{:?}", mf.data);
        let file_stem = related_bdf_path.file_stem().unwrap().to_owned();
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
//
// it only generate 3 parts: S_GLYPHS, s_data, and final {name}.
// You maybe reorganize according to your needs. For example, put the s_data into eeprom, 
// write your code reading them from eeprom, build a BdfFont instance with reference to {name}. 
//
pub use  unformatted::{name};
//pub use unformatted::{{LINE_HEIGHT, REPLACEMENT_CHARACTER, S_DATA_LEN, S_GLYPHS}};
#[rustfmt::skip]
mod unformatted {{
    use embedded_fonts::{{BdfGlyph,BdfFont}};
    use embedded_graphics::{{
        prelude::*,
        primitives::Rectangle,
    }};

    pub const S_GLYPHS:[BdfGlyph;{glyphs_count}] = {g};
    pub const S_DATA_LEN:usize = {data_cout};
    pub const REPLACEMENT_CHARACTER:usize = {replace};
    pub const LINE_HEIGHT:u32 = {height};

    /// maybe you want store it in special secion(e.g. .eeprom), you can use below attributes
    /// ```no_run
    /// #[no_mangle]
    /// #[link_section = ".eeprom"]
    /// ```
    static S_DATA: [u8;S_DATA_LEN] = {d};
    
    /// maybe you comment it, but use youself. e.g. store the data in eeprom, read data from eeprom and you contruct by youself.
    /// glyphs code include: "{list}"
    /// orig bdf file is {bdffile} 
    pub static  {name}: BdfFont = BdfFont{{
        glyphs: &S_GLYPHS,
        data : &S_DATA,
        line_height: LINE_HEIGHT,
        replacement_character:REPLACEMENT_CHARACTER,
    }};
}}    
"#,
            glyphs_count = mf.glyphs.len(),
            data_cout = mf.data.len(),
            list = code_scope,
            bdffile = related_bdf_path.to_str().unwrap(),
            name = constant,
            g = glyphs,
            d = data,
            height = mf.line_height,
            replace = mf.replacement_character,
        );

        Some(o)
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
