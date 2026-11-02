use std::cell::UnsafeCell;
use std::collections::{HashMap, HashSet};
use std::mem::{transmute, transmute_copy};
use std::ptr;
use std::str::SplitWhitespace;
use crate::instructions::IR;
use crate::ir_parse::string_pos;


pub struct Lexer<'a>{
    hs: HashSet<&'a str>,
    remainder: [u8; 8]
}
impl<'a> Lexer<'a> {
    pub fn new() -> Lexer<'a> {
        Lexer{
            hs: HashSet::from([
                "local",
                "global",
                "task",
                "func",
                "struct",
                "end"
            ]),
            remainder: [0u8; 8]
        }
    }
    pub fn new_remainder<'rem>(&mut self, remainder: [u8; 8]) {
        self.remainder = remainder;
    }

    //produces the textual IR which no one wants to read lol
    pub fn try_lex_single(&mut self, line: &str) -> &[u8; 16] {
        //println!("line: {:?}", line);
        //No keyword is longer than 8 chars so we use the string pos func
        let emptied = [0u8; 8];

        if line.is_empty() {
            return unsafe {transmute_copy(&[emptied, emptied])}
        }
        //dont want this mut, but will do for now
        let (chunks, rem) = unsafe {line.as_bytes().as_chunks::<8>()};

        let mut keyword_buff = emptied;

        let mut key_end = 0;

        if chunks.is_empty() {
            key_end = rem.len();
            unsafe {ptr::copy_nonoverlapping(rem.as_ptr(), keyword_buff.as_mut_ptr(), rem.len())};
        }else {
            key_end = string_pos(u64::from_le_bytes(chunks[0]), ' ');
            keyword_buff = chunks[0]
        }


        if key_end == 8{
            return unsafe {transmute_copy(&[emptied, emptied])}
        }
        let first = std::str::from_utf8(&keyword_buff[0..key_end]).unwrap();

        self.new_remainder(keyword_buff);

        let x = match first {
            exclude @ ("local" | "global" | "const") => self.parse_declaration(chunks, exclude.len() + 1),
            _ => {
                println!("unrecognized keyword: {:?}", first);

                return unsafe{transmute_copy(&[emptied, emptied])}
            }
        };
        //Ignore the clear UB for now, im not using the return value any time soon nor am i constructing it,
        //its just so the code compiles
        return unsafe {transmute_copy(&[emptied, emptied])}
    }

    pub const fn chunk_fill_zeros(chunk: &[u8]) -> [u8; 8]{
        assert!(chunk.len() < 8);
        let mut emptied = [0u8; 8];
        unsafe {ptr::copy_nonoverlapping(chunk.as_ptr(), emptied.as_mut_ptr(), chunk.len())};
        emptied
    }
    pub fn parse_declaration(&mut self, s: &[[u8; 8]], start_at: usize) -> [u8; 16] {

        if !self.remainder.is_empty() {

        }

        let mut first_chunk = true;

        for chunk in s{
            let chunk = if first_chunk {&Lexer::chunk_fill_zeros(&s[0][start_at..])} else {chunk};
            first_chunk = false;
            println!("s: {:?}", str::from_utf8(chunk).unwrap());
            let p = string_pos(u64::from_le_bytes(*chunk), ' ');
            if p != 8{

            }
        }


        return [0; 16]
    }

    pub fn try_parse_ident(&mut self, s: &[[u8; 8]]) -> Result<(), i32>{
        return Ok(())
    }

    pub fn try_function(&mut self) {}

    pub fn valid_identifier(&self, ident: &str) -> bool {
        matches!(ident.chars().next().unwrap(), 'a'..='z' | 'A'..='Z') && !self.hs.contains(ident)
    }
    pub fn next_expected(next: &str, expected: &str) -> bool {
        next == expected
    }
    pub fn check_keyword(&self, keyword: &str) -> Option<&&str> {
        self.hs.get(keyword)
    }
}