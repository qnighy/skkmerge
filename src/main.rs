extern crate regex;
extern crate encoding;
#[macro_use]
extern crate clap;

use std::io::{self, BufRead, BufReader};
use std::fs::File;
use std::str;
use std::mem::drop;
use encoding::DecoderTrap;
use encoding::label::encoding_from_whatwg_label;
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
        eprintln!("Loading {}...", filename);
        let file = File::open(filename).unwrap();
        let mut file = BufReader::new(file);
        let mut encoding = detect_encoding(&mut file).unwrap_or_else(
            || "euc-jp".to_string());
        if encoding == "euc-jis-2004" {
            eprintln!("euc-jis-2004: reading as euc-jp");
            encoding = "euc-jp".to_string();
        }
        let encoding = encoding_from_whatwg_label(&encoding).unwrap();
        let mut bytebuf = Vec::new();
        io::copy(&mut file, &mut bytebuf).unwrap();
        let s = encoding.decode(&bytebuf, DecoderTrap::Replace).unwrap();
        drop(bytebuf);
        for line in s.lines() {
            eprintln!("{:?}", line);
        }
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
