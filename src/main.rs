extern crate regex;
#[macro_use]
extern crate clap;

use std::io::{Read, BufRead, BufReader};
use std::fs::File;
use std::str;
use clap::{Arg, App};

fn main() {
    let matches =
        App::new("skkmerge")
        .version(crate_version!())
        .author("Masaki Hara <ackie.h.gmai@gmail.com>")
        .arg(Arg::with_name("subtract_files").multiple(true)
             .number_of_values(1)
             .short("-s")
             .long("--subtract")
             .takes_value(true)
             .value_name("FILE")
             .help("Specify a file to subtract."))
        .arg(Arg::with_name("files").multiple(true)
             .value_name("FILES")
             .help("Specify files to merge."))
        .get_matches();

    let merge_filenames = matches.values_of("files").map(|a| a.collect())
        .unwrap_or_else(|| vec!["-"]);
    let subtract_filenames = matches.values_of("subtract_files")
        .map(|a| a.collect()).unwrap_or_else(|| vec![]);
    for &filename in &merge_filenames {
        let file = File::open(filename).unwrap();
        let mut file = BufReader::new(file);
        let encoding = detect_encoding(&mut file).unwrap_or_else(|| "euc-jp".to_string());
        println!("{:?}", encoding);
    }
    for &filename in &subtract_filenames {
        println!("{:?}", filename);
    }
}

fn detect_encoding<I: BufRead>(f: &mut I) -> Option<String> {
    use regex::bytes::Regex;

    let buf = f.fill_buf().unwrap();
    if &buf[..2] == b"\xFE\xFF" {
        return Some("utf-16be".to_string());
    }
    if &buf[..2] == b"\xFF\xFE" {
        return Some("utf-16le".to_string());
    }

    let eol = buf.iter().position(|&b| b == b'\r' || b == b'\n')
        .unwrap_or(buf.len());
    let buf = &buf[..eol];

    let re = Regex::new(r"(?-u)coding:\s*([a-zA-Z0-9_-]+)").unwrap();
    if let Some(m) = re.captures(buf) {
        let name = m.get(1).unwrap().as_bytes();
        if let Ok(name) = str::from_utf8(name) {
            return Some(name.to_string());
        }
    }

    return None;
}
