use crate::chunk::{Chunk, OpCode};
use crate::function::FunctionType;
use crate::scanner::Scanner;
use crate::token::{Token, TokenType};
use crate::value::{Object, ObjectType, Value};
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
    //loop jump depth in the program scope, starting offset of loop, jumps in the loop
    jumps: Vec<(usize, usize, Vec<usize>)>,
    can_assign: bool,
    function: *mut Object,
    function_type: FunctionType,
}

pub struct Local {
    token: Token,
    depth: usize,
    is_initialized: bool,
}

// ── Compiler ─────────────────────────────────────────────────────────────────

impl Compiler {
    pub(crate) fn new(scanner: Scanner, vm: *mut Vm, func: *mut Object, function_type: FunctionType) -> Self {
        let locals = vec![Local {
            token: Token {
                token_type: TokenType::Identifier,
                length: 0,
                start: 0,
                line: 0,
            },
            depth: 0,
            is_initialized: true,
        }];
        Compiler {
            current_token: Token::default(),
            previous_token: Token::default(),
            had_error: false,
            panic_mode: false,
            locals,
            scope_depth: 0,
            scanner,
            vm,
            jumps: vec![],
            can_assign: false,
            function: func,
            function_type,
        }
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        unsafe {
            match &mut (*self.function).obj_type {
                ObjectType::Function(func) => &mut func.chunk,
                _ => unreachable!(),
            }
        }
    }

    // ── Public entry point ───────────────────────────────────────────────────

    pub fn compile(&mut self) -> bool {
        self.advance();
        while !self.match_token_type(TokenType::EOF) {
            self.declaration();
        }
        self.end_compiler();
        println!("{:?}", self.current_chunk());
        !self.had_error
    }

    // ── Emission ─────────────────────────────────────────────────────────────

    fn emit_byte(&mut self, byte: u8) {
        let line = self.previous_token.line;
        self.current_chunk().write_byte(byte, line);
    }

    fn emit_bytes(&mut self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    fn emit_constant(&mut self, value: Value) {
        let index = self.current_chunk().write_constant(value);
        self.emit_bytes(OpCode::OpConstant as u8, index);
    }

    fn emit_return(&mut self) {
        self.emit_byte(OpCode::OpReturn as u8);
    }

    fn end_compiler(&mut self) {
        self.emit_return();
    }
    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_byte(OpCode::OpLoop as u8);
        let offset = (self.current_chunk().count() - loop_start + 2) as u16;
        self.emit_bytes((offset >> 8 & 0xff) as u8, (offset & 0xff) as u8);
    }

    fn emit_jump(&mut self, op_code: OpCode) -> usize {
        self.emit_byte(op_code as u8);
        self.emit_bytes(0xff, 0xff);
        self.current_chunk().count() - 2
    }
    fn patch_jump(&mut self, offset: usize) {
        let jump = (self.current_chunk().count() - offset - 2) as u16;
        self.current_chunk().write_byte_at(offset, (jump >> 8) as u8);
        self.current_chunk().write_byte_at(offset + 1, (jump & 0xff) as u8);
    }

    fn emit_pop(&mut self) {
        self.emit_byte(OpCode::OpPop as u8);
    }
}
