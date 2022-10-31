//! usage:
//! cargo run -- .\testdata\wenquanyi_12pt.bdf --range "china"
//!
mod util;
use util::conv_bdf;

use clap::Parser;
use clap::{App, Arg, ArgMatches, SubCommand};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};


fn main() {
    // let args: Args = Args::parse();

    let matches = App::new("convert bdf to rust code")
        .version("0.1")
        .author("flyingyizi <flyingyizi@gmail.com>")
        .about("Does awesome things")
        .arg(
            Arg::with_name("OUTPUT")
                .short('o')
                .long("output")
                .value_name("PATH")
                .help("output path. if not exist \".rs\" extention, will look it as dirctory, and use the bdf file's stem as its stem")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("RANGE")
                .long("range")
                .value_name("STRING")
                .help(r#"export characters list,defaultly export all glyphs in the bdf. e.g --range "abc" means only export a,b and c code's glyphs.
if exist range and range-file options at the same time. merge them as final exporting glyphs scope"#)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("RANGEFILE")
                .long("range-file")
                .value_name("PATH")
                .help("same as range option, but through file.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("bdf file path")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("SERIALIZE")
                .long("serialize")
                .required(false)
                .takes_value(false)
                .help("serialize flavor, output easy to used in storing glyphs in eeprom scenario"),
        )
        .get_matches();

    let to_serialize = matches.contains_id("SERIALIZE");

    let bdf_file = Path::new(matches.value_of("INPUT").unwrap()); //safe, because it is marked required

    if bdf_file.is_file() == false {
        println!("bdf file not exist");
        return;
    }

    let mut range_input_str = String::new();
    if let Some(s) = matches.value_of("RANGE") {
        range_input_str += s;
    }

    if let Some(s) = matches.value_of("RANGEFILE") {
        let p = Path::new(s);
        if p.is_file() {
            let mut range_from_file = String::new();
            for c in fs::read_to_string(p)
                .expect("couldn't open BDF file")
                .chars()
            {
                if c == '\r' || c == '\n' {
                } else {
                    range_from_file.push(c);
                }
            }
            range_input_str += range_from_file.as_str();
        } else {
            println!("input range file is not exist, ignore it:{:?}", p);
        }
    }

    let mut range_input: Option<String> = None;
    if range_input_str.len() > 0 {
        let _ = range_input.replace(range_input_str);
    }

    if let Some((contents, left)) = conv_bdf(&bdf_file, range_input, to_serialize) {
        let mut ot = Path::new(matches.value_of("OUTPUT").unwrap_or(".")).to_path_buf();

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
            ot = ot.join(bdf_file.file_stem().unwrap()).with_extension("rs");
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
