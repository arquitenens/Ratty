
use std::arch::x86_64::_pext_u64;
use std::convert::Infallible;
use std::hash::Hash;
use std::hint::{black_box, unreachable_unchecked};
use std::mem::{transmute, transmute_copy};
use std::num::{NonZero, NonZeroU128, NonZeroU64};
use std::ptr;
use std::ptr::{slice_from_raw_parts, NonNull};
use std::time::Instant;
use crate::util::{ConstVec, Helper};
//TODO better approach than nested vec



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


static KW_NAMES: [&'static str; 1] = [""];

//I decided against storing the raw_name for the given hash inside the struct
//the extra 8 bytes are a pretty big performance cost
#[derive(Copy, Clone, Debug)]
pub(crate) struct Keyword{
    pub(crate) hash: u64,
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
        }
    }

    pub const fn to_u64(self) -> u64 {
        //this direction is safe
        unsafe{transmute(self)}
    }
    pub const unsafe fn from_u64(src: u64) -> Keyword{ transmute(src) }
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
                ConstVec::from_const_array([]),
                //1 len, probably unused
                ConstVec::from_const_array([]),
                //2 len
                ConstVec::from_const_array([
                    Keyword::make("in"), //01101111
                    Keyword::make("or"), //01101001
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
                ConstVec::from_const_array([])
            ],

            //The star symbol for a pointer is implicitly a primitive type
            //its just not listed
            primitive_types: ConstVec::from_const_array([
                //The stars literally aligned for this layout to work
                //the comments signify the lowest 3 bits in the ascii rep
                //All are 64 bits
                //TODO maybe expand to 4 bits so i can include something like Nil or None
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

        let raw_lines: Stream = line.as_bytes();
        //after this unchecked conversions should be fine

        let emptied: StreamChunk = [0u8; 8];

        let kw = Helper::load8_extend(raw_lines);

        let maybe_kw = Helper::slice8_split_and_extend_with_at_sign::<' '>(kw);

        let x = match &maybe_kw {
            //I don't want to do it like this but rust seems a bit stupid with its const matching
            _exclude @ (b"local@@@" | b"global@@" | b"const@@@") => {

                unsafe {
                    let stripped_pos = Helper::char_pos::<'@'>(u64::from_le_bytes(maybe_kw));

                    let new = Helper::add_u8_slice_offset(raw_lines, stripped_pos + 1);

                    let now = Instant::now();

                    //for i in 0..100_000_000 {
                    //    let res = self.parse_declaration(new);
                    //}
                    //let elapsed = now.elapsed();
                    //println!("Elapsed: {:.2?}", elapsed);


                    let res = self.parse_declaration(new);
                }

            },
            _ => {

                //println!("unrecognized keyword: {:?}", first);
                return unsafe{transmute_copy(&[emptied, emptied])}
            }
        };

        //im not using the return value any time soon nor am i constructing it,
        //its just so the code compiles
        return unsafe {transmute_copy(&[emptied, emptied])}
    }

    pub(crate) fn parse_declaration(&mut self, stream: Stream){
        let ident = self.parse_ident(stream);
        //print!("ident: {:?}", ident);
        //the new offset is + 2 due to the colon being skipped and the mandatory space
        //x: Int -> Int..
        let ident_offset_stream = unsafe {Helper::add_u8_slice_offset(stream, ident.len() + 2)};

        let ty = self.parse_type(ident_offset_stream);

        let val_offset_stream = unsafe {
            //+ 3 because ..." = val", we dont need the rest
             Helper::add_u8_slice_offset(ident_offset_stream, Self::type_length(ty) + 3)
         };

        let val = black_box(self.parse_expr(ty, val_offset_stream));
        black_box(val);

    }
    #[inline]
    pub(crate) const fn type_length(ty: u8) -> usize{
        //Technically the lowest 3 bits
        match ty {
            b'?' => 1, //?
            b'B' => 3, //Int
            b'G' => 5, //Float
            b'A' => 4, //Pack
            b'C' => 4, //Bool
            b'D' => 6, //String
            b'E' => 5, //Tuple
            b'F' => 4, //Uint
            _ => unreachable!(),
        }
    }

    const fn advance_stream8_get_idx<const C: char>(stream: Stream<'_>, added_offset: usize) -> (Stream<'_>, StreamChunk, Option<NonZero<usize>>){
        let offset_kw_stream = unsafe {Helper::add_u8_slice_offset(stream, added_offset)};
        let ident = Helper::load8_extend(offset_kw_stream);
        let maybe_valid_pos = Helper::char_pos_checked::<C>(u64::from_le_bytes(ident));

        return (offset_kw_stream, ident, maybe_valid_pos);
    }

    const fn parse_ident(&mut self, kw: Stream) -> &str{

        let (stream, ident, pos) = Self::advance_stream8_get_idx::<':'>(kw, 0);
        if let Some(p) = pos{
            //Fast path
            //Quick dirty check, better error handling later
            assert!(!self.check_keyword(ident, p.get() - 1));
            self.emit_error(Analyzer::Error("Use of keyword in identifier"));

            unsafe {
                return str::from_utf8_unchecked(&*slice_from_raw_parts(stream.as_ptr(), p.get() - 1));
            }
        }


        return self.parse_ident_long(stream);

    }

    const fn parse_ident_long(&mut self, kw: Stream) -> &str{
        //Hand unrolled, not pretty i know
        let mut keyword_len = 0usize;

        //first
        let (_, _, pos) = Self::advance_stream8_get_idx::<':'>(kw, 0);
        if let Some(x) = pos{
            keyword_len += (x.get() - 1);
            return unsafe {
                str::from_utf8_unchecked(&*slice_from_raw_parts(kw.as_ptr(), keyword_len))
            }
        }
        //Nothing found so inc by 8
        keyword_len += 8;
        //second
        let (_, _, pos) = Self::advance_stream8_get_idx::<':'>(kw, 8);
        if let Some(x) = pos{
            keyword_len += (x.get() - 1);
            return unsafe {
                str::from_utf8_unchecked(&*slice_from_raw_parts(kw.as_ptr(), keyword_len))
            }
        }

        keyword_len += 8;

        self.emit_warning(Analyzer::Warning("Identifier length too long"));
        //third
        let (_, _, pos) = Self::advance_stream8_get_idx::<':'>(kw, 16);
        if let Some(x) = pos{
            keyword_len += (x.get() - 1);
            return unsafe {
                str::from_utf8_unchecked(&*slice_from_raw_parts(kw.as_ptr(), keyword_len))
            }
        }
        keyword_len += 8;

        self.emit_warning(Analyzer::Warning("Identifier length too long"));

        //fourth
        let (_, _, pos) = Self::advance_stream8_get_idx::<':'>(kw, 24);
        if let Some(x) = pos{
            keyword_len += (x.get() - 1);
            return unsafe {
                str::from_utf8_unchecked(&*slice_from_raw_parts(kw.as_ptr(), keyword_len))
            }
        }
        keyword_len += 8;

        self.emit_error(Analyzer::Error("idk too long ident something something"));
        panic!("identifier length too long");
    }



    #[inline]
    pub(crate) const fn parse_type(&mut self, ty_stream: Stream) -> u8{
        let ty = Helper::slice8_split_and_extend_with_at_sign::<' '>(Helper::load8_extend(ty_stream));
        //The returned types directly return the correct representation for the IR
        if self.check_primitive(ty).is_some(){
            //No need to conver to u64, i only need to check the first byte anyway
            return ((ty[0] & 0x7) + 'A' as u8);
        }
        //The type is not primitive, thus marked as opaque for now
        return '?' as u8;
    }

    pub(crate) fn parse_expr(&mut self, ty: u8, expr_stream: Stream<'a>) -> Stream<'a>{
        return expr_stream;
    }

    //Im such a clever cookie
    #[inline]
    const fn check_primitive(&mut self, maybe_type: StreamChunk) -> Option<NonZeroU64>{
        let orig = u64::from_le_bytes(maybe_type);
        //im only matching the last 3 bits
        let try_match = orig & 0x7;
        //index will never be greater than 7 so the unchecked index is fine
        let maybe_type = unsafe {self.primitive_types.get_unchecked(try_match as usize)};

        let is_match = (orig == maybe_type.hash) as u64;

        return unsafe {
            //Keyword struct has proper alignment
            let ty: u64 = transmute(*maybe_type);
            //this is probably faster than the mul by 0 or 1
            NonZeroU64::new(ty & 0u64.wrapping_sub(is_match))
        }
    }
    #[inline]
     const fn check_keyword(&mut self, maybe_kw: StreamChunk, pos: usize) -> bool{

         let trimmed_raw = Helper::slice8shorten_at_n_extend_with_char::<'@'>(maybe_kw, pos);

        return self.kw_hash(trimmed_raw, pos).is_some();
    }
    #[inline]
    const fn emit_warning(&mut self, warning: Analyzer){}
    #[inline]
    const fn emit_error(&mut self, error: Analyzer){}

    //i don't know if this is as fast as i want it, once i have thought of all the keywords
    //i can try making some static hash or constant
    pub const fn kw_hash(&self, of: StreamChunk, len: usize) -> Option<u64> {
        let raw = u64::from_le_bytes(of);
        let mod_len = len & 7;

        //These masks have the bits set in the position of the
        //ascii lowest 5 bits value, e.g 'i' is "01101001" the lowest 5 bits are "01001" which is 9
        //thus the mask[mod_len] has a bit set at 9
        const LOOKUP: [u64; 8] = [
            0,
            0,
            0x00008200,
            0x00104072,
            0x00084202,
            0x00801004,
            0x0004400A,
            0,
        ];

        //Extract the lowest 5 bits, they are the only relevant things
        //this can result in any index within that range
        let raw_hash = raw & 31;

        let bit = 1u64 << raw_hash;

        //Branchless version is slower
        if LOOKUP[mod_len] & bit == 0 {
            return None;
        }else {
            let index = (LOOKUP[mod_len] & (bit - 1)).count_ones() as usize;
            return unsafe {
                Some(self.keywords[mod_len].get_unchecked(index).hash)
            }
        }
    }



}