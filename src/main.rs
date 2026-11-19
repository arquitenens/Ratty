

use std::fs;
use std::hint::black_box;
use std::iter::repeat_with;
use std::mem::transmute;
use std::time::Instant;

mod instructions;
mod vm;
mod ir_parse;
pub(crate) mod lexer;
mod util;
mod tests;

use MaybeRedundant::*;
use crate::ir_parse::RawParser;
use crate::instructions::{IROps, RType, Register, IR};
use crate::lexer::{Lexer, StreamChunk};





fn main() {
    
    //around 480 million lines per second on my cpu
    let t_120m_ir = "./src/parsingtests/textual_ir/Bigfile.rir";
    let t_1m_ir = "./src/parsingtests/textual_ir/Bigfile1m.rir";
    let t_small_ir = "./src/parsingtests/textual_ir/small_ir.rir";

    let small_test = "./src/parsingtests/dec_test.rt";

    let read = fs::read_to_string(small_test).unwrap();
    
    assert!(read.is_ascii());
    let mut lexer = Lexer::new();

    let now = Instant::now();
    for i in read.split("\n") {
        let _ = lexer.try_lex_single(i);
    }

}


