use std::fs;
use std::mem::transmute;
use std::time::Instant;

mod instructions;
mod vm;
mod byte_parse;

use MaybeRedundant::*;
use crate::byte_parse::RawParser;
use crate::instructions::IR;

fn main() {
    let read = fs::read_to_string("./src/test.rt").unwrap();
    let now = Instant::now();
    let raw_parser = RawParser::new(read).parse_after_entry();
    let elapsed = now.elapsed();
    println!("len: {:?}", raw_parser.len());
    println!("elapsed: {:?}", elapsed);
    //println!("{:#?}", raw_parser);
}


