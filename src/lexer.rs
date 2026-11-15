use std::arch;
use std::arch::x86_64::_pext_u64;
use std::hint::{black_box, unreachable_unchecked};
use std::mem::{transmute, transmute_copy};
use std::num::NonZero;
use std::ptr::slice_from_raw_parts;
use std::time::Instant;
use crate::lexer::KeywordSlot::Occupied;
use crate::util::ConstVec;
//TODO better approach than nested vec



#[derive(Copy, Clone)]
enum KeywordSlot{
    Occupied(Keyword),
    Empty
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum Analyzer{
    Warning(&'static str),
    Error(&'static str),
}




pub struct Lexer {
    //no keyword is longer than 8 that's why 8 static array len thing
    keywords: [ConstVec<Keyword, 8>; 8],
    remainder: [u8; 8]
}

pub(crate) type Stream<'a> = &'a[u8];
pub(crate) type StreamChunk = [u8; 8];

#[repr(align(8))]
#[derive(Copy,Clone)]
struct Keyword{
    pub(crate) hash: u64,
    pub(crate) stripped_len: usize,
    //raw name for error reporting
    //it cant really be a static str slice since it might change
    pub(crate) raw_name: [u8; 8],
}

impl Keyword{
    pub const fn make(s: &str) -> Keyword{
        //32 for easy ascii stripping
        let mut kw = [32u8; 8];
        //64 easier for masking
        let mut bytes = [64u8; 8];

        let raw_s = s.as_bytes();
        let mut counter = 0usize;
        while counter < s.len() {
            bytes[counter] = raw_s[counter];
            kw[counter] = raw_s[counter];
            counter += 1;
        }


        return Keyword{
            hash: u64::from_ne_bytes(bytes),
            stripped_len: s.len(),
            raw_name: kw
        }
    }
}

impl<'a> Lexer {
    pub const fn new() -> Lexer {
        Lexer{
            //the reason this is not a static array and instead a growable vec is-
            //im thinking about being able to "overwrite" and define custom keywords at compile-time and or runtime
            //which requires being apple to append keywords as long as they are less than 8 chars long
            //They cant be empty otherwise the returned pointer wont actually have an address
            keywords: [
                //0 case, reserved to not handle awkward indexing
                ConstVec::from_const_array([Keyword::make("_")]),
                //1 len, probably unused
                ConstVec::from_const_array([Keyword::make("_")]),
                //2 len
                ConstVec::from_const_array([Keyword::make("_")]),
                //3 len
                ConstVec::from_const_array([Keyword::make("end"), Keyword::make("dyn")]),
                //4 len
                ConstVec::from_const_array([Keyword::make("task"), Keyword::make("func")]),
                //5 len
                ConstVec::from_const_array([Keyword::make("local")]),
                //6 len
                ConstVec::from_const_array([Keyword::make("struct"), Keyword::make("global")]),
                //7 len
                ConstVec::from_const_array([Keyword::make("_")])
            ],
            remainder: [0u8; 8]
        }
    }


    //produces the textual IR which no one wants to read lol
    pub fn try_lex_single(&mut self, line: &'a str) -> [u8; 16] {
        if line.is_empty(){
            //well we can just ignore empty lies
            return [0; 16];
        }
        assert!(line.is_ascii(), "Only ascii is allowed");

        let raw_lines: Stream = line.as_bytes();
        //after this unchecked conversions should be fine

        let emptied: StreamChunk = [0u8; 8];



        let kw = Self::load8_extend(raw_lines);

        let maybe_kw = Self::slice8_split_and_extend_at_char::<' ', '@'>(kw);

        let x = match &maybe_kw {
            //I don't want to do it like this but rust seems a bit stupid with its const matching
            exclude @ (b"local@@@" | b"global@@" | b"const@@@") => {

                unsafe {
                    let stripped_pos = Self::char_pos::<'@'>(u64::from_le_bytes(maybe_kw));

                    let new = Self::add_u8_slice_offset(raw_lines, stripped_pos + 1);

                    /*
                    let now = Instant::now();
                    for i in 0..100_000_000{
                        let res = self.parse_ident(new);
                        black_box(res);
                    }
                    let elapsed = now.elapsed();
                    println!("elapsed: {:?}", elapsed);


                     */
                    let res = self.parse_ident(new);

                }

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

    const fn advance_stream8_get_idx<const C: char>(stream: Stream, added_offset: usize) -> (Stream<'_>, StreamChunk, Option<NonZero<usize>>){
        let offset_kw_stream = unsafe {Self::add_u8_slice_offset(stream, added_offset)};
        let ident = Self::load8_extend(offset_kw_stream);
        let maybe_valid_pos = Self::char_pos_checked::<C>(u64::from_le_bytes(ident));

        return (offset_kw_stream, ident, maybe_valid_pos);
    }

    const fn parse_ident(&mut self, kw: Stream) -> Stream<'_>{

        let (stream, ident, pos) = Self::advance_stream8_get_idx::<':'>(kw, 0);

        if let Some(p) = pos{
            //Fast path
            //Quick dirty check, better error handling later
            assert!(self.check_keyword(ident, p.get() - 1));
            self.emit_error(Analyzer::Error("Use of keyword in identifier"));

            unsafe {
                return &*slice_from_raw_parts(stream.as_ptr(), p.get() - 1);
            }
        }

        return self.parse_ident_long(stream);

    }

    const fn parse_ident_long(&mut self, kw: Stream) -> Stream<'_>{

        let upper = kw.len();
        let mut index = 0usize;
        let mut keyword_len = 0usize;


        while index < upper / 8  {
            //0 offset with loop start
            let (_, _, pos) = Self::advance_stream8_get_idx::<':'>(kw, index * 8);

            //the pos is only relevant on the last slice, if it is not 8 char aligned (by chance)
            //regardless, after that branch fails the loop is over

            if let Some(x) = pos{
                keyword_len += (x.get() - 1);
                break
            }

            index += 1;
            //After that it's the none case, None technically represents 8, thus, we increment by 8
            keyword_len += 8;

        }

        match index{
            0..2 => {}

            2..4 => {
                self.emit_warning(Analyzer::Warning("Identifier length too long"));
            }
            4 => {
                self.emit_error(Analyzer::Error("idk too long ident something something"));
                //TODO temporary, proper handling later

                panic!("identifier length too long");
            }
            //if the code above already this cannot be reached
            //since the index only increments by one
            _ => unsafe {unreachable_unchecked()}
        }

        unsafe {
            return &*slice_from_raw_parts(kw.as_ptr(), keyword_len);
        }
    }

     const fn check_keyword(&mut self, maybe_kw: StreamChunk, pos: usize) -> bool{

         unsafe {
             let trimmed_raw = Self::slice8shorten_at_n_extend_with_char::<'@'>(maybe_kw, pos);
             let trimmed = str::from_utf8_unchecked(&trimmed_raw);
             if self.kw_hash(trimmed, pos).is_some(){
                 self.emit_error(Analyzer::Error("Can use keywords as identifiers"));
                 //TODO temporary, better error handling later
                 panic!("keyword used as ident");
             }
         }

        return true;
    }

    const fn emit_warning(&mut self, warning: Analyzer){}

    const fn emit_error(&mut self, error: Analyzer){}

    //it's not unsafe because it just returns zero which is pretty useless techniclly
    const fn kw_hash_unchecked(&self, of: &str) -> u64{
        let keywords = &self.keywords[of.len()];
        let mut counter = 0;
        while counter < keywords.len() {
            let a = of.as_bytes();
            let hash = keywords.get(counter).hash.to_le_bytes();
            if Self::raw_cmp(a, &hash){
                return u64::from_le_bytes(hash);
            }
            counter += 1;
        }

        return 0;
    }
    //i don't know if this is as fast as i want it
    const fn kw_hash(&self, of: &str, len: usize) -> Option<u64>{
        let keywords = &self.keywords[len & 7];
        let mut counter = 0;
        while counter < keywords.len() {
            let a = of.as_bytes();
            let hash = keywords.get(counter).hash.to_le_bytes();
            if Self::raw_cmp(a, &hash){
                return Some(u64::from_le_bytes(hash));
            }
            counter += 1;
        }

        return None;
    }


    const fn raw_cmp(a: &[u8], b: &[u8]) -> bool{
        let a = u64::from_le_bytes(Self::load8_extend(a));
        let b = u64::from_le_bytes(Self::load8_extend(b));

        return a == b;
    }
    
    const fn is_full(src: &[u8; 8]) -> bool {
        //if index 7 is filled then logically all others must also be filled
        src[7] != b'@'
    }

    const fn load8_extend(src: Stream) -> StreamChunk{
        //64 or @ for easier masking since no used ascii value will be that
        let mut emptied = [64u8; 8];

        //TODO a while loop is probably just as fast if not faster
        let len = if src.len() >= 8 {8} else { src.len() };

        unsafe {
            core::ptr::copy_nonoverlapping(
                src.as_ptr(),
                emptied.as_mut_ptr(),
                len,
            );
        }
        return emptied
    }

    //where N is 8 or 0 it pretty much does nothing... so its stupid to do that?
    //but putting a branch against something like that in a private function is not worth it
    #[inline]
    const fn slice8shorten_static_zx<const N: usize>(src: StreamChunk) -> StreamChunk{
        assert!(N > 0 && N < 8,
                "N should be between 1..8, 0 and 8 just return an empty slice");
        let mask = (1u64 << ((N & 7) << 3)).wrapping_sub(1);
        (u64::from_le_bytes(src) & mask).to_le_bytes()
    }
    #[inline]
    const fn slice8shorten_zx(src: StreamChunk, new_len: usize) -> StreamChunk{
        let mask = (1u64 << ((new_len & 7) << 3)).wrapping_sub(1);
        (u64::from_le_bytes(src) & mask).to_le_bytes()
    }
    const fn slice8shorten_at_n_extend_with_char<const C: char>(src: StreamChunk, new_len: usize) -> StreamChunk{
        let shift = (new_len & 7) << 3;
        let clear = (1u64 << shift).wrapping_sub(1);
        //dynamically make new mask
        //the compiler should usually replace the mask with a constant
        let mask: u64 = Self::repeat_byte(C as u8) << shift;
        ((u64::from_le_bytes(src) & clear) | mask).to_le_bytes()
    }

    const fn slice8_split_and_extend_at_char<const AT: char, const WITH: char>(src: StreamChunk) -> StreamChunk{
        return match Self::char_pos::<AT>(u64::from_le_bytes(src)) {
           pos @ 0..8 => Self::slice8shorten_at_n_extend_with_char::<WITH>(src, pos),
            _ => src
        }
    }

    ///This returns the indecies incremented by 1 due to 0 being the invalid case
    ///As there is no real "range types" just yet, if they get released i can switch to that
    ///So be careful, the real "Some(x)" index is x - 1
    #[inline]
    pub const fn char_pos_checked<const C: char>(string: u64) -> Option<NonZero<usize>>{
        let to_find_mask: u64 = Self::repeat_byte(C as u8);
        let x: u64 = string ^ to_find_mask;
        let y: u64 = (x.overflowing_sub(0x0101010101010101).0) & !x & 0x8080808080808080;

        let pre = (y.trailing_zeros() >> 3) as usize;

        //little trickery
        let is_zero = (pre & 8) >> 3;
        let altered = pre + 1 - (9 * is_zero);
        NonZero::new(altered)

    }

    ///This returns index 0..8, where 8 is invalid
    #[inline]
    pub const fn char_pos<const C: char>(string: u64) -> usize{
        let to_find_mask: u64 = Self::repeat_byte(C as u8);
        let x: u64 = string ^ to_find_mask;
        let y: u64 = (x.overflowing_sub(0x0101010101010101).0) & !x & 0x8080808080808080;

        let pre = (y.trailing_zeros() >> 3) as usize;

        return pre;

    }
    #[inline(always)]
    const fn repeat_byte(b: u8) -> u64 { (b as u64) * (u64::MAX / 255) }


    const unsafe fn add_u8_slice_offset(slice_in: Stream, offset: usize) -> Stream{
        unsafe {
            let new_ptr = &*slice_from_raw_parts(
                slice_in.as_ptr().cast::<u8>().byte_add(offset),
                slice_in.len() - offset,
            );
            return new_ptr;
        }
    }

}