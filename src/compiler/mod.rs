use crate::chunk::{Chunk, OpCode};
use crate::scanner::Scanner;
use crate::token::{Token, TokenType};
use crate::value::Value;
use crate::vm::Vm;

mod parser;
mod rules;

// ── Types ────────────────────────────────────────────────────────────────────

pub struct Compiler {
    current_token: Token,
    previous_token: Token,
    scanner: Scanner,
    had_error: bool,
    panic_mode: bool,
    locals: Vec<Local>,
    scope_depth: usize,
    vm: *mut Vm,
}

pub struct Local {
    token: Token,
    depth: usize,
    is_initialized: bool,
}

// ── Compiler ─────────────────────────────────────────────────────────────────

impl Compiler {
    pub(crate) fn new(scanner: Scanner, vm: &mut Vm) -> Self {
        Compiler {
            current_token: Token::default(),
            previous_token: Token::default(),
            had_error: false,
            panic_mode: false,
            locals: vec![],
            scope_depth: 0,
            scanner,
            vm: vm as *mut Vm,
        }
    }

    // ── Public entry point ───────────────────────────────────────────────────

    pub fn compile(&mut self, chunk: &mut Chunk) -> bool {
        self.advance();
        while !self.match_token_type(TokenType::EOF) {
            self.declaration(chunk);
        }
        self.end_compiler(chunk);
        !self.had_error
    }

    // ── Emission ─────────────────────────────────────────────────────────────

    fn emit_byte(&self, byte: u8, chunk: &mut Chunk) {
        chunk.write_byte(byte, self.previous_token.line);
    }

    fn emit_bytes(&self, byte1: u8, byte2: u8, chunk: &mut Chunk) {
        self.emit_byte(byte1, chunk);
        self.emit_byte(byte2, chunk);
    }

    fn emit_constant(&mut self, value: Value, chunk: &mut Chunk) {
        let index = chunk.write_constant(value);
        self.emit_bytes(OpCode::OpConstant as u8, index, chunk);
    }

    fn emit_return(&self, chunk: &mut Chunk) {
        self.emit_byte(OpCode::OpReturn as u8, chunk);
    }

    fn end_compiler(&self, chunk: &mut Chunk) {
        self.emit_return(chunk);
    }
}