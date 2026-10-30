use std::arch::asm;
use std::hint::{black_box, unreachable_unchecked};
use crate::instructions::IR;

pub struct RawParser{
    source: String,
}

impl RawParser {
    pub fn new(source: String) -> RawParser {
        RawParser{
            source: source,
        }
    }

    fn parse_preprocessor(&mut self) -> &mut [u8]{
        const ENTRY: &str = "START: #";
        if self.source.contains(ENTRY){
            todo!()
        }
        return unsafe {self.source.as_bytes_mut()}
    }

    pub fn parse_after_entry(&mut self) -> Vec<IR>{
        let (chunks, remainder) = self.parse_preprocessor().as_chunks::<8>();
        assert!(remainder.is_empty());
        assert_eq!(chunks.len() % 2, 0);

        let mut collect: Vec<IR> = Vec::new();

        for (i, c) in chunks.iter().enumerate().step_by(2) {

            let discriminant = c[6] - 'A' as u8;
            let instr = unsafe {IR::from_raw(discriminant)};
            collect.push(instr);
            const REG_SEP: char = '&';
            let get_regs_cnt = string_pos(&chunks[i + 1], REG_SEP).ilog2() as usize;
            let regs = &chunks[i + 1];

            for i in (0..(get_regs_cnt << 1)).step_by(2){
                let r_upper = (regs[i] - ('A' as u8)) as usize;
                let r_lower = (regs[i + 1] - ('A' as u8)) as usize;
                let r = r_upper * 26 + r_lower;
            }
        }
        return collect;
    }
}

fn string_pos(string: &[u8; 8], c: char) -> usize{
    let to_find_mask: u64 = repeat_byte(c as u8) as u64;
    let x: u64 = u64::from_ne_bytes(*string) ^ to_find_mask;
    let y: u64 = (x.overflowing_sub(0x0101010101010101).0) & !x & 0x8080808080808080;
    (y.trailing_zeros() >> 3) as usize
}

#[inline(always)]
fn repeat_byte(b: u8) -> usize { (b as usize) * (usize::MAX / 255) }