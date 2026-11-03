use std::arch::asm;
use std::arch::x86_64::_pext_u64;
use std::hint::{black_box, unreachable_unchecked};
use std::mem::MaybeUninit;
use std::ptr;
use crate::instructions::{IROps, RType, Register, IR};

pub struct RawParser{
    source: String,
}

const fn create_register_table() -> [Register; 512] {
    let mut registers = [Register::None; 512];

    let mut i = 0;

    while i < 256 {
        registers[i] = Register::General(i as u8);
        i += 1;
    }

    while i < 384 {
        registers[i] = Register::Constant((i as u16 - 256) as u8);
        i += 1;
    }

    while i < 511 {
        registers[i] = Register::Imm((i as u16 - 384) as u8);
        i += 1;
    }

    registers
}

const RAW_REGISTERS: [Register; 512] = create_register_table();

impl RawParser {
    pub fn new(source: String) -> RawParser {
        RawParser{
            source: source,
        }
    }

    fn parse_preprocessor(&mut self) -> &[u8]{
        const ENTRY: &str = "START: #";
        if self.source.contains(ENTRY){
            todo!()
        }
        return self.source.as_bytes()
    }

    pub fn parse_after_entry(&mut self) -> Vec<IR>{
        let (chunks, remainder) = self.parse_preprocessor().as_chunks::<8>();

        assert!(remainder.is_empty());
        assert_eq!(chunks.len() % 2, 0);

        let mut collect: Vec<IR> = Vec::with_capacity(chunks.len() / 2);

        for i in unsafe {chunks.as_chunks_unchecked::<2>()} {
            let c = i[0];
            let regs = i[1];

            let instr = unsafe {IROps::from_raw(c[6])};
            //because its a pow 2 being 64 in ascii owowowow :33
            const REG_SEP: char = '@';
            let get_regs_cnt = string_pos(u64::from_le_bytes(regs), REG_SEP).ilog2() as usize;

            let raw_reg = u64::from_le_bytes(regs);

            let ty = unsafe {RType::from_raw(regs[6])};


            //Filter out the ampersand, newline and the type since i already extracted it
            //leaving me with only the registers, they might have different numbers than their ascii counterpart but that's not important

            let maybe_regs0 = raw_reg & !0xAAFFFFFFFFFF4040;
            let maybe_regs1 = (raw_reg & !0xAAFFFFFF4040FFFF) >> 16;
            let maybe_regs2 = (raw_reg & !0xAAFF4040FFFFFFFF) >> 32;

            //Using 3 branches otherwise is boooooo!!!
            let r0 = RAW_REGISTERS[RawParser::pair_index(maybe_regs0)];

            //TODO check the asm/what llvm produces, if its a jump table or cmov
            //and then try to coax it if need
            #[cfg(debug_assertions)]
            asm_marker();

            let registers = match get_regs_cnt {
                1 => [r0, Register::None, Register::None],

                2 => [
                    r0,
                    RAW_REGISTERS[RawParser::pair_index(maybe_regs1)],
                    Register::None,
                ],
                3 => [
                    r0,
                    RAW_REGISTERS[RawParser::pair_index(maybe_regs1)],
                    RAW_REGISTERS[RawParser::pair_index(maybe_regs2)],
                ],

                4 => [
                    //Hope llvm makes jump table
                    Register::None, Register::None, Register::None,
                ],

                _ => unreachable!(),
            };

            #[cfg(debug_assertions)]
            asm_marker();

            collect.push(IR {
                op: instr,
                ty,
                regs: registers,
            });

        }
        return collect;
    }

    pub const fn parse_single(line: &[u8; 16]) -> IR{


        let chunk: [u64; 2] = unsafe {std::mem::transmute(*line)};
        let c = chunk[0] ;
        let regs = chunk[1];


        let instr = unsafe {IROps::from_raw(((c >> 48) & 0xff) as u8)};
        //because its a pow 2 being 64 in ascii owowowow :33
        const REG_SEP: char = '@';

        let get_regs_cnt = string_pos(regs, REG_SEP).ilog2() as usize;


        let ty = unsafe {RType::from_raw(((regs >> 48) & 0xff) as u8)};
        //Filter out the ampersand, newline and the type since i already extracted it
        //leaving me with only the registers, they might have different numbers than their ascii counterpart but that's not important
        let maybe_regs0 = regs & !0xAAFFFFFFFFFF4040;
        let maybe_regs1 = (regs & !0xAAFFFFFF4040FFFF) >> 16;
        let maybe_regs2 = (regs & !0xAAFF4040FFFFFFFF) >> 32;


        //zero register instructions are rare so rather precompute cheaply than messing up the branch predictor
        let r0 = RAW_REGISTERS[RawParser::pair_index(maybe_regs0)];

        //TODO check the asm/what llvm produces, if its a jump table or cmov
        //and then try to coax it if need
        #[cfg(debug_assertions)]
        asm_marker();

        // i can probably improve this with a few tricks
        //though improving 3 predictable branches is hard
        let registers = match get_regs_cnt {
            0 => [Register::None, Register::None, Register::None],

            1 => [r0, Register::None, Register::None],

            2 => [
                r0,
                RAW_REGISTERS[RawParser::pair_index(maybe_regs1)],
                Register::None,
            ],
            3 => [
                r0,
                RAW_REGISTERS[RawParser::pair_index(maybe_regs1)],
                RAW_REGISTERS[RawParser::pair_index(maybe_regs2)],
            ],


            _ => unreachable!(),
        };

        IR {
            op: instr,
            ty,
            regs: registers,
        }


    }

    #[inline]
    const fn pair_index(pair: u64) -> usize {
        let a = pair & 0xFF;
        let b = (pair >> 8) & 0xFF;
        return (a * 26 + b - 27) as usize
    }
}

#[inline(never)]
const fn asm_marker() {
    std::hint::black_box(());
}

#[inline]
pub const fn string_pos(string: u64, c: char) -> usize{
    let to_find_mask: u64 = repeat_byte(c as u8) as u64;
    let x: u64 = string ^ to_find_mask;
    let y: u64 = (x.overflowing_sub(0x0101010101010101).0) & !x & 0x8080808080808080;
    (y.trailing_zeros() >> 3) as usize
}

#[inline(always)]
const fn repeat_byte(b: u8) -> usize { (b as usize) * (usize::MAX / 255) }