use super::Compiler;
use crate::chunk::{Chunk, OpCode};
use crate::compiler::rules::{Precedence, get_rule};
use crate::function::{FunctionObject, FunctionType};
use crate::scanner::Scanner;
use crate::token::{Token, TokenType};
use crate::value::{ObjectType, Value};

// ── Parser ───────────────────────────────────────────────────────────────────

pub struct Parser {
    pub current_token: Token,
    pub previous_token: Token,
    pub scanner: Scanner,
    pub had_error: bool,
    pub panic_mode: bool,
}

impl Parser {
    pub fn new(scanner: Scanner) -> Self {
        Parser {
            current_token: Token::default(),
            previous_token: Token::default(),
            scanner,
            had_error: false,
            panic_mode: false,
        }
    }

    pub fn error_at(&mut self, token: Token, message: &str) {
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;
        eprint!("[line {}] Error", token.line);
        match token.token_type {
            TokenType::EOF => eprint!(" at end"),
            TokenType::Error(_) => {}
            _ => eprint!(" at '{}'", self.scanner.get_lexeme(token)),
        }
        eprintln!(": {}", message);
        self.had_error = true;
    }

    pub fn synchronize(&mut self) {
        self.panic_mode = false;
        while self.current_token.token_type != TokenType::EOF {
            if self.previous_token.token_type == TokenType::Semicolon {
                return;
            }
            match self.current_token.token_type {
                TokenType::Class
                | TokenType::Fun
                | TokenType::If
                | TokenType::While
                | TokenType::Var
                | TokenType::For
                | TokenType::Print
                | TokenType::Return => return,
                _ => {}
            }
            self.advance();
        }
    }

    pub fn advance(&mut self) {
        self.previous_token = self.current_token;
        loop {
            self.current_token = self.scanner.next_token();
            if let TokenType::Error(message) = self.current_token.token_type {
                let token = self.current_token;
                self.error_at(token, message);
            } else {
                break;
            }
        }
    }

    pub fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.current_token.token_type != token_type {
            let token = self.current_token;
            self.error_at(token, message);
        } else {
            self.advance();
        }
    }

    pub fn check(&self, token_type: TokenType) -> bool {
        self.current_token.token_type == token_type
    }

    pub fn match_token_type(&mut self, token_type: TokenType) -> bool {
        if !self.check(token_type) {
            false
        } else {
            self.advance();
            true
        }
    }
}

impl Compiler {
    // ── Pratt parser ─────────────────────────────────────────────────────────

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.parser.advance();
        let current_can_assign = self.can_assign;
        self.can_assign = precedence <= Precedence::Assignment;
        let prev_type = self.parser.previous_token.token_type;
        match get_rule(prev_type).prefix {
            Some(prefix_fn) => prefix_fn(self),
            None => {
                let prev = self.parser.previous_token;
                self.parser.error_at(prev, "Expected expression.");
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
        if self.can_assign && self.parser.match_token_type(TokenType::Equal) {
            let prev = self.parser.previous_token;
            self.parser.error_at(prev, "Invalid assignment target.");
        }
        self.can_assign = current_can_assign;
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
        let obj_ptr = unsafe { self.vm.as_mut().unwrap().allocate_string(string_value.as_str()) };
        self.emit_constant(Value::Object(obj_ptr));
    }

    pub(super) fn variable(&mut self) {
        let (is_local, local_idx) = self.resolve_local();
        let (get_op, set_op, arg) = if is_local {
            (OpCode::OpGetLocal, OpCode::OpSetLocal, local_idx)
        } else {
            let global_idx = self.identifier_constant();
            (OpCode::OpGetGlobal, OpCode::OpSetGlobal, global_idx)
        };
        if self.can_assign && self.parser.match_token_type(TokenType::Equal) {
            self.expression();
            self.emit_bytes(set_op as u8, arg);
        } else {
            self.emit_bytes(get_op as u8, arg);
        }
    }

    pub(super) fn grouping(&mut self) {
        self.expression();
        self.parser.consume(TokenType::RightParen, "Expected ')' after expression");
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
        self.parser.consume(TokenType::LeftBrace, "Expected '{'");
        while self.parser.match_token_type(TokenType::Case) {
            self.emit_byte(OpCode::OpDup as u8);
            self.expression();
            self.emit_byte(OpCode::OpEqual as u8);
            self.parser.consume(TokenType::EqualGreater, "Expected '=>'");
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
        self.parser.consume(TokenType::Default, "Expected default");
        self.parser.consume(TokenType::EqualGreater, "Expected '=>'");
        self.case_block();
        self.parser.consume(TokenType::RightBrace, "Expected '}'");
        for jump in end_jumps {
            self.patch_jump(jump);
        }
    }

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
        self.parser.consume(TokenType::RightParen, "Expected ')'");
        self.emit_bytes(OpCode::OpCall as u8, arguments);
    }

    // ── Scope & locals ────────────────────────────────────────────────────────

    fn discard_locals(&mut self, target_depth: usize, modify_compiler_state: bool, with_value: bool) {
        let mut pop_count = 0;
        {
            let locals = &self.frames.last().unwrap().locals;
            for local in locals.iter().rev() {
                if local.depth <= target_depth {
                    break;
                }
                pop_count += 1;
            }
        }
        if modify_compiler_state {
            for _ in 0..pop_count {
                self.frames.last_mut().unwrap().locals.pop();
            }
        }
        if pop_count > 0 {
            if with_value {
                self.emit_bytes(OpCode::OpYieldBlock as u8, pop_count as u8);
            } else {
                self.emit_bytes(OpCode::OpPopN as u8, pop_count as u8);
            }
        }
    }

    fn begin_scope(&mut self) {
        self.frames.last_mut().unwrap().scope_depth += 1;
    }

    fn end_scope(&mut self, with_value: bool) {
        self.frames.last_mut().unwrap().scope_depth -= 1;
        let depth = self.frames.last().unwrap().scope_depth;
        self.discard_locals(depth, true, with_value);
    }

    fn resolve_local(&mut self) -> (bool, u8) {
        let token = self.parser.previous_token;
        let mut found_uninitialized = false;
        let len = self.frames.last().unwrap().locals.len();
        for i in (0..len).rev() {
            let local_token = self.frames.last().unwrap().locals[i].token;
            let local_is_initialized = self.frames.last().unwrap().locals[i].is_initialized;
            if self.same_identifier(local_token, token) {
                if !local_is_initialized {
                    found_uninitialized = true;
                    continue;
                }
                return (true, i as u8);
            }
        }
        if found_uninitialized {
            self.parser.error_at(token, "Can't read local variable in its own initializer");
        }
        (false, 0)
    }

    fn same_identifier(&self, a: Token, b: Token) -> bool {
        a.length == b.length && self.parser.scanner.get_lexeme(a) == self.parser.scanner.get_lexeme(b)
    }

    // ── Variables ─────────────────────────────────────────────────────────────

    fn identifier_constant(&mut self) -> u8 {
        let name = self.parser.scanner.get_lexeme(self.parser.previous_token).to_string();
        let var_name = unsafe { self.vm.as_mut().unwrap().allocate_string(&name) };
        self.current_chunk().write_constant(Value::Object(var_name))
    }

    fn parse_variable(&mut self, message: &str) -> u8 {
        self.parser.consume(TokenType::Identifier, message);
        let token = self.parser.previous_token;
        let scope_depth = self.frames.last().unwrap().scope_depth;
        if scope_depth > 0 {
            self.declare_variable_late(token);
            return 0;
        }
        self.identifier_constant()
    }

    fn mark_initialized(&mut self) {
        if self.frames.last().unwrap().scope_depth == 0 {
            return;
        }
        self.frames.last_mut().unwrap().locals.last_mut().unwrap().is_initialized = true;
    }

    fn define_variable(&mut self, global: u8) {
        if self.frames.last().unwrap().scope_depth > 0 {
            return;
        }
        self.emit_bytes(OpCode::OpDefineGlobal as u8, global);
    }

    fn function(&mut self, function_type: FunctionType) {
        let name = self.parser.scanner.get_lexeme(self.parser.previous_token).to_string();
        let func = unsafe { self.vm.as_mut().unwrap().allocate_function(FunctionObject::new(Chunk::new(), 0, &name)) };
        self.frames.push(crate::compiler::frame::FunctionCompiler::new(func, function_type));
        self.begin_scope();

        self.parser.consume(TokenType::LeftParen, "Expect '(' after function name.");
        if !self.parser.check(TokenType::RightParen) {
            loop {
                unsafe {
                    match &mut (*self.frames.last().unwrap().function).obj_type {
                        ObjectType::Function(f) => f.arity += 1,
                        _ => unreachable!(),
                    }
                }
                let param = self.parse_variable("Expect parameter name.");
                self.define_variable(param);
                self.mark_initialized();
                if !self.parser.match_token_type(TokenType::Comma) {
                    break;
                }
            }
        }
        self.parser.consume(TokenType::RightParen, "Expect ')' after parameters.");
        self.parser.consume(TokenType::LeftBrace, "Expect '{' before function body.");
        self.block();

        let compiled = self.end_compiler();
        let idx = self.current_chunk().write_constant(Value::Object(compiled.unwrap()));
        self.emit_bytes(OpCode::OpConstant as u8, idx);
    }

    // ── Control flow ──────────────────────────────────────────────────────────

    fn case_block(&mut self) {
        self.begin_scope();
        self.parser.consume(TokenType::LeftBrace, "Expected '{' after case.");
        while !self.parser.check(TokenType::Yield) && !self.parser.check(TokenType::EOF) && !self.parser.check(TokenType::RightBrace) {
            self.declaration();
        }
        if self.parser.check(TokenType::Yield) {
            self.parser.advance();
            self.expression();
            self.parser.consume(TokenType::Semicolon, "Expected ';' after yield expression.");
        } else {
            self.emit_byte(OpCode::OpNil as u8);
        }
        self.parser.consume(TokenType::RightBrace, "Expected '}'.");
        self.end_scope(true);
    }

    fn for_statement(&mut self) {
        self.begin_scope();
        let scope_depth = self.frames.last().unwrap().scope_depth;
        self.frames.last_mut().unwrap().jumps.push((scope_depth, 0, Vec::new()));
        self.parser.consume(TokenType::LeftParen, "Expected '(' after 'while'.");
        if self.parser.match_token_type(TokenType::Semicolon) {
        } else if self.parser.match_token_type(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }
        let mut loop_start = self.current_chunk().count();
        self.frames.last_mut().unwrap().jumps.last_mut().unwrap().1 = loop_start;
        let mut is_conditional = false;
        let mut exit_jump = 0;
        if !self.parser.match_token_type(TokenType::Semicolon) {
            is_conditional = true;
            self.expression();
            self.parser.consume(TokenType::Semicolon, "Expected ';' after the loop condition.");
            exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
            self.emit_pop();
        }
        if !self.parser.match_token_type(TokenType::RightParen) {
            let body_jump = self.emit_jump(OpCode::OpJump);
            let increment_start = self.current_chunk().count();
            self.expression();
            self.emit_pop();
            self.parser.consume(TokenType::RightParen, "Expected ')' after clauses.");
            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.frames.last_mut().unwrap().jumps.last_mut().unwrap().1 = loop_start;
            self.patch_jump(body_jump);
        }
        self.statement();
        self.emit_loop(loop_start);
        if is_conditional {
            self.patch_jump(exit_jump);
            self.emit_pop();
        }
        let (_, _, breaks) = self.frames.last_mut().unwrap().jumps.pop().unwrap();
        for break_ in breaks {
            self.patch_jump(break_);
        }
        self.end_scope(false);
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk().count();
        let scope_depth = self.frames.last().unwrap().scope_depth;
        self.frames.last_mut().unwrap().jumps.push((scope_depth, loop_start, Vec::new()));
        self.parser.consume(TokenType::LeftParen, "Expected '(' after 'while'.");
        self.expression();
        self.parser.consume(TokenType::RightParen, "Expected ')' after condition.");
        let exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
        self.emit_pop();
        self.statement();
        self.emit_loop(loop_start);
        self.patch_jump(exit_jump);
        self.emit_pop();
        let (_, _, breaks) = self.frames.last_mut().unwrap().jumps.pop().unwrap();
        for break_ in breaks {
            self.patch_jump(break_);
        }
    }

    fn if_statement(&mut self) {
        self.parser.consume(TokenType::LeftParen, "Expected '(' after 'if'.");
        self.expression();
        self.parser.consume(TokenType::RightParen, "Expected ')' after condition.");
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
        self.parser.consume(TokenType::Semicolon, "Expected ';' after break.");
        let jump_info = self.frames.last_mut().unwrap().jumps.pop();
        if let Some((loop_depth, start, mut jump)) = jump_info {
            self.discard_locals(loop_depth, false, false);
            let emit_jump = self.emit_jump(OpCode::OpJump);
            jump.push(emit_jump);
            self.frames.last_mut().unwrap().jumps.push((loop_depth, start, jump));
        } else {
            let prev = self.parser.previous_token;
            self.parser.error_at(prev, "Break can't be used outside loops");
        }
    }

    fn continue_statement(&mut self) {
        self.parser.consume(TokenType::Semicolon, "Expected ';' after continue.");
        let jump_info = self.frames.last_mut().unwrap().jumps.pop();
        if let Some((loop_depth, start, jump)) = jump_info {
            self.discard_locals(loop_depth, false, false);
            self.emit_loop(start);
            self.frames.last_mut().unwrap().jumps.push((loop_depth, start, jump));
        } else {
            let prev = self.parser.previous_token;
            self.parser.error_at(prev, "Continue can't be used outside loops");
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
            if self.frames.last().unwrap().function_type == FunctionType::TypeScript {
                self.parser.error_at(self.parser.previous_token, "Can't use return outside a function")
            }
            if self.parser.match_token_type(TokenType::Semicolon) {
                self.emit_byte(OpCode::OpNil as u8);
                self.emit_return();
            } else {
                self.expression();
                self.parser.consume(TokenType::Semicolon, "Expect ';' after return value.");
                self.emit_return();
            }
        }
        else if self.parser.match_token_type(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope(false);
        } else {
            self.expression_statement();
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
        self.function(FunctionType::TypeFunction);
        self.define_variable(name);
    }

    fn var_declaration(&mut self) {
        self.parser.consume(TokenType::Identifier, "Expected variable name");
        let token = self.parser.previous_token;
        let scope_depth = self.frames.last().unwrap().scope_depth;
        let constant = if scope_depth == 0 { self.identifier_constant() } else { 0 };

        if self.parser.match_token_type(TokenType::Equal) {
            self.expression();
        } else {
            self.emit_byte(OpCode::OpNil as u8);
        }
        self.parser.consume(TokenType::Semicolon, "Expected ';' after variable declaration.");

        if self.frames.last().unwrap().scope_depth > 0 {
            self.declare_variable_late(token);
            self.mark_initialized();
            return;
        }
        self.emit_bytes(OpCode::OpDefineGlobal as u8, constant);
    }

    fn declare_variable_late(&mut self, token: Token) {
        let scope_depth = self.frames.last().unwrap().scope_depth;
        let len = self.frames.last().unwrap().locals.len();
        for i in (0..len).rev() {
            let local_depth = self.frames.last().unwrap().locals[i].depth;
            let local_token = self.frames.last().unwrap().locals[i].token;
            if local_depth < scope_depth {
                break;
            }
            if self.same_identifier(token, local_token) {
                self.parser.error_at(token, "Already a variable with this name");
                return;
            }
        }
        self.current_frame().add_local(token);
    }

    fn block(&mut self) {
        while !self.parser.check(TokenType::RightBrace) && !self.parser.check(TokenType::EOF) {
            self.declaration();
        }
        self.parser.consume(TokenType::RightBrace, "Expected '}' after block");
    }

    fn print_statement(&mut self) {
        self.expression();
        self.parser.consume(TokenType::Semicolon, "Expected ';' after value.");
        self.emit_byte(OpCode::OpPrint as u8);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.parser.consume(TokenType::Semicolon, "Expected ';' after expression.");
        self.emit_pop();
    }
}
