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
        self.statement(scanner, chunk, vm);
        if self.panic_mode {
            self.synchronize();
        }
    }
    fn synchronize(&mut self) {}
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
        let prefix_rule = get_rule(self.previous_token.token_type).prefix;
        match prefix_rule {
            Some(prefix_fn) => {
                prefix_fn(self, scanner, chunk, vm);
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
                infix_fn(self, scanner, chunk, vm);
            }
        }
    }
    pub(super) fn expression(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        self.parse_precedence(Precedence::Assignment, scanner, chunk, vm)
    }

    // ======= Pratt Parser functions  =========
    pub(super) fn number(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, _vm: &mut Vm) {
        let lexeme = scanner.get_lexeme(self.previous_token);
        let val = Value::Number(lexeme.parse().unwrap());
        self.emit_constant(val, chunk);
    }

    pub(super) fn grouping(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        self.expression(scanner, chunk, vm);
        self.consume(TokenType::RightParen, "Expect ')' after expression", scanner);
    }
    pub(super) fn unary(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        let operator_type = self.previous_token.token_type;
        self.parse_precedence(Precedence::Unary, scanner, chunk, vm);
        match operator_type {
            TokenType::Minus => self.emit_byte(OpCode::OpNegate as u8, chunk),
            TokenType::Bang => self.emit_byte(OpCode::OpNot as u8, chunk),
            _ => unreachable!("Unknown unary operator"),
        }
    }
    pub(super) fn binary(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        let operator_type = self.previous_token.token_type;
        let rule = get_rule(operator_type);
        self.parse_precedence(
            Precedence::try_from(rule.precedence as u8 + 1).unwrap(),
            scanner,
            chunk,
            vm,
        );
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
    pub(super) fn literal(&mut self, _scanner: &mut Scanner, chunk: &mut Chunk, _vm: &mut Vm) {
        let operator_type = self.previous_token.token_type;
        match operator_type {
            TokenType::Nil => self.emit_byte(OpCode::OpNil as u8, chunk),
            TokenType::True => self.emit_byte(OpCode::OpTrue as u8, chunk),
            TokenType::False => self.emit_byte(OpCode::OpFalse as u8, chunk),
            _ => unreachable!("Unknown literal operator"),
        }
    }
    pub(super) fn string(&mut self, scanner: &mut Scanner, chunk: &mut Chunk, vm: &mut Vm) {
        let lexeme = scanner.get_lexeme(self.previous_token);
        let string_value = lexeme[1..lexeme.len() - 1].to_string();
        let obj_ptr = vm.allocate_string(string_value.as_str());
        self.emit_constant(Value::Object(obj_ptr), chunk);
    }
}
