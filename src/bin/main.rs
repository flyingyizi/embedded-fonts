//! usage:
//! convert-bdf  .\testdata\wenquanyi_12pt.bdf --range "china"
//!
pub use conv::conv_bdf;

use clap::Parser;
use std::{ffi::OsStr, fs, path::PathBuf};

#[derive(clap::Parser)]
struct Args {
    #[clap(help = "BDF input")]
    bdf_file: PathBuf,

    #[clap(
        long,
        help = r#"export characters list,defaultly export all glyphs in the bdf. e.g --range "abc" means only export a,b and c code's glyphs. 
if exist range and range-file options at the same time. merge them as final exporting glyphs scope"#
    )]
    range: Option<String>,
    #[clap(long, help = "same as range option, but through file.")]
    range_file: Option<PathBuf>,

    #[clap(
        short,
        long,
        help = "output path. if not exist \".rs\" extention, will look it as dirctory, and use the bdf file's stem as its stem",
        default_value = "./"
    )]
    output: PathBuf,
}

fn main() {
    let args: Args = Args::parse();
    if args.bdf_file.is_file() == false {
        println!("bdf file not exist");
        return;
    }

    let mut range_input: Option<String> = None;
    let mut s = String::new();

    if let Some(p) = args.range_file {
        if p.is_file() {
            let mut range_from_file = String::new();
            for c in fs::read_to_string(p).expect("couldn't open BDF file").chars() {
                if c == '\r' || c == '\n'  {
                }else {
                    range_from_file.push(c);
                }
            }
            s = s + &range_from_file;
        } else {
            println!("input range file is not exist, ignore it:{:?}", p);
        }
    }
    if let Some(ref s1) = args.range {
        s += s1;
    }
    if s.len() > 0 {
        let _ = range_input.replace(s);
    }

    if let Some((contents, left)) = conv_bdf(args.bdf_file.as_path(), range_input) {
        let mut ot: PathBuf = args.output;

        let rust_ext = OsStr::new("rs");
        if Some(rust_ext) == ot.extension() {
            // make sure directory exist. if not exist, create it
            if let Some(parent) = ot.parent() {
                if parent.is_dir() == false {
                    std::fs::create_dir_all(parent).expect("could'nt create not exist dir");
                }
            }
        } else {
            if ot.is_dir() == false {
                std::fs::create_dir_all(ot.as_path()).expect("could'nt create not exist dir");
            }
            ot = ot
                .join(args.bdf_file.file_stem().unwrap())
                .with_extension("rs");
        }

        fs::write(&ot.as_path(), contents).expect("write output file fail");
        if left.len() == 0 {
            println!("output rust glyphs file :{:?}", ot);
        } else {
            println!("output rust glyphs file :{:?}, but missing: {}", ot, left);
        }
    } else {
        println!("can not find glyphs, no output");
    }
}




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
// it only output 3 parts: s_glyphs, s_data, and final {name}.
// You maybe reorganize according to your needs. For example, put the s_data into eeprom, 
// write you code that read it from eeprom, build a BdfFont instance with reference to {name} and delete {name}. 
//
pub use  unformatted::{name};
#[rustfmt::skip]
mod unformatted {{
    use embedded_fonts::{{BdfGlyph,BdfFont}};
    use embedded_graphics::{{
        prelude::*,
        primitives::Rectangle,
    }};

    const s_glyphs:[BdfGlyph;{glyphs_count}] = {g};

    /// maybe you want store it in special secion(e.g. .eeprom), you can use below attributes
    /// ```no_run
    /// #[no_mangle]
    /// #[link_section = ".eeprom"]
    /// ```
    static S_DATA: [u8;{data_cout}] = {d};
    
    /// glyphs is [BdfGlyph;{glyphs_count}], data is [u8;{data_cout}]. 
    /// glyphs code include: "{list}"
    /// orig bdf file is {bdffile} 
    pub static  {name}: BdfFont = BdfFont{{
        glyphs: &s_glyphs,
        data : &S_DATA,
        line_height: {height},
        replacement_character:{replace},
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




#[cfg(test)]
mod test {
    use std::{
        ffi::OsStr,
        path::{Path, PathBuf},
    };

    #[test]
    fn it_adds_two() {
        let path = Path::new("/foo/bar");
        let parent = path.parent().unwrap();
        assert_eq!(parent, Path::new("/foo"));

        let path = Path::new("/foo/bar/");
        let parent = path.parent().unwrap();
        assert_eq!(parent, Path::new("/foo"));

        assert_eq!(OsStr::new("rs"), Path::new("foo.rs").extension().unwrap());
    }
}
