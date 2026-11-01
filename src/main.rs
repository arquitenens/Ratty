use std::fs;
use std::hint::black_box;
use std::iter::repeat_with;
use std::mem::transmute;
use std::time::Instant;

mod instructions;
mod vm;
mod ir_parse;
mod lexer;

use MaybeRedundant::*;
use crate::ir_parse::RawParser;
use crate::instructions::{IROps, RType, Register, IR};

fn main() {


    let size = size_of::<IR>();
    let align = align_of::<IR>();

    println!("size: {}", size);
    println!("align: {}", align);

    //around 480 million lines per second on my cpu
    let t_120m = "./src/parsingtests/textual_ir/Bigfile.rir";
    let t_1m = "./src/parsingtests/textual_ir/Bigfile1m.rir";
    let small_test = "./src/parsingtests/textual_ir/small_ir.rir";

    let read = fs::read_to_string(small_test).unwrap();
    let mut raw_parser = RawParser::new(read.clone());
    //let expected = raw_parser.parse_after_entry();
    //println!("{:#?}", expected);

    let now = Instant::now();
    for line in unsafe {read.as_bytes().as_chunks_unchecked::<16>()} {
        let x = RawParser::parse_single(line);
        println!("{:?}", x);
        black_box(x);
    }
    let elapsed = now.elapsed();
    println!("elapsed: {:?}", elapsed);
    println!("len: {:?}", read.lines().count());



}


