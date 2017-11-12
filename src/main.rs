extern crate regex;
extern crate encoding;
#[macro_use]
extern crate clap;

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader};
use std::fs::File;
use std::str;
use regex::Regex;
use encoding::DecoderTrap;
use encoding::label::encoding_from_whatwg_label;
use clap::{Arg, App};

fn main() {
    let matches =
        App::new("skkmerge")
        .version(crate_version!())
        .author("Masaki Hara <ackie.h.gmai@gmail.com>")
        .arg(Arg::with_name("retain_okuri_entries")
             .long("--retain-okuri-entries")
             .help("Retain okuri entries like /[け /分/]/."))
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

    let retain_okuri_entries = matches.is_present("retain_okuri_entries");

    let comment_re = Regex::new(r"^\s*;").unwrap();
    let part_re = Regex::new(r"\[[^]]*\]|[^/]+").unwrap();

    let mut entries : HashMap<String, Vec<String>> = HashMap::new();

    let merge_filenames = matches.values_of("files").map(|a| a.collect())
        .unwrap_or_else(|| vec!["-"]);
    let subtract_filenames = matches.values_of("subtract_files")
        .map(|a| a.collect()).unwrap_or_else(|| vec![]);
    for &filename in &merge_filenames {
        eprintln!("Loading {}...", filename);
        let file = File::open(filename).unwrap();
        let file = BufReader::new(file);
        let s = read_all_encoded(file);
        for line in s.lines() {
            let line = line.trim();
            if comment_re.is_match(line) {
                continue;
            }
            let spc = line.find(" ").unwrap_or(line.len());
            let hira = &line[..spc];
            let candidates = &line[spc+1..];
            let v = entries.entry(hira.to_string()).or_insert(vec![]);
            for part in part_re.find_iter(candidates) {
                let part = part.as_str();
                let part = if part.starts_with("[") && part.ends_with("]") {
                    if retain_okuri_entries {
                        &part[1..part.len()-1]
                    } else {
                        continue;
                    }
                } else {
                    part
                };
                if !v.iter().any(|x| x == part) {
                    v.push(part.to_string());
                }
            }
            // eprintln!("{:?}", (hira, candidates));
            // for part in part_re.find_iter(candidates) {
            //     // eprintln!("{:?}", part);
            // }
        }
    }
    for &filename in &subtract_filenames {
        eprintln!("Loading {}...", filename);
        let file = File::open(filename).unwrap();
        let file = BufReader::new(file);
        let s = read_all_encoded(file);
        for line in s.lines() {
            let line = line.trim();
            if comment_re.is_match(line) {
                continue;
            }
            let spc = line.find(" ").unwrap_or(line.len());
            let hira = &line[..spc];
            let candidates = &line[spc+1..];
            let v = if let Some(v) = entries.get_mut(hira) {
                v
            } else {
                continue;
            };
            for part in part_re.find_iter(candidates) {
                let part = part.as_str();
                let part = if part.starts_with("[") && part.ends_with("]") {
                    &part[1..part.len()-1]
                } else {
                    part
                };
                v.retain(|x| x != part);
            }
        }
    }
    eprintln!("Sorting...");
    let mut entries : Vec<_> = entries.drain().collect();
    entries.retain(|&(_, ref v)| !v.is_empty());
    entries.sort();
    for &(ref hira, ref v) in &entries {
        print!("{} /", hira);
        for part in v {
            if part.find("/").is_some() {
                print!("[{}]/", part);
            } else {
                print!("{}/", part);
            }
        }
        println!("");
    }
}

fn read_all_encoded<I: BufRead>(mut file: I) -> String {
    let mut encoding = detect_encoding(&mut file).unwrap_or_else(
        || "euc-jp".to_string());
    if encoding == "euc-jis-2004" {
        eprintln!("euc-jis-2004: reading as euc-jp");
        encoding = "euc-jp".to_string();
    }
    let encoding = encoding_from_whatwg_label(&encoding).unwrap();
    let mut bytebuf = Vec::new();
    io::copy(&mut file, &mut bytebuf).unwrap();
    return encoding.decode(&bytebuf, DecoderTrap::Replace).unwrap();
}

fn detect_encoding<I: BufRead>(f: &mut I) -> Option<String> {
    let (ret, pos) = detect_encoding_from_buf(f.fill_buf().unwrap());
    f.consume(pos);
    ret
}

fn detect_encoding_from_buf(buf: &[u8]) -> (Option<String>, usize) {
    use regex::bytes::Regex;

    if &buf[..2] == b"\xFE\xFF" {
        return (Some("utf-16be".to_string()), 2);
    }
    if &buf[..2] == b"\xFF\xFE" {
        return (Some("utf-16le".to_string()), 2);
    }
    if &buf[..3] == b"\xEF\xBB\xBF" {
        return (Some("utf-8".to_string()), 3);
    }

    let eol = buf.iter().position(|&b| b == b'\r' || b == b'\n')
        .unwrap_or(buf.len());
    let buf = &buf[..eol];

    let re = Regex::new(r"(?-u)coding:\s*([a-zA-Z0-9_-]+)").unwrap();
    if let Some(m) = re.captures(buf) {
        let name = m.get(1).unwrap().as_bytes();
        if let Ok(name) = str::from_utf8(name) {
            return (Some(name.to_string()), 0);
        }
    }

    return (None, 0);
}
