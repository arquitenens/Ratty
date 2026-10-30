use std::mem::transmute;
use crate::vm::{Imm, RegisterAliases, PC};

//Names are aligned to 8 chars
#[repr(u8)]
#[derive(Debug)]
pub enum IR{
    LOAD__A,
    STORE_B,
    MUL___C,
    DIV___D,
    MOD___E,
    CALL__F,
}
impl IR {
    pub(crate) unsafe fn from_raw(i: u8) -> IR{
        transmute(i)
    }
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