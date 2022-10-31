pub use conv::conv_bdf;

mod conv {
    use embedded_fonts::{BdfFont as MyBdfFont, BdfGlyph as MyBdfGlyph, GlyphRect};
    use embedded_graphics::{prelude::*, primitives::Rectangle};
    use std::{collections::hash_set::HashSet, convert::TryFrom, fs, path::Path};

    /// convert bdf to const rust code
    pub fn conv_bdf(
        path: &Path,
        characters: Option<String>,
        to_serialize: bool,
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
        let mut final_scope = String::new();
        for i in &glyphs {
            final_scope.push(i.character.clone());
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


        let r_code = if to_serialize {
            to_rust_code_serialize(&output, path, &final_scope)
        } else {
            to_rust_code(&output, path, &final_scope)
        };
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
pub use  unformatted::{name};
#[rustfmt::skip]
mod unformatted {{
    use embedded_fonts::{{BdfGlyph,BdfFont,GlyphRect}};
    use embedded_graphics::{{
        prelude::*,
        primitives::Rectangle,
    }};

    const S_GLYPHS:[BdfGlyph;{glyphs_count}] = {g};

    const S_DATA_LEN:usize = {data_cout};
    const REPLACEMENT_CHARACTER:usize = {replace};
    const LINE_HEIGHT:u32 = {height};

    const S_DATA: [u8;S_DATA_LEN] = {d};
    
    /// glyphs code include: "{list}"
    /// orig bdf file is {bdffile} 
    /// #example
    /// ```no_run
    ///    let my_style = BdfTextStyle::new(&{name}, Rgb888::BLUE);
    ///    Text::new("display content", Point::new(5, 30), my_style).draw(&mut display)?;
    /// ```
    pub const  {name}: BdfFont = BdfFont{{
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
    fn to_rust_code_serialize(
        mf: &MyBdfFont,
        related_bdf_path: &Path,
        code_scope: &String,
    ) -> Option<String> {
        if 0 == mf.glyphs.len() {
            return None;
        }
        //
        let tt = bdfglyphs_to_cobs(mf.glyphs);
        let cobs = format!("{:?}", tt.as_slice());
        let cobs_len = tt.len();

        let glyphs = format!("{:?}", mf.glyphs);
        let data = format!("{:?}", mf.data);
        let file_stem = related_bdf_path.file_stem().unwrap().to_owned();

        // format!("{:?}", output)
        let o = format!(
            r#"
//! GENERATED CODE by convert-bdf in tools
//!
//! you should construct instance like below and use it.
//! # example
//! ```no_run
//! let mut glyphs_vec = Vec::<BdfGlyph>::new();
//! // let mut temp_vec = Vec::<u8>::new();
//! for i in 0..CHARACTERS_SCOPE_COUNT{{
//!     if let Some(x)= glyphs_cobs_decode(&cobs/*you read from eeprom,it content should be same as S_GLYPHS_COBS*/,i){{
//!         glyphs_vec.push(x);
//!     }} else {{
//!         assert_eq!(true, false);
//!     }}
//! }}
//! //read data from eeprom, it content should be same as S_DATA
//! let myfont: BdfFont = BdfFont{{
//!     glyphs: glyphs_vec.as_slice(),
//!     data : &data,
//!     line_height: LINE_HEIGHT,
//!     replacement_character:REPLACEMENT_CHARACTER,
//! }};
//! let my_style = BdfTextStyle::new(&myfont, Rgb888::BLUE);
//! Text::new("display content", Point::new(5, 30), my_style).draw(&mut display)?;
//! ```
pub use unformatted::{{
    CHARACTERS_SCOPE, CHARACTERS_SCOPE_COUNT, LINE_HEIGHT, REPLACEMENT_CHARACTER, S_DATA_LEN,
    S_GLYPHS_COBS_LEN,
}};
#[rustfmt::skip]
/// glyphs code include: "{list}"
/// orig bdf file is {bdffile} 
mod unformatted {{
    use embedded_fonts::{{BdfGlyph,BdfFont,GlyphRect}};
    use embedded_graphics::{{
        prelude::*,
        primitives::Rectangle,
    }};

    pub const S_DATA_LEN:usize = {data_cout};
    pub const REPLACEMENT_CHARACTER:usize = {replace};
    pub const LINE_HEIGHT:u32 = {height};
    pub const S_GLYPHS_COBS_LEN:usize = {glyphs_cobs_count};

    /// character's pos can used when you access or when decode S_GLYPHS_COBS
    pub const CHARACTERS_SCOPE:&str = "{list}";
    pub const CHARACTERS_SCOPE_COUNT:usize = {glyphs_count};

    // after decode, the content should be same as below:
    // const S_GLYPHS:[BdfGlyph;{glyphs_count}] = {g};
    #[no_mangle]
    #[link_section = ".eeprom"]
    static S_GLYPHS_COBS:[u8;S_GLYPHS_COBS_LEN]={glyphs_cobs};

    #[no_mangle]
    #[link_section = ".eeprom"]
    static S_DATA: [u8;S_DATA_LEN] = {d};
}}    
"#,
            glyphs_count = mf.glyphs.len(),
            data_cout = mf.data.len(),
            list = code_scope,
            bdffile = related_bdf_path.to_str().unwrap(),
            g = glyphs,
            d = data,
            height = mf.line_height,
            replace = mf.replacement_character,
            glyphs_cobs = cobs,
            glyphs_cobs_count = cobs_len,
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
        let bounding_box: GlyphRect = bounding_box.into();
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

    fn bdfglyphs_to_cobs(gs: &[MyBdfGlyph]) -> Vec<u8> {
        let mut glyphs_vec = Vec::<u8>::new();
        for i in gs {
            let mut buf = [0; 24];
            let cc = postcard::to_slice_cobs(i, &mut buf).unwrap();
            glyphs_vec.extend_from_slice(cc);
        }

        glyphs_vec
    }
}
