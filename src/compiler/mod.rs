use crate::chunk::{Chunk, OpCode};
use crate::function::{FunctionObject, FunctionType};
use crate::scanner::Scanner;
use crate::token::{Token, TokenType};
use crate::value::{Object, Value};
use crate::vm::Vm;

mod frame;
mod parser;
mod rules;

use crate::closure::CompilerUpvalue;
use crate::compiler::frame::Local;
use crate::compiler::rules::{Precedence, get_rule};
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

    fn vm(&mut self) -> &mut Vm {
        unsafe { self.vm.as_mut().unwrap() }
    }

    fn current_function_mut(&mut self) -> &mut FunctionObject {
        unsafe { &mut *self.function() }.as_function_mut()
    }

    pub fn chunk(&mut self) -> &mut Chunk {
        &mut self.current_function_mut().chunk
    }

    pub fn consume(&mut self, token_type: TokenType, message: &str) {
        self.parser.consume(token_type, message);
    }
    pub fn error_at(&mut self, token: Token, message: &str) {
        self.parser.error_at(token, message);
    }
    pub fn locals(&mut self) -> &mut Vec<Local> {
        &mut self.frame().locals
    }
    pub fn jumps(&mut self) -> &mut Vec<(usize, usize, Vec<usize>)> {
        &mut self.frame().jumps
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
            let upvalue_count = self.frame().upvalues.len();
            let function_ptr = self.frame().function;
            self.current_function_mut().upvalue_count = upvalue_count;
            self.frames.pop();
            Some(function_ptr)
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

    // ── Pratt parser ─────────────────────────────────────────────────────────
    fn parse_precedence(&mut self, precedence: Precedence) {
        self.parser.advance();
        let current_can_assign = self.parser.can_assign;
        self.parser.can_assign = precedence <= Precedence::Assignment;
        let prev_type = self.parser.previous_token.token_type;
        match get_rule(prev_type).prefix {
            Some(prefix_fn) => prefix_fn(self),
            None => {
                let prev = self.parser.previous_token;
                self.error_at(prev, "Expected expression.");
                return;
            }
        }
        while precedence <= get_rule(self.parser.current_token.token_type).precedence {
            self.parser.advance();
            let prev_type = self.parser.previous_token.token_type;
            if let Some(infix_fn) = get_rule(prev_type).infix {
                infix_fn(self);
            }
        }
        if self.parser.can_assign && self.parser.match_token_type(TokenType::Equal) {
            let prev = self.parser.previous_token;
            self.error_at(prev, "Invalid assignment target.");
        }
        self.parser.can_assign = current_can_assign;
    }

    pub(super) fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    // ── Parse functions (prefix / infix) ─────────────────────────────────────

    pub(super) fn number(&mut self) {
        let lexeme = self.parser.scanner.get_lexeme(self.parser.previous_token).to_string();
        let val = Value::Number(lexeme.parse().unwrap());
        self.emit_constant(val);
    }

    pub(super) fn string(&mut self) {
        let lexeme = self.parser.scanner.get_lexeme(self.parser.previous_token);
        let string_value = lexeme[1..lexeme.len() - 1].to_string();
        let obj_ptr = self.vm().allocate_string(string_value.as_str());
        self.emit_constant(Value::Object(obj_ptr));
    }

    pub(super) fn variable(&mut self) {
        let (get_op, set_op, arg) = self.resolve_();
        if self.parser.match_token_type(TokenType::PlusPlus) {
            self.postfix_(get_op, set_op, OpCode::OpAdd, arg);
        } else if self.parser.match_token_type(TokenType::MinusMinus) {
            self.postfix_(get_op, set_op, OpCode::OpSubtract, arg);
        } else if self.parser.match_token_type(TokenType::LeftBracket) {
            self.emit_bytes(get_op as u8, arg); // get the array variable on stack.
            self.expression(); //index
            self.consume(TokenType::RightBracket, "Expect ']'");
            if self.parser.can_assign && self.parser.match_token_type(TokenType::Equal) {
                self.expression(); // value
                self.emit_byte(OpCode::OpSetIndex as u8);
            } else {
                self.emit_byte(OpCode::OpGetIndex as u8);
            }
        } else if self.parser.can_assign && self.parser.match_token_type(TokenType::Equal) {
            self.expression();
            self.emit_bytes(set_op as u8, arg);
        } else {
            self.emit_bytes(get_op as u8, arg);
        }
    }

    pub(super) fn grouping(&mut self) {
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after expression");
    }

    pub(super) fn unary(&mut self) {
        let operator_type = self.parser.previous_token.token_type;
        self.parse_precedence(Precedence::Unary);
        match operator_type {
            TokenType::Minus => self.emit_byte(OpCode::OpNegate as u8),
            TokenType::Bang => self.emit_byte(OpCode::OpNot as u8),
            _ => unreachable!("Unknown unary operator"),
        }
    }
    pub(super) fn prefix_(&mut self) {
        let op = match self.parser.previous_token.token_type {
            TokenType::PlusPlus => OpCode::OpAdd,
            TokenType::MinusMinus => OpCode::OpSubtract,
            _ => unreachable!("Unknown prefix operator"),
        };
        self.parser.advance();
        let (get_op, set_op, arg) = self.resolve_();
        self.emit_bytes(get_op as u8, arg);
        self.emit_constant(Value::Number(1.0));
        self.emit_byte(op as u8);
        self.emit_bytes(set_op as u8, arg);
    }
    pub(super) fn postfix_(&mut self, get_op: OpCode, set_op: OpCode, op: OpCode, arg: u8) {
        self.emit_bytes(get_op as u8, arg); // [4]
        self.emit_byte(OpCode::OpDup as u8); // [4, 4]
        self.emit_constant(Value::Number(1.0)); // [4, 4, 1]
        self.emit_byte(op as u8); // [4, 5]
        self.emit_bytes(set_op as u8, arg); // [4, 5] → sets, leaves 5
        self.emit_byte(OpCode::OpPop as u8); // [4]
    }

    pub(super) fn binary(&mut self) {
        let operator_type = self.parser.previous_token.token_type;
        let rule = get_rule(operator_type);
        self.parse_precedence(Precedence::try_from(rule.precedence as u8 + 1).unwrap());
        match operator_type {
            TokenType::BangEqual => self.emit_bytes(OpCode::OpEqual as u8, OpCode::OpNot as u8),
            TokenType::EqualEqual => self.emit_byte(OpCode::OpEqual as u8),
            TokenType::Greater => self.emit_byte(OpCode::OpGreater as u8),
            TokenType::Less => self.emit_byte(OpCode::OpLess as u8),
            TokenType::GreaterEqual => self.emit_bytes(OpCode::OpLess as u8, OpCode::OpNot as u8),
            TokenType::LessEqual => self.emit_bytes(OpCode::OpGreater as u8, OpCode::OpNot as u8),
            TokenType::Plus => self.emit_byte(OpCode::OpAdd as u8),
            TokenType::Minus => self.emit_byte(OpCode::OpSubtract as u8),
            TokenType::Star => self.emit_byte(OpCode::OpMultiply as u8),
            TokenType::Slash => self.emit_byte(OpCode::OpDivide as u8),
            _ => unreachable!("Unknown binary operator"),
        }
    }

    pub(super) fn literal(&mut self) {
        match self.parser.previous_token.token_type {
            TokenType::Nil => self.emit_byte(OpCode::OpNil as u8),
            TokenType::True => self.emit_byte(OpCode::OpTrue as u8),
            TokenType::False => self.emit_byte(OpCode::OpFalse as u8),
            _ => unreachable!("Unknown literal"),
        }
    }

    pub(super) fn and_(&mut self) {
        let end_jump = self.emit_jump(OpCode::OpJumpIfFalse);
        self.emit_pop();
        self.parse_precedence(Precedence::And);
        self.patch_jump(end_jump);
    }

    pub(super) fn or_(&mut self) {
        let else_jump = self.emit_jump(OpCode::OpJumpIfFalse);
        let end_jump = self.emit_jump(OpCode::OpJump);
        self.patch_jump(else_jump);
        self.emit_pop();
        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
    }

    pub(super) fn switch(&mut self) {
        self.expression();
        let mut end_jumps: Vec<usize> = vec![];
        self.consume(TokenType::LeftBrace, "Expected '{'");
        while self.parser.match_token_type(TokenType::Case) {
            self.emit_byte(OpCode::OpDup as u8);
            self.expression();
            self.emit_byte(OpCode::OpEqual as u8);
            self.consume(TokenType::EqualGreater, "Expected '=>'");
            let fail_jump = self.emit_jump(OpCode::OpJumpIfFalse);
            self.emit_pop();
            self.emit_pop();
            self.case_block();
            let end_jump = self.emit_jump(OpCode::OpJump);
            end_jumps.push(end_jump);
            self.patch_jump(fail_jump);
            self.emit_pop();
        }
        self.emit_pop();
        self.consume(TokenType::Default, "Expected default");
        self.consume(TokenType::EqualGreater, "Expected '=>'");
        self.case_block();
        self.consume(TokenType::RightBrace, "Expected '}'");
        for jump in end_jumps {
            self.patch_jump(jump);
        }
    }

    // ── Array ────────────────────────────────────────────────────────
    pub(super) fn array(&mut self) {
        let mut count = 0;
        while !self.parser.check(TokenType::RightBracket) {
            self.expression(); // Value
            count += 1;
            if !self.parser.match_token_type(TokenType::Comma) {
                break;
            }
        }
        self.consume(TokenType::RightBracket, "Expected ']'");
        self.emit_bytes(OpCode::OpArray as u8, count as u8);
    }
    pub(super) fn make_array(&mut self) {
        self.consume(TokenType::LeftParen, "Expected '('");
        self.expression(); // length
        self.consume(TokenType::RightParen, "Expected ')'");
        self.emit_byte(OpCode::OpMakeArray as u8);
    }

    pub(super) fn len_(&mut self) {
        self.consume(TokenType::LeftParen, "Expected '('");
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')'");
        self.emit_byte(OpCode::OpLen as u8);
    }

    // ── Functions ────────────────────────────────────────────────────────
    pub(super) fn call(&mut self) {
        let mut arguments = 0;
        if !self.parser.check(TokenType::RightParen) {
            loop {
                self.expression();
                arguments += 1;
                if !self.parser.match_token_type(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expected ')'");
        self.emit_bytes(OpCode::OpCall as u8, arguments);
    }

    // ── Scope & locals ────────────────────────────────────────────────────────

    fn resolve_(&mut self) -> (OpCode, OpCode, u8) {
        let (is_local, local_idx) = self.resolve_local(self.frames.len() - 1);
        if is_local {
            return (OpCode::OpGetLocal, OpCode::OpSetLocal, local_idx);
        }
        let (found, upvalue_idx) = self.resolve_upvalue(self.frames.len() - 1);
        if found {
            return (OpCode::OpGetUpvalue, OpCode::OpSetUpvalue, upvalue_idx);
        }
        let global_idx = self.identifier_constant();
        (OpCode::OpGetGlobal, OpCode::OpSetGlobal, global_idx)
    }

    fn discard_locals(&mut self, target_depth: usize, modify_compiler_state: bool, with_value: bool) {
        let mut pop_count = 0;
        {
            let locals = &self.locals();
            for local in locals.iter().rev() {
                if local.depth <= target_depth {
                    break;
                }
                pop_count += 1;
            }
        }
        if modify_compiler_state {
            for _ in 0..pop_count {
                self.locals().pop();
            }
        }
        if pop_count > 0 {
            if with_value {
                self.emit_bytes(OpCode::OpYield as u8, pop_count as u8);
            } else {
                self.emit_bytes(OpCode::OpPopN as u8, pop_count as u8);
            }
        }
    }

    fn begin_scope(&mut self) {
        self.frame().scope_depth += 1;
    }

    fn end_scope(&mut self, with_value: bool) {
        self.frame().scope_depth -= 1;
        let depth = self.frame().scope_depth;
        self.discard_locals(depth, true, with_value);
    }

    fn resolve_local(&mut self, frame_ix: usize) -> (bool, u8) {
        let token = self.parser.previous_token;
        let mut found_uninitialized = false;
        let len = self.frames[frame_ix].locals.len();
        for i in (0..len).rev() {
            let local_token = self.frames[frame_ix].locals[i].token;
            let local_is_initialized = self.frames[frame_ix].locals[i].is_initialized;
            if self.same_identifier(local_token, token) {
                if !local_is_initialized {
                    found_uninitialized = true;
                    continue;
                }
                return (true, i as u8);
            }
        }
        if found_uninitialized {
            self.error_at(token, "Can't read local variable in its own initializer");
        }
        (false, 0)
    }

    fn resolve_upvalue(&mut self, frame_ix: usize) -> (bool, u8) {
        if frame_ix == 0 {
            return (false, 0);
        }
        let (found, local_slot) = self.resolve_local(frame_ix - 1);
        if found {
            self.frames[frame_ix - 1].locals[local_slot as usize].is_captured = true;
            return (true, self.add_upvalue(frame_ix, local_slot, true));
        }
        let (found, upvalue_slot) = self.resolve_upvalue(frame_ix - 1);
        if found {
            return (true, self.add_upvalue(frame_ix, upvalue_slot, false));
        }
        (false, 0)
    }

    fn add_upvalue(&mut self, frame_ix: usize, index: u8, is_local: bool) -> u8 {
        for (i, uv) in self.frames[frame_ix].upvalues.iter().enumerate() {
            if uv.index == index && uv.is_local == is_local {
                return i as u8;
            }
        }
        let slot = self.frames[frame_ix].upvalues.len() as u8;
        self.frames[frame_ix].upvalues.push(CompilerUpvalue { index, is_local });
        slot
    }

    fn same_identifier(&self, a: Token, b: Token) -> bool {
        a.length == b.length && self.parser.scanner.get_lexeme(a) == self.parser.scanner.get_lexeme(b)
    }

    // ── Variables ─────────────────────────────────────────────────────────────

    fn identifier_constant(&mut self) -> u8 {
        let name = self.parser.scanner.get_lexeme(self.parser.previous_token).to_string();
        let var_name = self.vm().allocate_string(&name);
        self.chunk().write_constant(Value::Object(var_name))
    }

    fn parse_variable(&mut self, message: &str) -> u8 {
        self.consume(TokenType::Identifier, message);
        let token = self.parser.previous_token;
        let scope_depth = self.frame().scope_depth;
        if scope_depth > 0 {
            self.declare_variable_late(token);
            return 0;
        }
        self.identifier_constant()
    }

    fn mark_initialized(&mut self) {
        if self.frame().scope_depth == 0 {
            return;
        }
        self.locals().last_mut().unwrap().is_initialized = true;
    }

    fn define_variable(&mut self, global: u8) {
        if self.frame().scope_depth > 0 {
            return;
        }
        self.emit_bytes(OpCode::OpDefineGlobal as u8, global);
    }

    fn function_statement(&mut self, function_type: FunctionType) {
        let name = self.parser.scanner.get_lexeme(self.parser.previous_token).to_string();
        let func = self.vm().allocate_function(FunctionObject::new(Chunk::new(), 0, &name));
        self.frames.push(FunctionCompiler::new(func, function_type));
        self.begin_scope();

        self.consume(TokenType::LeftParen, "Expect '(' after function name.");
        if !self.parser.check(TokenType::RightParen) {
            loop {
                self.current_function_mut().arity += 1;
                let param = self.parse_variable("Expect parameter name.");
                self.define_variable(param);
                self.mark_initialized();
                if !self.parser.match_token_type(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after parameters.");
        self.consume(TokenType::LeftBrace, "Expect '{' before function body.");
        self.block();
        let upvalues = self.frame().upvalues.clone();
        let compiled = self.end_compiler();
        let idx = self.chunk().write_constant(Value::Object(compiled.unwrap()));
        self.emit_bytes(OpCode::OpClosure as u8, idx);
        for uv in &upvalues {
            self.emit_byte(if uv.is_local { 1 } else { 0 });
            self.emit_byte(uv.index);
        }
    }

    // ── Control flow ──────────────────────────────────────────────────────────

    fn case_block(&mut self) {
        self.begin_scope();
        self.consume(TokenType::LeftBrace, "Expected '{' after case.");
        while !self.parser.check(TokenType::Yield) && !self.parser.check(TokenType::EOF) && !self.parser.check(TokenType::RightBrace) {
            self.declaration();
        }
        if self.parser.check(TokenType::Yield) {
            self.parser.advance();
            self.expression();
            self.consume(TokenType::Semicolon, "Expected ';' after yield expression.");
        } else {
            self.emit_byte(OpCode::OpNil as u8);
        }
        self.consume(TokenType::RightBrace, "Expected '}'.");
        self.end_scope(true);
    }

    fn for_statement(&mut self) {
        self.begin_scope();
        let scope_depth = self.frame().scope_depth;
        self.jumps().push((scope_depth, 0, Vec::new()));
        self.consume(TokenType::LeftParen, "Expected '(' after 'while'.");
        if self.parser.match_token_type(TokenType::Semicolon) {
        } else if self.parser.match_token_type(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }
        let mut loop_start = self.chunk().count();
        self.jumps().last_mut().unwrap().1 = loop_start;
        let mut is_conditional = false;
        let mut exit_jump = 0;
        if !self.parser.match_token_type(TokenType::Semicolon) {
            is_conditional = true;
            self.expression();
            self.consume(TokenType::Semicolon, "Expected ';' after the loop condition.");
            exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
            self.emit_pop();
        }
        if !self.parser.match_token_type(TokenType::RightParen) {
            let body_jump = self.emit_jump(OpCode::OpJump);
            let increment_start = self.chunk().count();
            self.expression();
            self.emit_pop();
            self.consume(TokenType::RightParen, "Expected ')' after clauses.");
            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.jumps().last_mut().unwrap().1 = loop_start;
            self.patch_jump(body_jump);
        }
        self.statement();
        self.emit_loop(loop_start);
        if is_conditional {
            self.patch_jump(exit_jump);
            self.emit_pop();
        }
        let (_, _, breaks) = self.jumps().pop().unwrap();
        for break_ in breaks {
            self.patch_jump(break_);
        }
        self.end_scope(false);
    }

    fn while_statement(&mut self) {
        let loop_start = self.chunk().count();
        let scope_depth = self.frame().scope_depth;
        self.jumps().push((scope_depth, loop_start, Vec::new()));
        self.consume(TokenType::LeftParen, "Expected '(' after 'while'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after condition.");
        let exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
        self.emit_pop();
        self.statement();
        self.emit_loop(loop_start);
        self.patch_jump(exit_jump);
        self.emit_pop();
        let (_, _, breaks) = self.jumps().pop().unwrap();
        for break_ in breaks {
            self.patch_jump(break_);
        }
    }

    fn if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expected '(' after 'if'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after condition.");
        let then_jump = self.emit_jump(OpCode::OpJumpIfFalse);
        self.emit_pop();
        self.statement();
        let else_jump = self.emit_jump(OpCode::OpJump);
        self.patch_jump(then_jump);
        self.emit_pop();
        if self.parser.match_token_type(TokenType::Else) {
            self.statement();
        }
        self.patch_jump(else_jump);
    }

    fn break_statement(&mut self) {
        self.consume(TokenType::Semicolon, "Expected ';' after break.");
        let jump_info = self.jumps().pop();
        if let Some((loop_depth, start, mut jump)) = jump_info {
            self.discard_locals(loop_depth, false, false);
            let emit_jump = self.emit_jump(OpCode::OpJump);
            jump.push(emit_jump);
            self.jumps().push((loop_depth, start, jump));
        } else {
            let prev = self.parser.previous_token;
            self.error_at(prev, "Break can't be used outside loops");
        }
    }

    fn continue_statement(&mut self) {
        self.consume(TokenType::Semicolon, "Expected ';' after continue.");
        let jump_info = self.jumps().pop();
        if let Some((loop_depth, start, jump)) = jump_info {
            self.discard_locals(loop_depth, false, false);
            self.emit_loop(start);
            self.jumps().push((loop_depth, start, jump));
        } else {
            let prev = self.parser.previous_token;
            self.error_at(prev, "Continue can't be used outside loops");
        }
    }

    // ── Statements & declarations ─────────────────────────────────────────────

    fn statement(&mut self) {
        if self.parser.match_token_type(TokenType::Print) {
            self.print_statement();
        } else if self.parser.match_token_type(TokenType::Break) {
            self.break_statement();
        } else if self.parser.match_token_type(TokenType::Continue) {
            self.continue_statement();
        } else if self.parser.match_token_type(TokenType::For) {
            self.for_statement();
        } else if self.parser.match_token_type(TokenType::If) {
            self.if_statement();
        } else if self.parser.match_token_type(TokenType::While) {
            self.while_statement();
        } else if self.parser.match_token_type(TokenType::Return) {
            self.return_statement();
        } else if self.parser.match_token_type(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope(false);
        } else {
            self.expression_statement();
        }
    }
    fn return_statement(&mut self) {
        if self.frame().function_type == FunctionType::TypeScript {
            self.error_at(self.parser.previous_token, "Can't use return outside a function")
        }
        if self.parser.match_token_type(TokenType::Semicolon) {
            self.emit_byte(OpCode::OpNil as u8);
            self.emit_return();
        } else {
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after return value.");
            self.emit_return();
        }
    }
    pub(super) fn declaration(&mut self) {
        if self.parser.match_token_type(TokenType::Var) {
            self.var_declaration();
        } else if self.parser.match_token_type(TokenType::Fun) {
            self.fun_declaration();
        } else {
            self.statement();
        }
        if self.parser.panic_mode {
            self.parser.synchronize();
        }
    }

    fn fun_declaration(&mut self) {
        let name = self.parse_variable("Expect function name.");
        self.mark_initialized();
        self.function_statement(FunctionType::TypeFunction);
        self.define_variable(name);
    }

    fn var_declaration(&mut self) {
        self.consume(TokenType::Identifier, "Expected variable name");
        let token = self.parser.previous_token;
        let scope_depth = self.frame().scope_depth;
        let constant = if scope_depth == 0 { self.identifier_constant() } else { 0 };

        if self.parser.match_token_type(TokenType::Equal) {
            self.expression();
        } else {
            self.emit_byte(OpCode::OpNil as u8);
        }
        self.consume(TokenType::Semicolon, "Expected ';' after variable declaration.");

        if self.frame().scope_depth > 0 {
            self.declare_variable_late(token);
            self.mark_initialized();
            return;
        }
        self.emit_bytes(OpCode::OpDefineGlobal as u8, constant);
    }

    fn declare_variable_late(&mut self, token: Token) {
        let scope_depth = self.frame().scope_depth;
        let len = self.locals().len();
        for i in (0..len).rev() {
            let local_depth = self.locals()[i].depth;
            let local_token = self.locals()[i].token;
            if local_depth < scope_depth {
                break;
            }
            if self.same_identifier(token, local_token) {
                self.error_at(token, "Already a variable with this name");
                return;
            }
        }
        self.frame().add_local(token);
    }

    fn block(&mut self) {
        while !self.parser.check(TokenType::RightBrace) && !self.parser.check(TokenType::EOF) {
            self.declaration();
        }
        self.consume(TokenType::RightBrace, "Expected '}' after block");
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' after value.");
        self.emit_byte(OpCode::OpPrint as u8);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' after expression.");
        self.emit_pop();
    }
}
