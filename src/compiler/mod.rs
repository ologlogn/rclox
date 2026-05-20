use crate::chunk::{Chunk, OpCode};
use crate::function::{FunctionObject, FunctionType};
use crate::scanner::Scanner;
use crate::token::TokenType;
use crate::value::{Object, ObjectType, Value};
use crate::vm::Vm;

mod frame;
mod parser;
mod rules;

use frame::FunctionCompiler;
use parser::Parser;

// ── Types ────────────────────────────────────────────────────────────────────

pub struct Compiler {
    pub parser: Parser,
    pub frames: Vec<FunctionCompiler>,
    pub vm: *mut Vm,
}

// ── Compiler ─────────────────────────────────────────────────────────────────

impl Compiler {
    pub fn new(scanner: Scanner, vm: &mut Vm, function_type: FunctionType) -> Self {
        let chunk = Chunk::new();
        let init_function = vm.allocate_function(FunctionObject::new(chunk, 0, ""));
        Compiler {
            parser: Parser::new(scanner),
            frames: vec![FunctionCompiler::new(init_function, function_type)],
            vm,
        }
    }

    pub fn frame(&mut self) -> &mut FunctionCompiler {
        self.frames.last_mut().expect("compiler stack should not be empty")
    }

    pub fn function(&mut self) -> *mut Object {
        self.frame().function
    }

    pub fn chunk(&mut self) -> &mut Chunk {
        let function = self.function();
        unsafe {
            match &mut (*function).obj_type {
                ObjectType::Function(func) => &mut func.chunk,
                _ => unreachable!(),
            }
        }
    }

    // ── Public entry point ───────────────────────────────────────────────────

    pub fn compile(&mut self) -> Option<*mut Object> {
        self.parser.advance();
        while !self.parser.match_token_type(TokenType::EOF) {
            self.declaration();
        }
        self.end_compiler()
    }

    pub fn end_compiler(&mut self) -> Option<*mut Object> {
        self.emit_byte(OpCode::OpNil as u8);
        self.emit_return();
        println!("{:?}", self.chunk());
        if !self.parser.had_error {
            let fun = self.function();
            self.frames.pop();
            Some(fun)
        } else {
            None
        }
    }

    // ── Emission ─────────────────────────────────────────────────────────────

    pub fn emit_byte(&mut self, byte: u8) {
        let line = self.parser.previous_token.line;
        self.chunk().write_byte(byte, line);
    }

    pub fn emit_bytes(&mut self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    pub fn emit_constant(&mut self, value: Value) {
        let index = self.chunk().write_constant(value);
        self.emit_bytes(OpCode::OpConstant as u8, index);
    }

    pub fn emit_return(&mut self) {
        self.emit_byte(OpCode::OpReturn as u8);
    }

    pub fn emit_loop(&mut self, loop_start: usize) {
        self.emit_byte(OpCode::OpLoop as u8);
        let offset = (self.chunk().count() - loop_start + 2) as u16;
        self.emit_bytes((offset >> 8 & 0xff) as u8, (offset & 0xff) as u8);
    }

    pub fn emit_jump(&mut self, op_code: OpCode) -> usize {
        self.emit_byte(op_code as u8);
        self.emit_bytes(0xff, 0xff);
        self.chunk().count() - 2
    }

    pub fn patch_jump(&mut self, offset: usize) {
        let jump = (self.chunk().count() - offset - 2) as u16;
        self.chunk().write_byte_at(offset, (jump >> 8) as u8);
        self.chunk().write_byte_at(offset + 1, (jump & 0xff) as u8);
    }

    pub fn emit_pop(&mut self) {
        self.emit_byte(OpCode::OpPop as u8);
    }
}
