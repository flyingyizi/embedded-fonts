//! usage:
//! cargo run -- .\testdata\wenquanyi_12pt.bdf --range "china"
//!
mod util;
use util::conv_bdf;

use clap::Parser;
use std::{ffi::OsStr, fs, path::PathBuf};

#[derive(clap::Parser)]
struct Args {
    #[clap(help = "BDF input")]
    bdf_file: PathBuf,

    #[clap(
        long,
        help = "list of characters,defaultly export all glyphs in the bdf. e.g --range \"abc\" means only export a,b and c code's glyphs "
    )]
    range: Option<String>,

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

    let range = args.range;

    if let Some((contents, left)) = conv_bdf(args.bdf_file.as_path(), range) {
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
            println!("output rust glyphs file :{:?}, but missing: {}", ot , left );
        }
    } else {
        println!("can not find glyphs, no output");
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
