use std::mem::transmute;
use crate::vm::{Imm, RegisterAliases, PC};

//Names are aligned to 8 chars
#[repr(u8)]
#[derive(Debug)]
pub enum IROps{
    LOAD__A = 65,
    STORE_B,
    MUL___C,
    DIV___D,
    MOD___E,
    CALL__F,
}
impl IROps {
    pub(crate) const unsafe fn from_raw(i: u8) -> IROps{
        transmute(i)
    }
}
#[repr(u8)]
#[derive(Debug)]
pub enum RType{
    Unknown = 63, //Question mark in ascii
    //must skip et sign
    Nil = 65,
    Int,
    Float,
}


impl RType {
    pub(crate) const unsafe fn from_raw(i: u8) -> RType{
        transmute(i)
    }
}


#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Register{
    General(u8),
    Constant(u8),
    Imm(u8),
    None
}


#[derive(Debug)]
pub struct IR{
    pub regs: [Register; 3],
    pub op: IROps,
    pub ty: RType,
}


pub enum Instructions {
    MOV(RegisterAliases, Imm),
    ADD(RegisterAliases, Imm),
    SUB(RegisterAliases, Imm),
    MUL(RegisterAliases, Imm),
    SHL(RegisterAliases, Imm),
    SHR(RegisterAliases, Imm),
    SRA(RegisterAliases, Imm),
    JMP(Imm),
}