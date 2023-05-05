//! usage:
//! convert-bdf  .\testdata\wenquanyi_12pt.bdf --range "china"
//!
pub use conv::conv_bdf;

use clap::{Arg, ArgAction, Command};
use std::{ffi::OsStr, fs, path::PathBuf};

fn main() {
    let _ = run();
}

fn run() -> Result<(), ()> {
    let app = Command::new("convert-bdf")
         .arg_required_else_help(true)
        .about(
r#"Generate embedded-graphic accepted Glyphs from bdf fonts file. 
if exist multi range* options at the same time. merge them as final exporting glyphs scope"
"#
)
        .arg(
            Arg::new("input")
                .long("bdffile")
                .help("Input bdf file")
                .short('i')
                .value_parser(clap::value_parser!(PathBuf))
                .required(true)
                .action(ArgAction::Set)
                .value_name("FILE"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .help("output path. if not exist \".rs\" extention in it, will look it as dirctory, and use the bdf file's stem as its stem.")
                .short('o')
                .value_parser(clap::value_parser!(PathBuf))
                .default_value("./")
                .action(ArgAction::Set)
                .value_name("PATH"),
        )
        .arg(
            Arg::new("range")
                .long("range")
                .help(
r#"export characters list,defaultly export all glyphs in the bdf. e.g --range "abc" means only export a,b and c code's glyphs. 
"#
                )
                // .value_parser(clap::value_parser!(String))
                .action(ArgAction::Append)
                .value_name("RANGE"),
        )
        .arg(
            Arg::new("range-file")
                .long("range-file")
                .help(
                   r#"same as range option, but through characters file."#
                )
                .value_parser(clap::value_parser!(PathBuf))
                .action(ArgAction::Append)
                .value_name("RANGEFILE"),
        )
        .arg(
            Arg::new("range-path")
                .long("range-path")
                .help(
                   r#"same as range option, but through rust source directory. it will colllect the first paraments of all Text::new stmts as the input characters list"#
                )
                .value_parser(clap::value_parser!(PathBuf))
                .action(ArgAction::Append)
                .value_name("RANGEPATH"),
        )
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        // .version(concat!(
        //     env!("CARGO_PKG_VERSION"),
        //     include_str!(concat!(env!("OUT_DIR"), "/commit-info.txt"))
        // ))
        ;

    let matches = app.get_matches();

    let bdf_file = matches.get_one::<PathBuf>("input").unwrap();
    if bdf_file.is_file() == false {
        println!("bdf file not exist");
        return Err(());
    }

    let mut output = matches.get_one::<PathBuf>("output").unwrap().clone();

    let mut range_input: Option<String> = None;
    {
        let mut store = Vec::<char>::new();
        if let Some(paths) = matches.get_many::<PathBuf>("range-file") {
            for p in paths.collect::<Vec<_>>() {
                if p.is_file() {
                    for c in fs::read_to_string(p)
                        .expect("couldn't open BDF file")
                        .chars()
                    {
                        if c != '\r' && c != '\n' {
                            store.push(c);
                        }
                    }
                } else {
                    println!("input range file is not exist, ignore it:{:?}", p);
                }
            }
        }
        if let Some(paths) = matches.get_many::<PathBuf>("range-path") {
            for p in paths.collect::<Vec<_>>() {
                if p.is_dir() {
                    let from_rust = collect_chars_from_ast::dump_total(p.as_path());
                    store.extend(from_rust.chars());
                } else {
                    println!("input range path is not directory, ignore it:{:?}", p);
                }
            }
        }
        if let Some(ss) = matches.get_many::<String>("range") {
            for p in ss.collect::<Vec<_>>() {
                store.extend(p.chars());
            }
        }
        if store.len() > 0 {
            store.sort();
            store.dedup();
            range_input.replace(store.iter().collect::<String>());
        }
    }

    if let Some((contents, left)) = conv_bdf(bdf_file.as_path(), range_input) {
        if Some(OsStr::new("rs")) == output.extension() {
            // make sure directory exist. if not exist, create it
            if let Some(parent) = output.parent() {
                if parent.is_dir() == false {
                    std::fs::create_dir_all(parent).expect("could'nt create not exist dir");
                }
            }
        } else {
            if output.is_dir() == false {
                std::fs::create_dir_all(output.as_path()).expect("could'nt create not exist dir");
            }
            output = output
                .join(bdf_file.file_stem().unwrap())
                .with_extension("rs");
        }

        fs::write(&output.as_path(), contents).expect("write output file fail");
        if left.len() == 0 {
            println!("output rust glyphs file :{:?}", output);
        } else {
            println!(
                "output rust glyphs file :{:?}, but missing: {}",
                output, left
            );
        }
    } else {
        println!("can not find glyphs, no output");
    }

    Ok(())
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

        let final_scope: HashSet<_> = chars_range_set
            .difference(&left_chars_range_set)
            .map(|c| c.clone())
            .collect();
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

/// helper to collect chars that inputed by Text::new
mod collect_chars_from_ast {
    use ignore::Walk;
    use quote::ToTokens;
    use std::{fs, path::Path};
    use syn::visit::{self, Visit};

    pub fn dump_total(path: &Path) -> String {
        let mut total = String::new();

        let walk = Walk::new(path)
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|e| e == "rs").unwrap_or(false))
            .map(|e| e.into_path());

        for path in walk {
            let mut vis = TextNewVisitor::new();
            let contents = fs::read_to_string(path).unwrap();
            let ast = syn::parse_file(&contents).unwrap();
            vis.visit_file(&ast);

            let all = vis.get_allchars();
            total.push_str(all.as_str());
        }
        total
    }

    /// to collect the Text::new's first param
    struct TextNewVisitor {
        pub chars: Vec<String>,
    }
    impl TextNewVisitor {
        pub fn new() -> Self {
            Self {
                chars: Vec::<String>::new(),
            }
        }
        /// collect all unique chars that is in the Text::new first param.
        pub fn get_allchars(&self) -> String {
            let mut result_set = std::collections::HashSet::new();
            for s in &self.chars {
                for c in s.chars() {
                    result_set.insert(c);
                }
            }
            let mut result = String::new();
            for i in result_set {
                result.push(i);
            }
            result
            // println!("{:?}", result);
        }

        pub fn get_expr_path(p: &syn::Expr) -> Option<String> {
            match p {
                syn::Expr::Path(exprpath) => {
                    let mut result = String::new();
                    for seg in exprpath.path.segments.pairs() {
                        let (a, b) = seg.into_tuple();
                        result = result + a.ident.to_string().as_str();
                        if let Some(b) = b {
                            result = result + b.into_token_stream().to_string().as_str();
                        }
                    }
                    return Some(result);
                }
                _ => {
                    return None;
                }
            }
        }

        pub fn get_expr_lit(p: &syn::Expr) -> Option<String> {
            match p {
                syn::Expr::Lit(l) => {
                    return Some(l.lit.to_token_stream().to_string());
                }
                _ => return None,
            }
        }
    }

    impl<'ast> Visit<'ast> for TextNewVisitor {
        /// visit `pub const fn new(text: &'a str, position: Point, character_style: S) -> Self`
        /// in embedded_graphics::text::text.
        fn visit_expr_call(&mut self, i: &'ast syn::ExprCall) {
            let name = String::from("Text::new");
            if Some(name.clone()) == Self::get_expr_path(&i.func) {
                let t = Self::get_expr_lit(&i.args[0]).unwrap();
                self.chars.push(t);
                // println!("Function with 123={}({:?})", name, t);
            }

            visit::visit_expr_call(self, i);
        }
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
