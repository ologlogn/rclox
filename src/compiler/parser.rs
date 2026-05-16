use super::{Compiler, Local};
use crate::chunk::{Chunk, OpCode};
use crate::compiler::rules::{Precedence, get_rule};
use crate::token::{Token, TokenType};
use crate::value::Value;
use crate::vm::Vm;

impl Compiler {
    pub(super) fn error_at(&mut self, token: Token, message: &str) {
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;
        eprint!("[line {}] Error", token.line);
        match token.token_type {
            TokenType::EOF => eprint!(" at end"),
            TokenType::Error(_) => {}
            _ => {
                eprint!(" at '{}'", self.scanner.get_lexeme(token));
            }
        }
        eprintln!(": {}", message);
        self.had_error = true;
    }
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
            self.error_at(self.current_token, message)
        } else {
            self.advance();
        }
    }
    pub(super) fn declaration(&mut self, chunk: &mut Chunk, vm: &mut Vm) {
        if self.match_token_type(TokenType::Var) {
            self.var_declaration(chunk, vm);
        } else {
            self.statement(chunk, vm);
        }
        if self.panic_mode {
            self.synchronize();
        }
    }
    fn identifier_constant(&mut self, chunk: &mut Chunk, vm: &mut Vm) -> u8 {
        let name = self.scanner.get_lexeme(self.previous_token);
        let var_name = vm.allocate_string(name);
        chunk.write_constant(Value::Object(var_name))
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
            if self.token_equals(token, local.token) {
                self.error_at(token, "Already a variable with this name");
                return;
            }
        }
        self.add_local(token);
    }
    fn add_local(&mut self, token: Token) {
        let local = Local {
            token,
            depth: self.scope_depth,
            is_initialized: false,
        };
        self.locals.push(local);
    }
    fn parse_variable(&mut self, chunk: &mut Chunk, vm: &mut Vm) -> u8 {
        self.consume(TokenType::Identifier, "Expected Variable name");
        self.declare_variable();
        if self.scope_depth > 0 {
            return 0; // a dummy value, not pushed to chunk
        }
        self.identifier_constant(chunk, vm)
    }
    fn var_declaration(&mut self, chunk: &mut Chunk, vm: &mut Vm) {
        let constant = self.parse_variable(chunk, vm);
        if self.match_token_type(TokenType::Equal) {
            self.expression(chunk, vm);
        } else {
            self.emit_byte(OpCode::OpNil as u8, chunk);
        }
        self.consume(TokenType::Semicolon, "Expected Semicolon after declaration.");
        if self.scope_depth > 0 {
            self.locals.last_mut().unwrap().is_initialized = true;
            return;
        }
        self.emit_bytes(OpCode::OpDefineGlobal as u8, constant, chunk);
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
                | TokenType::Return => {
                    return;
                }
                _ => {}
            }
            self.advance()
        }
    }
    fn statement(&mut self, chunk: &mut Chunk, vm: &mut Vm) {
        if self.match_token_type(TokenType::Print) {
            self.print_statement(chunk, vm);
        } else if self.match_token_type(TokenType::LeftBrace) {
            self.begin_scope();
            self.block_statement(chunk, vm);
            self.end_scope(chunk);
        } else {
            self.expression_statement(chunk, vm);
        }
    }
    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }
    fn end_scope(&mut self, chunk: &mut Chunk) {
        self.scope_depth -= 1;
        let mut to_pop = 0;
        while let Some(local) = self.locals.last() {
            if local.depth <= self.scope_depth {
                break;
            }
            self.locals.pop();
            to_pop += 1;
        }
        if to_pop > 0 {
            let c = chunk.write_constant(Value::Number(to_pop as f64));
            self.emit_bytes(OpCode::OpPopN as u8, c, chunk);
        }
    }
    fn block_statement(&mut self, chunk: &mut Chunk, vm: &mut Vm) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::EOF) {
            self.declaration(chunk, vm);
        }
        self.consume(TokenType::RightBrace, "Expected '}' after the block");
    }
    pub(super) fn match_token_type(&mut self, token_type: TokenType) -> bool {
        if !self.check(token_type) {
            false
        } else {
            self.advance();
            true
        }
    }
    fn check(&self, token_type: TokenType) -> bool {
        self.current_token.token_type == token_type
    }
    fn consume_expression(&mut self, chunk: &mut Chunk, vm: &mut Vm) {
        self.expression(chunk, vm);
        self.consume(TokenType::Semicolon, "Expect ';' after statement.");
    }
    fn print_statement(&mut self, chunk: &mut Chunk, vm: &mut Vm) {
        self.consume_expression(chunk, vm);
        self.emit_byte(OpCode::OpPrint as u8, chunk);
    }

    fn expression_statement(&mut self, chunk: &mut Chunk, vm: &mut Vm) {
        self.consume_expression(chunk, vm);
        self.emit_byte(OpCode::OpPop as u8, chunk);
    }

    fn resolve_local(&mut self) -> (bool, u8) {
        let token = self.previous_token;
        let mut found_uninitialized = false;
        for i in (0..self.locals.len()).rev() {
            let local = &self.locals[i];
            if self.token_equals(local.token, token) {
                if !local.is_initialized {
                    found_uninitialized = true;
                    continue;
                }
                return (true, i as u8);
            }
        }
        if found_uninitialized {
            self.error_at(token, "Can't read local variable in its own init");
        }
        (false, 0)
    }
    fn token_equals(&self, a: Token, b: Token) -> bool {
        a.length == b.length && self.scanner.get_lexeme(a) == self.scanner.get_lexeme(b)
    }
    // ======= Pratt parser ==========
    fn parse_precedence(&mut self, precedence: Precedence, chunk: &mut Chunk, vm: &mut Vm) {
        self.advance();
        // prefix
        let can_assign = precedence <= Precedence::Assignment;
        let prefix_rule = get_rule(self.previous_token.token_type).prefix;
        match prefix_rule {
            Some(prefix_fn) => {
                prefix_fn(self, can_assign, chunk, vm);
            }
            None => {
                self.error_at(self.previous_token, "Expect expression.");
                return;
            }
        }
        // infix
        while precedence <= get_rule(self.current_token.token_type).precedence {
            self.advance();
            let infix_rule = get_rule(self.previous_token.token_type).infix;
            if let Some(infix_fn) = infix_rule {
                infix_fn(self, can_assign, chunk, vm);
            }
        }
        if can_assign && self.match_token_type(TokenType::Equal) {
            self.error_at(self.previous_token, "Invalid assignment target.");
            return;
        }
    }
    pub(super) fn expression(&mut self, chunk: &mut Chunk, vm: &mut Vm) {
        self.parse_precedence(Precedence::Assignment, chunk, vm)
    }

    // ======= Functions =========
    pub(super) fn number(&mut self, _can_assign: bool, chunk: &mut Chunk, _vm: &mut Vm) {
        let lexeme = self.scanner.get_lexeme(self.previous_token);
        let val = Value::Number(lexeme.parse().unwrap());
        self.emit_constant(val, chunk);
    }

    pub(super) fn grouping(&mut self, _can_assign: bool, chunk: &mut Chunk, vm: &mut Vm) {
        self.expression(chunk, vm);
        self.consume(TokenType::RightParen, "Expect ')' after expression");
    }
    pub(super) fn unary(&mut self, _can_assign: bool, chunk: &mut Chunk, vm: &mut Vm) {
        let operator_type = self.previous_token.token_type;
        self.parse_precedence(Precedence::Unary, chunk, vm);
        match operator_type {
            TokenType::Minus => self.emit_byte(OpCode::OpNegate as u8, chunk),
            TokenType::Bang => self.emit_byte(OpCode::OpNot as u8, chunk),
            _ => unreachable!("Unknown unary operator"),
        }
    }
    pub(super) fn binary(&mut self, _can_assign: bool, chunk: &mut Chunk, vm: &mut Vm) {
        let operator_type = self.previous_token.token_type;
        let rule = get_rule(operator_type);
        self.parse_precedence(Precedence::try_from(rule.precedence as u8 + 1).unwrap(), chunk, vm);
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
    pub(super) fn literal(&mut self, _can_assign: bool, chunk: &mut Chunk, _vm: &mut Vm) {
        let operator_type = self.previous_token.token_type;
        match operator_type {
            TokenType::Nil => self.emit_byte(OpCode::OpNil as u8, chunk),
            TokenType::True => self.emit_byte(OpCode::OpTrue as u8, chunk),
            TokenType::False => self.emit_byte(OpCode::OpFalse as u8, chunk),
            _ => unreachable!("Unknown literal operator"),
        }
    }
    pub(super) fn string(&mut self, _can_assign: bool, chunk: &mut Chunk, vm: &mut Vm) {
        let lexeme = self.scanner.get_lexeme(self.previous_token);
        let string_value = lexeme[1..lexeme.len() - 1].to_string();
        let obj_ptr = vm.allocate_string(string_value.as_str());
        self.emit_constant(Value::Object(obj_ptr), chunk);
    }
    pub(super) fn identifier(&mut self, can_assign: bool, chunk: &mut Chunk, vm: &mut Vm) {
        let get_op: OpCode;
        let set_op: OpCode;
        let (is_local, mut constant) = self.resolve_local();
        if is_local {
            get_op = OpCode::OpGetLocal;
            set_op = OpCode::OpSetLocal;
        } else {
            get_op = OpCode::OpGetGlobal;
            set_op = OpCode::OpSetGlobal;
            constant = self.identifier_constant(chunk, vm);
        }
        if can_assign && self.match_token_type(TokenType::Equal) {
            self.expression(chunk, vm);
            self.emit_bytes(set_op as u8, constant, chunk);
        } else {
            self.emit_bytes(get_op as u8, constant, chunk);
        }
    }
}
