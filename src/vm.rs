use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::mem::{transmute, MaybeUninit};
use std::ptr;
use std::sync::atomic;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::atomic::Ordering::{Acquire, Release};
use std::thread::JoinHandle;
use seq_macro::seq;
use crate::instructions::{Instructions};
use MaybeRedundant::*;
use MaybeRedundant::config::{Config, ThreadAmount};
use MaybeRedundant::scheduler::Scheduler;
use MaybeRedundant::worker::Worker;

const NANIZATION: u64 = 0x7FF << 52;

const REGISTER_FLAG_OFFSET: u8 = 52;


seq!(N in 0..=511 {
    #[repr(u16)]
    #[derive(Clone, Debug, Copy)]
pub enum RegisterAliases {
        #(
            R~N = N,
        )*
    }
});

pub struct Imm(u64);

pub struct PC(u64);

#[repr(u64)]
enum RegisterFlags{
    //With these set the lower 0..32 bits will be read as int/uint respectively
    InlineI32 = 1 << REGISTER_FLAG_OFFSET - 1,
    InlineU32 = 1 << REGISTER_FLAG_OFFSET - 2,

    //These count as SIMD-Ish register slots
    //If the platform supports it does up to 8 word native load on the internal data states
    //If Avx512 for example is not supported will be done as a loop
    PtrWord1 = 1 << REGISTER_FLAG_OFFSET - 3,
    PtrWord2 = 1 << REGISTER_FLAG_OFFSET - 4,
    PtrWord4 = 1 << REGISTER_FLAG_OFFSET - 5,
    PtrWord8 = 1 << REGISTER_FLAG_OFFSET - 6,


}
const fn load_size_translation<const LOAD_SIZE: usize>() -> usize{
    assert!(LOAD_SIZE.is_power_of_two() && LOAD_SIZE <= 8);
    //The PtrWord section starts at - 3
    return 1 << REGISTER_FLAG_OFFSET - (3 + LOAD_SIZE.ilog2() as u8);
}

const fn ptr_i64<const LOAD_SIZE: usize>() -> i64{
    let x = NANIZATION
        | RegisterFlags::PtrWord1 as u64
        | load_size_translation::<LOAD_SIZE>() as u64;
    x as i64
}
const fn ptr_u64<const LOAD_SIZE: usize>() -> u64{
    NANIZATION
        | RegisterFlags::PtrWord1 as u64
        | load_size_translation::<LOAD_SIZE>() as u64
}
const fn ptr_f64<const LOAD_SIZE: usize>() -> f64{
    f64::from_bits(
        NANIZATION
            | RegisterFlags::PtrWord1 as u64
            | load_size_translation::<LOAD_SIZE>() as u64
    )
}


///Serves as a T register
///with special cases
///TODO implement shared registers for ipc
#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct LocalRegister(u64);
impl LocalRegister {
    pub const fn new(value: f64) -> LocalRegister {
        //Register is repr transparent
        return unsafe {transmute(value)};
    }
    pub const fn inner(self) -> f64 {
        //self is copy and repr transparent
        return unsafe {transmute(self) };
    }
    pub const fn load(context: &mut Context, registers: RegisterAliases) -> LocalRegister {
        let idx: usize = registers as u8 as usize;
        context.registers[idx]
    }

    pub const fn load_n<const N: usize>() -> [LocalRegister; N] {
        todo!()
    }

    pub const fn store(context: &mut Context, register: RegisterAliases, value: LocalRegister) {
        let idx: usize = register as u8 as usize;
        context.registers[idx] = value;
    }
}


pub struct Context<'instr>{
    instruction_pointer: &'instr Instructions,
    registers: [LocalRegister; 256],
}
impl<'instr> Context<'instr>{
    pub fn new(instruction_pointer: &'instr Instructions, registers: [LocalRegister; 256]) -> Self {
        return Context{instruction_pointer, registers}
    }
}


