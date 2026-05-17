use super::{Compiler, Local};
use crate::chunk::{Chunk, OpCode};
use crate::compiler::rules::{Precedence, get_rule};
use crate::token::{Token, TokenType};
use crate::value::Value;

impl Compiler {
    // ── Error handling ──────────────────────────────────────────────────────

    pub(super) fn error_at(&mut self, token: Token, message: &str) {
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

    fn synchronize(&mut self) {
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

    // ── Token navigation ────────────────────────────────────────────────────

    pub(super) fn advance(&mut self) {
        self.previous_token = self.current_token;
        loop {
            self.current_token = self.scanner.next_token();
            if let TokenType::Error(message) = self.current_token.token_type {
                self.error_at(self.current_token, &message);
            } else {
                break;
            }
        }
    }

    pub(super) fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.current_token.token_type != token_type {
            self.error_at(self.current_token, message);
        } else {
            self.advance();
        }
    }

    fn check(&self, token_type: TokenType) -> bool {
        self.current_token.token_type == token_type
    }

    pub(super) fn match_token_type(&mut self, token_type: TokenType) -> bool {
        if !self.check(token_type) {
            false
        } else {
            self.advance();
            true
        }
    }
    // ── Pratt parser ─────────────────────────────────────────────────────────

    fn parse_precedence(&mut self, precedence: Precedence, chunk: &mut Chunk) {
        self.advance();
        let can_assign = precedence <= Precedence::Assignment;
        match get_rule(self.previous_token.token_type).prefix {
            Some(prefix_fn) => prefix_fn(self, can_assign, chunk),
            None => {
                self.error_at(self.previous_token, "Expected expression.");
                return;
            }
        }
        while precedence <= get_rule(self.current_token.token_type).precedence {
            self.advance();
            if let Some(infix_fn) = get_rule(self.previous_token.token_type).infix {
                infix_fn(self, can_assign, chunk);
            }
        }
        if can_assign && self.match_token_type(TokenType::Equal) {
            self.error_at(self.previous_token, "Invalid assignment target.");
        }
    }

    pub(super) fn expression(&mut self, chunk: &mut Chunk) {
        self.parse_precedence(Precedence::Assignment, chunk);
    }

    // ── Parse functions (prefix / infix) ─────────────────────────────────────

    pub(super) fn number(&mut self, _can_assign: bool, chunk: &mut Chunk) {
        let lexeme = self.scanner.get_lexeme(self.previous_token);
        let val = Value::Number(lexeme.parse().unwrap());
        self.emit_constant(val, chunk);
    }

    pub(super) fn string(&mut self, _can_assign: bool, chunk: &mut Chunk) {
        let lexeme = self.scanner.get_lexeme(self.previous_token);
        let string_value = lexeme[1..lexeme.len() - 1].to_string();
        let obj_ptr = unsafe { self.vm.as_mut().unwrap().allocate_string(string_value.as_str()) };
        self.emit_constant(Value::Object(obj_ptr), chunk);
    }

    pub(super) fn variable(&mut self, can_assign: bool, chunk: &mut Chunk) {
        let (is_local, local_idx) = self.resolve_local();
        let (get_op, set_op, arg) = if is_local {
            (OpCode::OpGetLocal, OpCode::OpSetLocal, local_idx)
        } else {
            let global_idx = self.identifier_constant(chunk);
            (OpCode::OpGetGlobal, OpCode::OpSetGlobal, global_idx)
        };
        if can_assign && self.match_token_type(TokenType::Equal) {
            self.expression(chunk);
            self.emit_bytes(set_op as u8, arg, chunk);
        } else {
            self.emit_bytes(get_op as u8, arg, chunk);
        }
    }

    pub(super) fn grouping(&mut self, _can_assign: bool, chunk: &mut Chunk) {
        self.expression(chunk);
        self.consume(TokenType::RightParen, "Expected ')' after expression");
    }

    pub(super) fn unary(&mut self, _can_assign: bool, chunk: &mut Chunk) {
        let operator_type = self.previous_token.token_type;
        self.parse_precedence(Precedence::Unary, chunk);
        match operator_type {
            TokenType::Minus => self.emit_byte(OpCode::OpNegate as u8, chunk),
            TokenType::Bang => self.emit_byte(OpCode::OpNot as u8, chunk),
            _ => unreachable!("Unknown unary operator"),
        }
    }

    pub(super) fn binary(&mut self, _can_assign: bool, chunk: &mut Chunk) {
        let operator_type = self.previous_token.token_type;
        let rule = get_rule(operator_type);
        self.parse_precedence(Precedence::try_from(rule.precedence as u8 + 1).unwrap(), chunk);
        match operator_type {
            TokenType::BangEqual => self.emit_bytes(OpCode::OpEqual as u8, OpCode::OpNot as u8, chunk),
            TokenType::EqualEqual => self.emit_byte(OpCode::OpEqual as u8, chunk),
            TokenType::Greater => self.emit_byte(OpCode::OpGreater as u8, chunk),
            TokenType::Less => self.emit_byte(OpCode::OpLess as u8, chunk),
            TokenType::GreaterEqual => self.emit_bytes(OpCode::OpLess as u8, OpCode::OpNot as u8, chunk),
            TokenType::LessEqual => self.emit_bytes(OpCode::OpGreater as u8, OpCode::OpNot as u8, chunk),
            TokenType::Plus => self.emit_byte(OpCode::OpAdd as u8, chunk),
            TokenType::Minus => self.emit_byte(OpCode::OpSubtract as u8, chunk),
            TokenType::Star => self.emit_byte(OpCode::OpMultiply as u8, chunk),
            TokenType::Slash => self.emit_byte(OpCode::OpDivide as u8, chunk),
            _ => unreachable!("Unknown binary operator"),
        }
    }

    pub(super) fn literal(&mut self, _can_assign: bool, chunk: &mut Chunk) {
        match self.previous_token.token_type {
            TokenType::Nil => self.emit_byte(OpCode::OpNil as u8, chunk),
            TokenType::True => self.emit_byte(OpCode::OpTrue as u8, chunk),
            TokenType::False => self.emit_byte(OpCode::OpFalse as u8, chunk),
            _ => unreachable!("Unknown literal"),
        }
    }

    pub(super) fn and_(&mut self, _can_assign: bool, chunk: &mut Chunk) {
        let end_jump = self.emit_jump(chunk, OpCode::OpJumpIfFalse);
        self.emit_pop(chunk);
        self.parse_precedence(Precedence::And, chunk);
        self.patch_jump(chunk, end_jump);
    }
    pub(super) fn or_(&mut self, _can_assign: bool, chunk: &mut Chunk) {
        let else_jump = self.emit_jump(chunk, OpCode::OpJumpIfFalse);
        let end_jump = self.emit_jump(chunk, OpCode::OpJump);
        self.patch_jump(chunk, else_jump);
        self.emit_pop(chunk);
        self.parse_precedence(Precedence::Or, chunk);
        self.patch_jump(chunk, end_jump);
    }

    // ── Scope & locals ──────────────────────────────────────────────────────

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self, chunk: &mut Chunk) {
        self.scope_depth -= 1;
        let mut pop_count = 0;
        while let Some(local) = self.locals.last() {
            if local.depth <= self.scope_depth {
                break;
            }
            self.locals.pop();
            pop_count += 1;
        }
        if pop_count > 0 {
            let c = chunk.write_constant(Value::Number(pop_count as f64));
            self.emit_bytes(OpCode::OpPopN as u8, c, chunk);
        }
    }

    fn add_local(&mut self, token: Token) {
        self.locals.push(Local {
            token,
            depth: self.scope_depth,
            is_initialized: false,
        });
    }

    fn declare_variable(&mut self) {
        if self.scope_depth == 0 {
            return;
        }
        let token = self.previous_token;
        for local in self.locals.iter().rev() {
            if local.depth < self.scope_depth {
                break;
            }
            if self.same_identifier(token, local.token) {
                self.error_at(token, "Already a variable with this name");
                return;
            }
        }
        self.add_local(token);
    }

    fn resolve_local(&mut self) -> (bool, u8) {
        let token = self.previous_token;
        let mut found_uninitialized = false;
        for i in (0..self.locals.len()).rev() {
            let local = &self.locals[i];
            if self.same_identifier(local.token, token) {
                if !local.is_initialized {
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

    fn same_identifier(&self, a: Token, b: Token) -> bool {
        a.length == b.length && self.scanner.get_lexeme(a) == self.scanner.get_lexeme(b)
    }

    // ── Variables ───────────────────────────────────────────────────────────

    fn identifier_constant(&mut self, chunk: &mut Chunk) -> u8 {
        let name = self.scanner.get_lexeme(self.previous_token);
        let var_name = unsafe { self.vm.as_mut().unwrap().allocate_string(name) };
        chunk.write_constant(Value::Object(var_name))
    }

    fn parse_variable(&mut self, chunk: &mut Chunk) -> u8 {
        self.consume(TokenType::Identifier, "Expected variable name");
        self.declare_variable();
        if self.scope_depth > 0 {
            return 0; // dummy value, locals are not looked up by constant index
        }
        self.identifier_constant(chunk)
    }

    // ── Control flow ────────────────────────────────────────────
    fn for_statement(&mut self, chunk: &mut Chunk) {
        self.begin_scope();
        self.jumps.push((self.scope_depth, 0, Vec::new()));
        self.consume(TokenType::LeftParen, "Expected '(' after 'while'.");
        if self.match_token_type(TokenType::Semicolon) {
        } else if self.match_token_type(TokenType::Var) {
            self.var_declaration(chunk);
        } else {
            self.expression_statement(chunk);
        }
        let mut loop_start = chunk.count();
        self.jumps.last_mut().unwrap().1 = loop_start;
        let mut is_conditional = false;
        // we can't use this to check patch jump, because 0 is valid code. we can't set to -1 because it's unsigned.
        // that's why we use is_conditional
        let mut exit_jump = 0;
        if !self.match_token_type(TokenType::Semicolon) {
            is_conditional = true;
            self.expression(chunk);
            self.consume(TokenType::Semicolon, "Expected ';' after the loop condition.");
            exit_jump = self.emit_jump(chunk, OpCode::OpJumpIfFalse);
            self.emit_pop(chunk);
        }
        if !self.match_token_type(TokenType::RightParen) {
            let body_jump = self.emit_jump(chunk, OpCode::OpJump);
            let increment_start = chunk.count();
            self.expression(chunk);
            self.emit_pop(chunk);
            self.consume(TokenType::RightParen, "Expected ')' after clauses.");
            self.emit_loop(chunk, loop_start);
            loop_start = increment_start;
            self.jumps.last_mut().unwrap().1 = loop_start;
            self.patch_jump(chunk, body_jump);
        }
        self.statement(chunk);
        self.emit_loop(chunk, loop_start);
        if is_conditional {
            self.patch_jump(chunk, exit_jump);
            self.emit_pop(chunk);
        }
        let (_, _, breaks) = self.jumps.pop().unwrap();
        for break_ in breaks {
            self.patch_jump(chunk, break_);
        }
        self.end_scope(chunk);
    }
    fn while_statement(&mut self, chunk: &mut Chunk) {
        let loop_start = chunk.count();
        self.jumps.push((self.scope_depth, loop_start, Vec::new()));
        self.consume(TokenType::LeftParen, "Expected '(' after 'while'.");
        self.expression(chunk);
        self.consume(TokenType::RightParen, "Expected ')' after condition.");
        let exit_jump = self.emit_jump(chunk, OpCode::OpJumpIfFalse);
        self.emit_pop(chunk);
        self.statement(chunk);
        self.emit_loop(chunk, loop_start);
        self.patch_jump(chunk, exit_jump);
        self.emit_pop(chunk);
        let (_, _, breaks) = self.jumps.pop().unwrap();
        for break_ in breaks {
            self.patch_jump(chunk, break_);
        }
    }

    fn if_statement(&mut self, chunk: &mut Chunk) {
        self.consume(TokenType::LeftParen, "Expected '(' after 'if'.");
        self.expression(chunk);
        self.consume(TokenType::RightParen, "Expected ')' after condition.");
        let then_jump = self.emit_jump(chunk, OpCode::OpJumpIfFalse);
        self.emit_pop(chunk);
        self.statement(chunk);
        let else_jump = self.emit_jump(chunk, OpCode::OpJump);
        self.patch_jump(chunk, then_jump);
        self.emit_pop(chunk);
        if self.match_token_type(TokenType::Else) {
            self.statement(chunk);
        }
        self.patch_jump(chunk, else_jump);
    }
    fn break_statement(&mut self, chunk: &mut Chunk) {
        self.consume(TokenType::Semicolon, "Expected ';' after break.");
        let jumps = self.jumps.pop();
        if let Some((loop_depth, start, mut jump)) = jumps {
            let mut pop_count = 0;
            for local in self.locals.iter().rev() {
                if local.depth <= loop_depth {
                    break;
                }
                pop_count += 1;
            }
            if pop_count > 0 {
                let c = chunk.write_constant(Value::Number(pop_count as f64));
                self.emit_bytes(OpCode::OpPopN as u8, c, chunk);
            }
            let emit_jump = self.emit_jump(chunk, OpCode::OpJump);
            jump.push(emit_jump);
            self.jumps.push((loop_depth, start, jump));
        } else {
            self.error_at(self.previous_token, "Break can't be used outside loops");
        }
    }
    fn continue_statement(&mut self, chunk: &mut Chunk) {
        self.consume(TokenType::Semicolon, "Expected ';' after continue.");
        let jumps = self.jumps.pop();
        if let Some((loop_depth, start, jump)) = jumps {
            let mut pop_count = 0;
            for local in self.locals.iter().rev() {
                if local.depth <= loop_depth {
                    break;
                }
                pop_count += 1;
            }
            if pop_count > 0 {
                let c = chunk.write_constant(Value::Number(pop_count as f64));
                self.emit_bytes(OpCode::OpPopN as u8, c, chunk);
            }
            self.emit_loop(chunk, start);
            self.jumps.push((loop_depth, start, jump));
        } else {
            self.error_at(self.previous_token, "Continue can't be used outside loops");
        }
    }
    // ── Statements & declarations ────────────────────────────────────────────
    fn statement(&mut self, chunk: &mut Chunk) {
        if self.match_token_type(TokenType::Print) {
            self.print_statement(chunk);
        } else if self.match_token_type(TokenType::Break) {
            self.break_statement(chunk);
        } else if self.match_token_type(TokenType::Continue) {
            self.continue_statement(chunk);
        } else if self.match_token_type(TokenType::For) {
            self.for_statement(chunk);
        } else if self.match_token_type(TokenType::If) {
            self.if_statement(chunk);
        } else if self.match_token_type(TokenType::While) {
            self.while_statement(chunk)
        } else if self.match_token_type(TokenType::LeftBrace) {
            self.begin_scope();
            self.block(chunk);
            self.end_scope(chunk);
        } else {
            self.expression_statement(chunk);
        }
    }
    pub(super) fn declaration(&mut self, chunk: &mut Chunk) {
        if self.match_token_type(TokenType::Var) {
            self.var_declaration(chunk);
        } else {
            self.statement(chunk);
        }
        if self.panic_mode {
            self.synchronize();
        }
    }

    fn var_declaration(&mut self, chunk: &mut Chunk) {
        let constant = self.parse_variable(chunk);
        if self.match_token_type(TokenType::Equal) {
            self.expression(chunk);
        } else {
            self.emit_byte(OpCode::OpNil as u8, chunk);
        }
        self.consume(TokenType::Semicolon, "Expected ';' after variable declaration.");
        if self.scope_depth > 0 {
            self.locals.last_mut().unwrap().is_initialized = true;
            return;
        }
        self.emit_bytes(OpCode::OpDefineGlobal as u8, constant, chunk);
    }

    fn block(&mut self, chunk: &mut Chunk) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::EOF) {
            self.declaration(chunk);
        }
        self.consume(TokenType::RightBrace, "Expected '}' after block");
    }

    fn print_statement(&mut self, chunk: &mut Chunk) {
        self.expression(chunk);
        self.consume(TokenType::Semicolon, "Expected ';' after value.");
        self.emit_byte(OpCode::OpPrint as u8, chunk);
    }

    fn expression_statement(&mut self, chunk: &mut Chunk) {
        self.expression(chunk);
        self.consume(TokenType::Semicolon, "Expected ';' after expression.");
        self.emit_pop(chunk);
    }
}
