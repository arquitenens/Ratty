
use std::arch::x86_64::_pext_u64;
use std::convert::Infallible;
use std::hint::{black_box, unreachable_unchecked};
use std::mem::{transmute, transmute_copy};
use std::num::{NonZero, NonZeroU128};
use std::ptr::{slice_from_raw_parts, NonNull};
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
    primitive_types: ConstVec<Keyword, 8>,
    remainder: [u8; 8]
}

pub(crate) type Stream<'a> = &'a[u8];
pub(crate) type StreamChunk = [u8; 8];

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug)]
struct Keyword{
    pub(crate) hash: u64,
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
            raw_name: kw
        }
    }

    pub const fn to_u128(self) -> u128 {
        //this direction is safe
        unsafe{transmute(self)}
    }
    pub const unsafe fn from_u128(src: u128) -> Keyword{ transmute(src) }
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
                ConstVec::from_const_array([
                    Keyword::make("_"),
                ]),
                //1 len, probably unused
                ConstVec::from_const_array([
                    Keyword::make("_")
                ]),
                //2 len
                ConstVec::from_const_array([
                    Keyword::make("in"),
                    Keyword::make("or"),
                ]),
                //3 len
                ConstVec::from_const_array([
                    Keyword::make("end"),
                    Keyword::make("dyn"),
                    Keyword::make("try"),
                    Keyword::make("for"),
                    Keyword::make("and"),
                    Keyword::make("not"),
                ]),
                //4 len
                ConstVec::from_const_array([
                    Keyword::make("task"),
                    Keyword::make("func"),
                    Keyword::make("weak"),
                    Keyword::make("wait")
                ]),
                //5 len
                ConstVec::from_const_array([
                    Keyword::make("local"),
                    Keyword::make("block"),
                    Keyword::make("while"),

                ]),
                //6 len
                ConstVec::from_const_array([
                    Keyword::make("struct"),
                    Keyword::make("global"),
                    Keyword::make("strong"),
                    Keyword::make("return"),
                ]),
                //7 len
                ConstVec::from_const_array([Keyword::make("_")])
            ],

            //The star symbol for a pointer is implicitly a primitive type
            //its just not listed
            primitive_types: ConstVec::from_const_array([
                //The stars literally aligned for this layout to work
                //the comments signify the lowest 3 bits in the ascii rep
                //All are 64 bits

                //8 raw bytes, unknown type
                Keyword::make("Pack"),  //000

                Keyword::make("Int"),   //001
                Keyword::make("Bool"),  //010
                Keyword::make("String"),//011
                Keyword::make("Tuple"), //100
                Keyword::make("Uint"),  //101
                Keyword::make("Float"), //110

                //Might not be 64 bits
                Keyword::make("?") //111 Its an unknown type
            ]),

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
            _exclude @ (b"local@@@" | b"global@@" | b"const@@@") => {

                unsafe {
                    let stripped_pos = Self::char_pos::<'@'>(u64::from_le_bytes(maybe_kw));

                    let new = Self::add_u8_slice_offset(raw_lines, stripped_pos + 1);
                    /*
                    let now = Instant::now();
                    for i in 0..100_000_000{
                        let res = self.parse_declaration(new);
                        black_box(res);
                    }

                    let elapsed = now.elapsed();
                    println!("parsed declaration: {:?}", elapsed);
                    */
                    let now = Instant::now();
                    for i in 0..100_000_000{
                        let res = self.parse_declaration(new);
                        black_box(res);
                    }
                    let elapsed = now.elapsed();
                    println!("parsed declaration: {:?}", elapsed);

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

    fn parse_declaration(&mut self, stream: Stream){
        let ident = self.parse_ident(stream);

        //the new offset is + 2 due to the colon being skipped and the mandatory space
        //x: Int -> Int..
        let ident_offset_stream = unsafe {Self::add_u8_slice_offset(stream, ident.len() + 2)};

        let ty = self.parse_type(ident_offset_stream);
        black_box(ty);

    }

    const fn advance_stream8_get_idx<const C: char>(stream: Stream<'_>, added_offset: usize) -> (Stream<'_>, StreamChunk, Option<NonZero<usize>>){
        let offset_kw_stream = unsafe {Self::add_u8_slice_offset(stream, added_offset)};
        let ident = Self::load8_extend(offset_kw_stream);
        let maybe_valid_pos = Self::char_pos_checked::<C>(u64::from_le_bytes(ident));

        return (offset_kw_stream, ident, maybe_valid_pos);
    }


    const fn parse_ident(&mut self, kw: Stream) -> &str{

        let (stream, ident, pos) = Self::advance_stream8_get_idx::<':'>(kw, 0);
        if let Some(p) = pos{
            //Fast path
            //Quick dirty check, better error handling later
            assert!(self.check_keyword(ident, p.get() - 1));
            self.emit_error(Analyzer::Error("Use of keyword in identifier"));

            unsafe {
                return str::from_utf8_unchecked(&*slice_from_raw_parts(stream.as_ptr(), p.get() - 1));
            }
        }

        return self.parse_ident_long(stream);

    }

    const fn parse_ident_long(&mut self, kw: Stream) -> &str{

        let upper = kw.len();
        let mut index = 0usize;
        let mut keyword_len = 0usize;

        while index < upper / 8  {
            //0 offset with loop start
            let (_, _, pos) = Self::advance_stream8_get_idx::<':'>(kw, index * 8);

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
            return str::from_utf8_unchecked(&*slice_from_raw_parts(kw.as_ptr(), keyword_len));
        }
    }



    const fn parse_type(&mut self, ty_stream: Stream) -> u8{
        let ty = Self::slice8_split_and_extend_at_char::<' ', '@'>(Self::load8_extend(ty_stream));
        //The returned types directly return the correct representation for the IR
        if self.check_primitive(ty).is_some(){
            //This code is technically already inside the "check_primitive" function
            //But since i didn't want to change the layout of KeyWord this will do
            return ((u64::from_le_bytes(ty) & 0x7) + 'A' as u64) as u8;
        }
        //The type is not primitive, thus marked as opaque for now
        return '?' as u8;
    }

    //Im such a clever cookie

    const fn check_primitive(&mut self, maybe_type: StreamChunk) -> Option<NonZeroU128>{
        let orig = u64::from_le_bytes(maybe_type);
        //im only matching the last 3 bits
        let try_match = orig & 0x7;
        //index will never be greater than 7 so the unchecked index is fine
        let maybe_type = unsafe {self.primitive_types.get_unchecked(try_match as usize)};

        let is_match = orig == maybe_type.hash;

        //TODO wrapping sub maybe optimization
        //let mask = 0u128.wrapping_sub(is_match);
        //let result = ty & mask;

        return unsafe {
            //Keyword struct has proper alignment
            let ty: u128 = transmute(*maybe_type);
            //the match is either 1 or 0, in the not matching case it multiplies by zero thus, turning the value into None
            //else it keeps the original value
            NonZeroU128::new(ty * (is_match as u128))
        }
    }

     const fn check_keyword(&mut self, maybe_kw: StreamChunk, pos: usize) -> bool{

         unsafe {
             let trimmed_raw = Self::slice8shorten_at_n_extend_with_char::<'@'>(maybe_kw, pos);
             let trimmed = str::from_utf8_unchecked(&trimmed_raw);
             if self.kw_hash(trimmed, pos).is_some(){
                 self.emit_error(Analyzer::Error("Cant use keywords as identifiers"));
                 //TODO temporary, better error handling later
                 panic!("keyword used as ident");
             }
         }

        return true;
    }

    const fn emit_warning(&mut self, warning: Analyzer){}

    const fn emit_error(&mut self, error: Analyzer){}

    //i don't know if this is as fast as i want it, once i have thought of all the keywords
    //i can try making some static hash or constant
    const fn kw_hash(&self, of: &str, len: usize) -> Option<u64>{
        let keywords = &self.keywords[len & 7];
        let mut counter = 0;
        while counter < keywords.len() {
            let a = of.as_bytes();
            let hash = unsafe {keywords.get_unchecked(counter).hash.to_le_bytes()};
            if Self::raw_cmp(a, &hash){
                return Some(u64::from_le_bytes(hash));
            }
            counter += 1;
        }

        return None;
    }

    #[inline(always)]
    const fn raw_cmp(a: &[u8], b: &[u8]) -> bool{
        let a = u64::from_le_bytes(Self::load8_extend(a));
        let b = u64::from_le_bytes(Self::load8_extend(b));

        return a == b;
    }
    #[inline(always)]
    const fn is_full(src: &[u8; 8]) -> bool {
        //if index 7 is filled then logically all others must also be filled
        src[7] != b'@'
    }
    #[inline(always)]
    const fn load8_extend(src: Stream) -> StreamChunk {
        let mut emptied = [64u8; 8];
        let len = if src.len() < 8 { src.len() } else { 8 };

        //LLVM just chocked on the while loop for whatever reason
        //It was ruining the performance
        if len > 0 { emptied[0] = src[0]; }
        if len > 1 { emptied[1] = src[1]; }
        if len > 2 { emptied[2] = src[2]; }
        if len > 3 { emptied[3] = src[3]; }
        if len > 4 { emptied[4] = src[4]; }
        if len > 5 { emptied[5] = src[5]; }
        if len > 6 { emptied[6] = src[6]; }
        if len > 7 { emptied[7] = src[7]; }

        return emptied
    }

    //where N is 8 or 0 it pretty much does nothing... so its stupid to do that?
    //but putting a branch against something like that in a private function is not worth it
    #[inline(always)]
    const fn slice8shorten_static_zx<const N: usize>(src: StreamChunk) -> StreamChunk{
        assert!(N > 0 && N < 8,
                "N should be between 1..8, 0 and 8 just return an empty slice");
        let mask = (1u64 << ((N & 7) << 3)).wrapping_sub(1);
        (u64::from_le_bytes(src) & mask).to_le_bytes()
    }
    #[inline(always)]
    const fn slice8shorten_zx(src: StreamChunk, new_len: usize) -> StreamChunk{
        let mask = (1u64 << ((new_len & 7) << 3)).wrapping_sub(1);
        (u64::from_le_bytes(src) & mask).to_le_bytes()
    }

    #[inline(always)]
    const fn slice8shorten_at_n_extend_with_char<const C: char>(src: StreamChunk, new_len: usize) -> StreamChunk{
        let shift = (new_len & 7) << 3;
        let clear = (1u64 << shift).wrapping_sub(1);
        //dynamically make new mask
        //the compiler should usually replace the mask with a constant
        let mask: u64 = Self::repeat_byte(C as u8) << shift;
        ((u64::from_le_bytes(src) & clear) | mask).to_le_bytes()
    }

    #[inline(always)]
    const fn slice8_split_and_extend_at_char<const AT: char, const WITH: char>(src: StreamChunk) -> StreamChunk{
        return match Self::char_pos::<AT>(u64::from_le_bytes(src)) {
           pos @ 0..8 => Self::slice8shorten_at_n_extend_with_char::<WITH>(src, pos),
            _ => src
        }
    }

    ///This returns the indecies incremented by 1 due to 0 being the invalid case
    ///As there is no real "range types" just yet, if they get released i can switch to that
    ///So be careful, the real "Some(x)" index is x - 1
    #[inline(always)]
    pub const fn char_pos_checked<const C: char>(string: u64) -> Option<NonZero<usize>>{
        let to_find_mask: u64 = Self::repeat_byte(C as u8);
        let x: u64 = string ^ to_find_mask;
        let y: u64 = (x.overflowing_sub(0x0101010101010101).0) & !x & 0x8080808080808080;

        let pre = (y.trailing_zeros() >> 3) as usize;

        //shift the 8 to position 0
        //must be logical shift
        let is_zero = pre >> 3;
        let altered = pre + 1 - (9 * is_zero);
        //the new constructor already does what i want.
        //i want a None when the value is zero which it just does with a transmute
        NonZero::new(altered)

    }

    ///This returns index 0..8, where 8 is invalid
    #[inline(always)]
    pub const fn char_pos<const C: char>(string: u64) -> usize{
        let to_find_mask: u64 = Self::repeat_byte(C as u8);
        let x: u64 = string ^ to_find_mask;
        let y: u64 = (x.overflowing_sub(0x0101010101010101).0) & !x & 0x8080808080808080;

        let pre = (y.trailing_zeros() >> 3) as usize;

        return pre;

    }

    #[inline(always)]
    const fn repeat_byte(b: u8) -> u64 { (b as u64) * 0x0101010101010101 }

    #[inline(always)]
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