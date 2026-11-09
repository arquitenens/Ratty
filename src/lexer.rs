use std::any::Any;
use std::arch::{asm, naked_asm};
use std::arch::x86_64::{__m256i, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8, _mm256_set1_epi8, _mm512_set1_epi64};
use std::cell::{Cell, UnsafeCell};
use std::collections::{HashMap, HashSet};
use std::hint::black_box;
use std::io::Read;
use std::mem::{transmute, transmute_copy};
use std::{ptr, slice};
use std::str::SplitWhitespace;
use std::time::Instant;
use crate::instructions::IR;
use crate::ir_parse::{repeat_byte, string_pos};


pub struct Lexer<'a>{
    keywords: Vec<
        &'a [&'a str]
    >,
    remainder: [u8; 8]
}
//TODO if i forget to change these values im in trouble
//in case i want tl
const SHORTEST_KEYWORD: usize = 3;
const LONGEST_KEYWORD: usize = 6;

impl<'a> Lexer<'a> {
    pub fn new() -> Lexer<'a> {
        Lexer{
            //TODO probably precalculate the "hashes"/masks and store them statically for performance
            //the reason this is not a static array and instead a growable vec is-
            //im thinking about being able to "overwrite" and define custom keywords at compile-time and or runtime
            //which requires being apple to append keywords as long as they are less than 8 chars long
            keywords: vec![
                //0 case, reserved to not handle awkward indexing
                &[""],
                //1 len, probably unused
                &[""],
                //2 len
                &[""],
                //3 len
                &["end"],
                //4 len
                &["task", "func",],
                //5 len
                &["local"],
                //6 len
                &["struct", "global"],
                //7 len
                &[""]
            ],
            remainder: [0u8; 8]
        }
    }
    pub const fn new_remainder<'rem>(&mut self, remainder: [u8; 8]) {
        self.remainder = remainder;
    }

    //produces the textual IR which no one wants to read lol
    pub fn try_lex_single(&mut self, line: &'a str) -> [u8; 16] {
        //println!("line: {:?}", line);
        //No keyword is longer than 8 chars so we use the string pos func
        let emptied = [0u8; 8];

        if line.is_empty() {
            return unsafe {transmute_copy(&[emptied, emptied])}
        }

        let (chunks, rem) = line.as_bytes().as_chunks::<8>();

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
                //self.parse_declaration(chunks, exclude.len() + 1);

                let now = Instant::now();
                for i in 0..100_000_000{
                    let x = self.parse_declaration(chunks, exclude.len() + 1);
                    black_box(x);
                }
                let elapsed = now.elapsed();
                println!("elapsed {:?}", elapsed);

            },
            _ => {
                //println!("unrecognized keyword: {:?}", first);

                return unsafe{transmute_copy(&[emptied, emptied])}
            }
        };
        //Ignore the clear UB for now, im not using the return value any time soon nor am i constructing it,
        //its just so the code compiles
        return unsafe {transmute_copy(&[emptied, emptied])}
    }


    const fn zero_extend_at_offset(s: &[[u8; 8]], offset: usize) -> ([u8; 8], usize) {
        let idx_8 = offset >> 3;
        //if 0 then the current thingy doesn't have anything
        //good explanation I know
        static LOOKUP: [u8; 8] = [0, 1, 1, 1, 1, 1, 1, 1];
        let stripped_offset = offset & 7;

        let non_full = LOOKUP[stripped_offset] as usize * idx_8;
        //assumes lil endian machines
        let mut extended = u64::from_le_bytes(s[non_full]);

        extended >>= stripped_offset * 8;
        //should be the opposite endian but there doesn't seem to be a
        //"to_opposite_bytes" function

        return (extended.to_be_bytes(), stripped_offset);
    }

    pub fn parse_declaration(&mut self, s: &'a [[u8; 8]], start_at: usize) -> &str{

        let (ident, offset) = self.parse_identifier(s, start_at);
        let (first, z_offset) = Self::zero_extend_at_offset(s, offset);
        black_box(ident);

        let (ty, offset) = self.parse_type(first, s, z_offset);
        let (first, z_offset) = Self::zero_extend_at_offset(s, offset);
        black_box(ty);

        let (val, _offset) = self.parse_value(first, s, z_offset);

        black_box(val);

        ""
    }


    #[inline]
    pub fn parse_identifier<'b>(&mut self, s: &'a [[u8; 8]], start_at: usize) -> (&'b str, usize) where 'a: 'b{
        //look if you don't have avx512 that's okay!!
        //but if you can't do avx2 then come on that is not on me
        //check ident_size
        //TODO proper bounds checking probably only works since theres lot of spaces in a usual text
        //It probably doesnt matter because who in their right mind would write "local x = 0\n" like that's mad
        let ident_len = unsafe {

            let ptr = s.as_ptr().cast::<u8>().add(start_at);

            let loaded = _mm256_loadu_si256(ptr.cast::<__m256i>());


            //well the ident ends with a colon
            let target = _mm256_set1_epi8(':' as i8);
            let mask = _mm256_cmpeq_epi8(loaded, target);

            let bitmask = _mm256_movemask_epi8(mask);
            assert!(bitmask != 0, "the identifiers/variable names are capped at 32 characters");
            //TODO look further ahead to see if a colon is every present, if not throw a syntax error instead
            bitmask.trailing_zeros() as usize

        };



        let raw = s.as_flattened();

        //no keyword is longer than 8 chars
        //im using 64 since no alphanumeric ascii char is 64, thus I can easily filter out empty spaces

        //println!("indent_len {:?}", ident_len);
        let i = &raw[start_at..start_at + ident_len];

        self.check_valid_ident(i);

        let x = unsafe {str::from_utf8_unchecked(i)};

        return (x, ident_len + start_at);
    }
    
    pub fn parse_type(&mut self, first: [u8; 8], s: &[[u8; 8]], z_offset: usize) -> (&str, usize) {
        return ("", z_offset);
    }

    pub fn parse_value(&mut self, first: [u8; 8], s: &[[u8; 8]], z_offset: usize) -> (&str, usize) {
        return ("", z_offset);
    }

    //panic for now, error handling later
    pub fn check_valid_ident(&self, s: &[u8]){
        let mut lil_buff = [64u8; 8];

        //using the funny binding thingy to get the proper range, could use a naive if statement
        //but im feeling fancy today
        //i could just get every match between SHORTEST_KEYWORD..=LONGEST_KEYWORD for some future optimization
        //but for flexibility in terms of user-defined keywords it stays simple
        if let len @ ..8 = s.len(){

            unsafe {
                ptr::copy_nonoverlapping(s.as_ptr(), lil_buff.as_mut_ptr(), len);
            }

            //the 64 i filled the buffer with before
            //for the keen eye you notice im ignoring 48 to 57 because well.. no keyword has numbers in it! but an identifier might.
            //but im looking for keywords not identifiers so uhm i dont care lol
            const MASK: u8 = '@' as u8;
            //TODO this is very naive and very slow dropping performance by like 3 times when the ident is 6 in length

            let max_kw = self.keywords[len].len();
            let mut current_kw = 0;
            while current_kw < max_kw{
                let c = &lil_buff[..len];
                assert!(self.keywords[len][current_kw].as_bytes() != &lil_buff[..len],
                        "your identifiers cant be the same as a keyword {:?}",
                        str::from_utf8(c)
                );
                current_kw += 1;
            }


        }
    }
    #[inline]
    pub const fn valid_char(&self, singleton_ident: char, is_first: bool) -> bool {
        return if is_first{
            matches!(singleton_ident, 'a'..='z' | 'A'..='Z' | '_' )
        }else {
            matches!(singleton_ident, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9')
        }
    }

    pub fn next_expected(next: &str, expected: &str) -> bool {
        next == expected
    }
}