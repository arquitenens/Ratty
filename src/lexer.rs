use std::arch;
use std::arch::x86_64::_pext_u64;
use std::mem::transmute_copy;
use crate::lexer::KeywordSlot::Occupied;
use crate::util::ConstVec;
//TODO better approach than nested vec



#[derive(Copy, Clone)]
enum KeywordSlot{
    Occupied(Keyword),
    Empty
}




pub struct Lexer {
    //no keyword is longer than 8 thats why 8 static array len thing
    keywords: [ConstVec<Keyword, 16>; 8],
    remainder: [u8; 8]
}



#[repr(align(8))]
#[derive(Copy,Clone)]
struct Keyword{
    pub(crate) hash: u64,
    //raw name for error reporting
    //it cant really be a static str slice since it might change
    pub(crate) raw_name: [u8; 8],
}

impl Keyword{
    pub fn make(s: &str) -> Keyword{
        //32 for easy ascii stripping
        let mut kw = [32u8; 8];
        //64 easier for masking
        let mut bytes = [64u8; 8];
        bytes[..s.len()].copy_from_slice(s.as_bytes());
        kw[..s.len()].copy_from_slice(s.as_bytes());
        return Keyword{
            hash: u64::from_ne_bytes(bytes),
            raw_name: kw
        }
    }
}

impl<'a> Lexer {
    pub fn new() -> Lexer {
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
                ConstVec::from_const_array([Keyword::make("end")]),
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
        assert!(line.is_ascii(), "Only ascii is allowed");
        //after this unchecked conversions should be fine

        let emptied = [0u8; 8];

        let (chunks, rem) = line.as_bytes().as_chunks::<8>();



        let kw = Self::load8_extend(&chunks[0]);
        println!("key {:?}", str::from_utf8(&kw));

        let x = match "" {
            exclude @ ("local" | "global" | "const") if self.check_keyword("") => {
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

    //the argument comes from chunks which are 8 in size
    const fn check_keyword(&self, maybe_key: &str) -> bool{
        self.kw_hash(maybe_key).is_some()
    }

    //i don't know if this is as fast as i want it
    const fn kw_hash(&self, of: &str) -> Option<u64>{
        let keywords = &self.keywords[of.len()];
        let mut counter = 0;
        while counter < keywords.len() {
            let a = of.as_bytes();
            let hash = keywords.get(counter).hash.to_le_bytes();
            if Self::raw_cmp(a, &hash){
                return Some(u64::from_ne_bytes(hash));
            }
            counter += 1;
        }

        return None;
    }

    //it's not unsafe because it just returns zero which is pretty useless techniclly
    const fn kw_hash_unchecked(&self, of: &str) -> u64{
        let keywords = &self.keywords[of.len()];
        let mut counter = 0;
        while counter < keywords.len() {
            let a = of.as_bytes();
            let hash = keywords.get(counter).hash.to_le_bytes();
            if Self::raw_cmp(a, &hash){
                return u64::from_ne_bytes(hash);
            }
            counter += 1;
        }

        return 0;
    }

    const fn raw_cmp(a: &[u8], b: &[u8]) -> bool{
        let a = u64::from_ne_bytes(Self::load8_extend(a));
        let b = u64::from_ne_bytes(Self::load8_extend(b));

        return a == b;
    }

    const fn is_full(src: &[u8; 8]) -> bool {
        //if index 7 is filled then logically all others must also be filled
        src[7] != 64
    }

    const fn load8_extend(src: &[u8]) -> [u8; 8]{
        //64 or @ for easier masking since no used ascii value will be that
        let mut emptied = [64u8; 8];

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
    const fn slice8shorten_static_zx<const N: usize>(src: [u8; 8]) -> [u8; 8]{
        let mask = (1u64 << ((N & 7) << 3)).wrapping_sub(1);
        (u64::from_ne_bytes(src) & mask).to_ne_bytes()
    }
    #[inline]
    const fn slice8shorten_zx(src: [u8; 8], new_len: usize) -> [u8; 8]{
        let mask = (1u64 << ((new_len & 7) << 3)).wrapping_sub(1);
        (u64::from_ne_bytes(src) & mask).to_ne_bytes()
    }
    const fn slice8shorten_ex_with_char<const C: char>(src: [u8; 8], new_len: usize) -> [u8; 8]{
        let shift = (new_len & 7) << 3;
        let clear = (1u64 << shift).wrapping_sub(1);
        //dynamically make new mask
        //the compiler should usually replace the mask with a constant
        let mask: u64 = Self::repeat_byte(C as u8) << shift;
        ((u64::from_ne_bytes(src) & clear) | mask).to_ne_bytes()
    }

    const fn slice8_split_and_extend_at_char<const AT: char, const WITH: char>(src: [u8; 8]) -> [u8; 8]{
        return match Self::char_pos::<AT>(u64::from_ne_bytes(src))  {
            p @ 0..8 => Self::slice8shorten_ex_with_char::<WITH>(src, p),
            _ => src
        }
    }
    #[inline]
    pub const fn char_pos<const C: char>(string: u64) -> usize{
        let to_find_mask: u64 = Self::repeat_byte(C as u8);
        let x: u64 = string ^ to_find_mask;
        let y: u64 = (x.overflowing_sub(0x0101010101010101).0) & !x & 0x8080808080808080;
        (y.trailing_zeros() >> 3) as usize
    }
    #[inline(always)]
    pub const fn repeat_byte(b: u8) -> u64 { (b as u64) * (u64::MAX / 255) }

}