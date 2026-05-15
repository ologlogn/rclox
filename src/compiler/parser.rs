use super::Compiler;
use crate::chunk::{Chunk, OpCode};
use crate::compiler::rules::{Precedence, get_rule};
use crate::scanner::Scanner;
use crate::token::{Token, TokenType};
use crate::value::Value;
use crate::vm::Vm;

impl Compiler {
    pub(super) fn error_at(&mut self, token: Token, message: &str, scanner: &Scanner) {
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;
        eprint!("[line {}] Error", token.line);
        match token.token_type {
            TokenType::EOF => eprint!(" at end"),
            TokenType::Error(_) => {}
            _ => {
                eprint!(" at '{}'", scanner.get_lexeme(token));
            }
        }
        eprintln!(": {}", message);
        self.had_error = true;
    }
    pub(super) fn advance(&mut self, scanner: &mut Scanner) {
        self.previous_token = self.current_token;
        loop {
            self.current_token = scanner.next_token();
            if let TokenType::Error(message) = self.current_token.token_type {
                self.error_at(self.current_token, &message, scanner);
            } else {
                break;
            }
        }
    }
    pub(super) fn consume(&mut self, token_type: TokenType, message: &str, scanner: &mut Scanner) {
        if self.current_token.token_type != token_type {
            self.error_at(self.current_token, message, scanner)
        } else {
            self.advance(scanner);
        }
    }
    pub(super) fn declaration(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        if self.match_token_type(TokenType::Var, scanner) {
            self.var_declaration(scanner, chunk, vm);
        } else {
            self.statement(scanner, chunk, vm);
        }
        if self.panic_mode {
            self.synchronize(scanner);
        }
    }
    fn identifier_constant(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) -> u8 {
        let name = scanner.get_lexeme(self.previous_token);
        let var_name = vm.allocate_string(name);
        chunk.write_constant(Value::Object(var_name))
    }
    fn parse_variable(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) -> u8 {
        self.consume(TokenType::Identifier, "Expected Variable name", scanner);
        self.identifier_constant(scanner, chunk, vm)
    }
    fn var_declaration(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        let constant = self.parse_variable(scanner, chunk, vm);
        if self.match_token_type(TokenType::Equal, scanner) {
            self.expression(scanner, chunk, vm);
        } else {
            self.emit_byte(OpCode::OpNil as u8, chunk);
        }
        self.consume(TokenType::Semicolon, "Expected Semicolon after declaration.", scanner);
        self.emit_bytes(OpCode::OpDefineGlobal as u8, constant, chunk);
    }
    fn synchronize(&mut self, scanner: &mut Scanner) {
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
            self.advance(scanner)
        }
    }
    fn statement(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        if self.match_token_type(TokenType::Print, scanner) {
            self.print_statement(scanner, chunk, vm);
        } else {
            self.expression_statement(scanner, chunk, vm);
        }
    }

    pub(super) fn match_token_type(&mut self, token_type: TokenType, scanner: &mut Scanner) -> bool {
        if !self.check(token_type) {
            false
        } else {
            self.advance(scanner);
            true
        }
    }
    fn check(&self, token_type: TokenType) -> bool {
        self.current_token.token_type == token_type
    }
    fn print_statement(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        self.expression(scanner, chunk, vm);
        self.consume(TokenType::Semicolon, "Expect ';' after statement.", scanner);
        self.emit_byte(OpCode::OpPrint as u8, chunk);
    }

    fn expression_statement(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        self.expression(scanner, chunk, vm);
        self.consume(TokenType::Semicolon, "Expect ';' after statement.", scanner);
        self.emit_byte(OpCode::OpPop as u8, chunk);
    }

    fn parse_precedence(&mut self, precedence: Precedence, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        self.advance(scanner);
        // prefix
        let can_assign = precedence <= Precedence::Assignment;
        let prefix_rule = get_rule(self.previous_token.token_type).prefix;
        match prefix_rule {
            Some(prefix_fn) => {
                prefix_fn(self, can_assign, scanner, chunk, vm);
            }
            None => {
                self.error_at(self.previous_token, "Expect expression.", scanner);
                return;
            }
        }
        // infix
        while precedence <= get_rule(self.current_token.token_type).precedence {
            self.advance(scanner);
            let infix_rule = get_rule(self.previous_token.token_type).infix;
            if let Some(infix_fn) = infix_rule {
                infix_fn(self, can_assign, scanner, chunk, vm);
            }
        }
        if can_assign && self.match_token_type(TokenType::Equal, scanner) {
            self.error_at(self.previous_token, "Invalid assignment target.", scanner);
        }
    }
    pub(super) fn expression(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        self.parse_precedence(Precedence::Assignment, scanner, chunk, vm)
    }

    // ======= Pratt Parser functions  =========
    pub(super) fn number(&mut self, _can_assign: bool, scanner: &mut Scanner, chunk: &mut Chunk, _vm: &mut Vm) {
        let lexeme = scanner.get_lexeme(self.previous_token);
        let val = Value::Number(lexeme.parse().unwrap());
        self.emit_constant(val, chunk);
    }

    pub(super) fn grouping(&mut self, _can_assign: bool, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        self.expression(scanner, chunk, vm);
        self.consume(TokenType::RightParen, "Expect ')' after expression", scanner);
    }
    pub(super) fn unary(&mut self, _can_assign: bool, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        let operator_type = self.previous_token.token_type;
        self.parse_precedence(Precedence::Unary, scanner, chunk, vm);
        match operator_type {
            TokenType::Minus => self.emit_byte(OpCode::OpNegate as u8, chunk),
            TokenType::Bang => self.emit_byte(OpCode::OpNot as u8, chunk),
            _ => unreachable!("Unknown unary operator"),
        }
    }
    pub(super) fn binary(&mut self, _can_assign: bool, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        let operator_type = self.previous_token.token_type;
        let rule = get_rule(operator_type);
        self.parse_precedence(Precedence::try_from(rule.precedence as u8 + 1).unwrap(), scanner, chunk, vm);
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
    pub(super) fn literal(&mut self, _can_assign: bool, _scanner: &mut Scanner, chunk: &mut Chunk, _vm: &mut Vm) {
        let operator_type = self.previous_token.token_type;
        match operator_type {
            TokenType::Nil => self.emit_byte(OpCode::OpNil as u8, chunk),
            TokenType::True => self.emit_byte(OpCode::OpTrue as u8, chunk),
            TokenType::False => self.emit_byte(OpCode::OpFalse as u8, chunk),
            _ => unreachable!("Unknown literal operator"),
        }
    }
    pub(super) fn string(&mut self, _can_assign: bool, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        let lexeme = scanner.get_lexeme(self.previous_token);
        let string_value = lexeme[1..lexeme.len() - 1].to_string();
        let obj_ptr = vm.allocate_string(string_value.as_str());
        self.emit_constant(Value::Object(obj_ptr), chunk);
    }
    pub(super) fn identifier(&mut self, can_assign: bool, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        let constant = self.identifier_constant(scanner, chunk, vm);
        if can_assign && self.match_token_type(TokenType::Equal, scanner) {
            self.expression(scanner, chunk, vm);
            self.emit_bytes(OpCode::OpSetGlobal as u8, constant, chunk);
        } else {
            self.emit_bytes(OpCode::OpGetGlobal as u8, constant, chunk);
        }
    }
}
