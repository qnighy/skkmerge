extern crate regex;
extern crate encoding;
#[macro_use]
extern crate clap;

use std::io::{self, BufRead, BufReader};
use std::fs::File;
use std::str;
use std::mem::replace;
use encoding::{RawDecoder};
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
            encoding = "euc-jp".to_string();
        }
        let encoding = encoding_from_whatwg_label(&encoding).unwrap();
        for line in DecodedLines::new(
            file, RawDecoderProxy(encoding.raw_decoder())) {
            // eprint!(".");
            line.unwrap();
        }
    }
    for &filename in &subtract_filenames {
        println!("{:?}", filename);
    }
}

pub struct DecodedLines<I: BufRead, D: RawDecoder> {
    reader: I,
    decoder: D,
    buffer: String,
    finished: bool,
}

impl<I: BufRead, D: RawDecoder> DecodedLines<I, D> {
    pub fn new(reader: I, decoder: D) -> Self {
        Self { reader, decoder, buffer: "".to_string(), finished: false }
    }
}

impl<I: BufRead, D: RawDecoder> Iterator for DecodedLines<I, D> {
    type Item = io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if !self.finished {
                let (pos, err) = {
                    let buf = match self.reader.fill_buf() {
                        Ok(buf) => buf,
                        Err(e) => return Some(Err(e)),
                    };

                    if buf.is_empty() {
                        self.finished = true;
                        (0, self.decoder.raw_finish(&mut self.buffer))
                    } else {
                        self.decoder.raw_feed(buf, &mut self.buffer)
                    }
                };

                self.reader.consume(pos);

                if let Some(_) = err {
                    return Some(Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Decoding error")));
                }
            }
            if let Some(pos) = self.buffer.find("\n") {
                let line = self.buffer[..pos+1].to_string();
                self.buffer.drain(..pos+1);
                return Some(Ok(line));
            } else if self.finished {
                if self.buffer.is_empty() {
                    return None
                }
                return Some(Ok(replace(&mut self.buffer, "".to_string())));
            }
        }
    }
}

use encoding::{StringWriter, CodecError};

pub struct RawDecoderProxy(Box<RawDecoder>);

impl RawDecoder for RawDecoderProxy {
    fn from_self(&self) -> Box<RawDecoder> {
        self.0.from_self()
    }
    fn raw_feed(&mut self, input: &[u8], output: &mut StringWriter)
        -> (usize, Option<CodecError>) {
        self.0.raw_feed(input, output)
    }
    fn raw_finish(&mut self, output: &mut StringWriter) -> Option<CodecError> {
        self.0.raw_finish(output)
    }
    fn is_ascii_compatible(&self) -> bool {
        self.0.is_ascii_compatible()
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
