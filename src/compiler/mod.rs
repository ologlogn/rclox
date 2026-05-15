use crate::chunk::{Chunk, OpCode};
use crate::scanner::Scanner;
use crate::token::{Token, TokenType};
use crate::value::Value;
use crate::vm::Vm;

mod parser;
mod rules;

pub struct Compiler {
    current_token: Token,
    previous_token: Token,
    had_error: bool,
    panic_mode: bool,
}
impl Compiler {
    pub(crate) fn new() -> Self {
        Compiler {
            current_token: Token::default(),
            previous_token: Token::default(),
            had_error: false,
            panic_mode: false,
        }
    }
    fn emit_byte(&self, byte: u8, chunk: &mut Chunk) {
        chunk.write_byte(byte, self.previous_token.line);
    }
    fn emit_bytes(&self, byte1: u8, byte2: u8, chunk: &mut Chunk) {
        self.emit_byte(byte1, chunk);
        self.emit_byte(byte2, chunk);
    }

    fn emit_constant(&mut self, val: Value, chunk: &mut Chunk) {
        let constant = chunk.write_constant(val);
        self.emit_bytes(OpCode::OpConstant as u8, constant, chunk);
    }
    fn emit_return(&self, chunk: &mut Chunk) {
        self.emit_byte(OpCode::OpReturn as u8, chunk)
    }
    fn end(&self, chunk: &mut Chunk) {
        self.emit_return(chunk);
    }
    pub fn compile(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) -> bool {
        self.advance(scanner);
        while !self.match_token_type(TokenType::EOF, scanner) {
            self.declaration(scanner, chunk, vm);
        }
        self.end(chunk);
        !self.had_error
    }
}
