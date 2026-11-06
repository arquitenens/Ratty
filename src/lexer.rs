use std::arch::{asm, naked_asm};
use std::arch::x86_64::{__m256i, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8, _mm256_set1_epi8, _mm512_set1_epi64};
use std::cell::UnsafeCell;
use std::collections::{HashMap, HashSet};
use std::hint::black_box;
use std::mem::{transmute, transmute_copy};
use std::ptr;
use std::str::SplitWhitespace;
use std::time::Instant;
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
    pub const fn new_remainder<'rem>(&mut self, remainder: [u8; 8]) {
        self.remainder = remainder;
    }

    //produces the textual IR which no one wants to read lol
    pub fn try_lex_single(&mut self, line: &str) -> [u8; 16] {
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
            exclude @ ("local" | "global" | "const") => {
                self.parse_declaration(chunks, exclude.len() + 1)
            },
            _ => {
                println!("unrecognized keyword: {:?}", first);

                return unsafe{transmute_copy(&[emptied, emptied])}
            }
        };
        //Ignore the clear UB for now, im not using the return value any time soon nor am i constructing it,
        //its just so the code compiles
        return unsafe {transmute_copy(&[emptied, emptied])}
    }

    pub const fn parse_declaration(&mut self, s: &[[u8; 8]], start_at: usize) -> &str{
        let ident = self.parse_identifier(s, start_at);
        let ty = ();
        let val = ();

        ident

    }

    pub fn parse_type(){}

    #[inline]
    pub const fn parse_identifier(&mut self, s: &[[u8; 8]], start_at: usize) -> &str {
        //look if you don't have avx512 that's okay!!
        //but if you can't do avx2 then come on that is not on me
        //check ident_size
        //TODO proper bounds checking probably only works since theres lot of spaces in a usual text
        //It probably doesnt matter because who in their right mind would write "local x = 0\n" like that's mad
        let ident_len = unsafe {

            let ptr = s.as_ptr().cast::<u8>();

            let loaded = _mm256_loadu_si256(ptr.cast::<__m256i>());

            //well the ident ends with a colon
            let target = _mm256_set1_epi8(':' as i8);
            let mask = _mm256_cmpeq_epi8(loaded, target);

            let bitmask = _mm256_movemask_epi8(mask);
            assert!(bitmask != 0, "the identifiers/variable names are capped at 32 characters");
            bitmask.trailing_zeros() as usize - start_at

        };

        //not using a "transmute" because im really safe and never using any unsafe code
        let flattened = s.as_flattened();

        let ident_bytes = unsafe {
            core::slice::from_raw_parts(
                flattened.as_ptr().add(start_at),
                ident_len,
            )
        };

        return unsafe {str::from_utf8_unchecked(&ident_bytes)};
    }
    
    pub fn try_parse_type(){}

    pub fn try_function(&mut self) {}

    #[inline]
    pub const fn valid_char(&self, singleton_ident: char, is_first: bool) -> bool {
        return if is_first{
            matches!(singleton_ident, 'a'..='z' | 'A'..='Z' | '_' )
        }else {
            matches!(singleton_ident, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9')
        }
    }
    pub fn valid_ident(&self, ident: &str) -> bool {
        !self.hs.contains(ident)
    }

    pub fn next_expected(next: &str, expected: &str) -> bool {
        next == expected
    }
    pub fn check_keyword(&self, keyword: &str) -> Option<&&str> {
        self.hs.get(keyword)
    }
}