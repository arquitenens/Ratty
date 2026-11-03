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

    pub fn parse_declaration(&mut self, s: &[[u8; 8]], start_at: usize) -> [u8; 16] {
        println!("");
        if !self.remainder.is_empty() {

        }
        //look if you don't have avx512 that's okay!!
        //but if you can't do avx2 then come on that is not on me
        //check ident_size
        let ident_len = unsafe {
            let ptr = s.as_ptr().cast::<u8>();

            let loaded = _mm256_loadu_si256(ptr.cast::<__m256i>());

            let target = _mm256_set1_epi8(58);
            let mask = _mm256_cmpeq_epi8(loaded, target);

            let bitmask = _mm256_movemask_epi8(mask);
            assert_ne!(bitmask, 0, "the identifiers/variable names are capped at 32 characters");
            bitmask.trailing_zeros() as usize - start_at
        };

        //parse ident

        println!("ident_len: {}", ident_len);
        let ident = self.try_parse_ident(start_at);


        return [0; 16]
    }

    //i don't know what an error looks like yet
    //so it just returns a string, always apparently
    pub fn try_parse_ident(&mut self, start_at: usize) -> [u8; 32]{
        let mut first_chunk = true;
        let mut ident_done = false;

        //TODO replace with static array, e.g. 32 bytes, no reason to have-
        //an ident with more than 32 chars
        let mut ident_collect = [0u8; 32];



        let mut total_offset = 0;


        return ident_collect
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