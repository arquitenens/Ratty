use std::intrinsics::transmute;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::num::NonZero;
use std::ptr;
use std::ptr::slice_from_raw_parts;
use crate::lexer::{Stream, StreamChunk};

//A helper struct because I couldn't find or bothered to find crate that has both const initialization
//and const indexing/manipulation
#[derive(Debug)]
pub(crate) struct ConstVec<T, const N: usize> {
    inner: [MaybeUninit<T>; N],
    len: usize,
}
impl<T, const N: usize> ConstVec<T, N> {
    pub(crate) const fn from_const_array<const M: usize>(inner: [T; M]) -> Self where T: Copy{
        //just copied this from heapless::Vec, though its also quite straightforward and not many other ways of doing it
        let mut emptied = [MaybeUninit::zeroed(); N];

        let no_drop = ManuallyDrop::new(inner);

        let src_ptr = ptr::from_ref(&no_drop).cast();

        let dst_ptr = ptr::from_mut(&mut emptied) as *mut [T; M];

        unsafe {
            ptr::copy_nonoverlapping(src_ptr, dst_ptr, M)
        }

        Self { inner: emptied, len: M }
    }
    #[inline(always)]
    pub(crate) const unsafe fn get_unchecked_mut(&mut self, idx: usize) -> &mut T{
        unsafe {self.inner[idx].assume_init_mut()}
    }
    #[inline(always)]
    pub(crate) const unsafe fn get_unchecked(&self, idx: usize) -> &T{
        unsafe {self.inner[idx].assume_init_ref()}
    }
    #[inline(always)]
    pub(crate) const fn get_mut(&mut self, idx: usize) -> Option<&mut T> where T: Sized{
        unsafe {
            //The uninit type is zeroed so something un-initialized will turn into None
            if idx < self.len {
                return transmute(self.get_unchecked_mut(idx))
            }
            return None;
        }
    }
    #[inline(always)]
    pub(crate) const fn get(&self, idx: usize) -> Option<&T> where T: Sized{
        unsafe {
            //The uninit type is zeroed so something un-initialized will turn into None
            if idx < self.len {
                return transmute(self.get_unchecked(idx))
            }
            return None;
        }
    }

    pub(crate) const fn len(&self) -> usize {
        self.len
    }
}

pub(crate) struct Helper;

impl Helper{
    #[inline(always)]
    pub(crate) const fn raw_cmp(a: &[u8], b: &[u8]) -> bool{
        let a = u64::from_le_bytes(Self::load8_extend(a));
        let b = u64::from_le_bytes(Self::load8_extend(b));

        return a == b;
    }
    #[inline(always)]
    pub(crate) const fn is_full(src: &[u8; 8]) -> bool {
        //if index 7 is filled then logically all others must also be filled
        src[7] != b'@'
    }

    #[inline]
    pub(crate) const fn load8_extend(src: Stream) -> StreamChunk {
        let mut emptied = [64u8; 8];
        //the compiler didn't seem to catch up on that so well fast path
        //it now produces "mov rax, qword ptr [rdi]" which is great! i didn't even have to nudge it
        if src.len() >= 8{
            unsafe {
                ptr::copy_nonoverlapping(src.as_ptr(), emptied.as_mut_ptr(), 8);
                return emptied;
            }
        }
        //LLVM just chocked on the while loop for whatever reason
        //It was ruining the performance
        let len = src.len();

        //replace with raw pointer writes if the bounds checking is bad
        unsafe {
            if len > 0 { emptied[0] = src[0]; }
            if len > 1 { emptied[1] = src[1]; }
            if len > 2 { emptied[2] = src[2]; }
            if len > 3 { emptied[3] = src[3]; }
            if len > 4 { emptied[4] = src[4]; }
            if len > 5 { emptied[5] = src[5]; }
            if len > 6 { emptied[6] = src[6]; }
            if len > 7 { emptied[7] = src[7]; }
        }

        return emptied
    }

    //where N is 8 or 0 it pretty much does nothing... so its stupid to do that?
    //but putting a branch against something like that in a private function is not worth it
    #[inline(always)]
    pub(crate) const fn slice8shorten_static_zx<const N: usize>(src: StreamChunk) -> StreamChunk{
        assert!(N > 0 && N < 8,
                "N should be between 1..8, 0 and 8 just return an empty slice");
        let mask = (1u64 << ((N & 7) << 3)).wrapping_sub(1);
        (u64::from_le_bytes(src) & mask).to_le_bytes()
    }
    #[inline(always)]
    pub(crate) const fn slice8shorten_zx(src: StreamChunk, new_len: usize) -> StreamChunk{
        let mask = (1u64 << ((new_len & 7) << 3)).wrapping_sub(1);
        (u64::from_le_bytes(src) & mask).to_le_bytes()
    }

    #[inline(always)]
    pub(crate) const fn slice8shorten_at_n_extend_with_char<const C: char>(src: StreamChunk, new_len: usize) -> StreamChunk{
        let shift = (new_len & 7) << 3;

        //This might be faster?
        const CLEAR_MASKS: [u64; 8] = [
            0x0000000000000000,
            0x00000000000000FF,
            0x000000000000FFFF,
            0x0000000000FFFFFF,
            0x00000000FFFFFFFF,
            0x000000FFFFFFFFFF,
            0x0000FFFFFFFFFFFF,
            0x00FFFFFFFFFFFFFF,
        ];

        let clear = CLEAR_MASKS[new_len & 7];
        //dynamically make new mask
        //the compiler should usually replace the mask with a constant
        let mask: u64 = Self::repeat_char_static::<C>() << shift;
        ((u64::from_le_bytes(src) & clear) | mask).to_le_bytes()
    }

    #[inline(always)]
    pub(crate) const fn slice8shorten_extend_with_at(src: StreamChunk, new_len: usize) -> StreamChunk{

        //This might be faster?
        const CLEAR_MASKS: [u64; 8] = [
            0x0000000000000000,
            0x00000000000000FF,
            0x000000000000FFFF,
            0x0000000000FFFFFF,
            0x00000000FFFFFFFF,
            0x000000FFFFFFFFFF,
            0x0000FFFFFFFFFFFF,
            0x00FFFFFFFFFFFFFF,
        ];

        let clear = CLEAR_MASKS[new_len & 7];

        //i don't have to shift the 0x40 mask because every accepted ascii char is above 64
        //thus it will naturally have said bit set and the OR will do nothing
        ((u64::from_le_bytes(src) & clear) | 0x4040404040404040).to_le_bytes()
    }

    #[inline(always)]
    pub(crate) const fn slice8_split_and_extend_at_char<const AT: char, const WITH: char>(src: StreamChunk) -> StreamChunk{
        return match Self::char_pos::<AT>(u64::from_le_bytes(src)) {
            pos @ 0..8 => Self::slice8shorten_at_n_extend_with_char::<WITH>(src, pos),
            _ => src
        }
    }

    #[inline(always)]
    //returns a slice full of '@' when the char is not contained
    pub(crate) const fn slice8_split_and_extend_with_at_sign<const AT: char>(src: StreamChunk) -> StreamChunk{
        return Self::slice8shorten_extend_with_at(src, Self::char_pos::<AT>(u64::from_le_bytes(src)))
    }

    ///This returns the indecies incremented by 1 due to 0 being the invalid case
    ///As there is no real "range types" just yet, if they get released i can switch to that
    ///So be careful, the real "Some(x)" index is x - 1
    #[inline(always)]
    pub(crate) const fn char_pos_checked<const C: char>(string: u64) -> Option<NonZero<usize>>{
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
    pub(crate) const fn char_pos<const C: char>(string: u64) -> usize{
        let to_find_mask: u64 = Self::repeat_byte(C as u8);
        let x: u64 = string ^ to_find_mask;
        let y: u64 = (x.overflowing_sub(0x0101010101010101).0) & !x & 0x8080808080808080;

        let pre = (y.trailing_zeros() >> 3) as usize;

        return pre;

    }

    #[inline(always)]
    pub(crate) const fn repeat_byte(b: u8) -> u64 { (b as u64) * 0x0101010101010101 }
    #[inline(always)]
    pub(crate) const fn repeat_char_static<const B: char>() -> u64 { u64::from_le_bytes([B as u8; 8]) }


    #[inline(always)]
    pub(crate) const unsafe fn add_u8_slice_offset(slice_in: Stream, offset: usize) -> Stream{
        unsafe {
            let new_ptr = &*slice_from_raw_parts(
                slice_in.as_ptr().cast::<u8>().byte_add(offset),
                slice_in.len() - offset,
            );
            return new_ptr;
        }
    }
}