use crate::chunk::{Chunk, OpCode};
use crate::rules::{Precedence, get_rule};
use crate::scanner::Scanner;
use crate::token::{Token, TokenType};
use crate::value::Value;

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

    pub fn parse_precedence(
        &mut self,
        precedence: Precedence,
        scanner: &mut Scanner,
        chunk: &mut Chunk,
    ) {
        self.advance(scanner);
        // prefix
        let prefix_rule = get_rule(self.previous_token.token_type).prefix;
        match prefix_rule {
            Some(prefix_fn) => {
                prefix_fn(self, scanner, chunk);
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
                infix_fn(self, scanner, chunk);
            }
        }
    }
    fn emit_constant(&mut self, val: Value, chunk: &mut Chunk) {
        let constant = chunk.write_constant(val);
        self.emit_bytes(OpCode::OpConstant as u8, constant as u8, chunk);
    }
    pub fn number(&mut self, scanner: &mut Scanner, chunk: &mut Chunk) {
        let lexeme = scanner.get_lexeme(self.previous_token);
        let val = Value::Number(lexeme.parse().unwrap());
        self.emit_constant(val, chunk);
    }

    pub fn grouping(&mut self, scanner: &mut Scanner, chunk: &mut Chunk) {
        self.expression(scanner, chunk);
        self.consume(
            TokenType::RightParen,
            "Expect ')' after expression",
            scanner,
        );
    }
    pub fn unary(&mut self, scanner: &mut Scanner, chunk: &mut Chunk) {
        let operator_type = self.previous_token.token_type;
        self.parse_precedence(Precedence::Unary, scanner, chunk);
        if operator_type == TokenType::Minus {
            self.emit_byte(OpCode::OpNegate as u8, chunk)
        }
    }
    pub fn binary(&mut self, scanner: &mut Scanner, chunk: &mut Chunk) {
        let operator_type = self.previous_token.token_type;
        let rule = get_rule(operator_type);
        self.parse_precedence(
            Precedence::try_from(rule.precedence as u8 + 1).unwrap(),
            scanner,
            chunk,
        );
        match operator_type {
            TokenType::Plus => self.emit_byte(OpCode::OpAdd as u8, chunk),
            TokenType::Minus => self.emit_byte(OpCode::OpSubtract as u8, chunk),
            TokenType::Star => self.emit_byte(OpCode::OpMultiply as u8, chunk),
            TokenType::Slash => self.emit_byte(OpCode::OpDivide as u8, chunk),
            _ => unreachable!("Unknown binary operator"),
        }
    }
    pub fn emit_byte(&self, byte: u8, chunk: &mut Chunk) {
        chunk.write_byte(byte, self.previous_token.line);
    }
    pub fn emit_bytes(&self, byte1: u8, byte2: u8, chunk: &mut Chunk) {
        self.emit_byte(byte1, chunk);
        self.emit_byte(byte2, chunk);
    }
    pub fn compile(&mut self, scanner: &mut Scanner, chunk: &mut Chunk) -> bool {
        self.advance(scanner);
        self.expression(scanner, chunk);
        self.consume(TokenType::EOF, "Expect end of expression", scanner);
        self.end(chunk);
        !self.had_error
    }
    fn end(&self, chunk: &mut Chunk) {
        self.emit_return(chunk);
    }

    fn emit_return(&self, chunk: &mut Chunk) {
        self.emit_byte(OpCode::OpReturn as u8, chunk)
    }
    fn consume(&mut self, token_type: TokenType, message: &str, scanner: &mut Scanner) {
        if self.current_token.token_type == token_type {
            self.advance(scanner);
            return;
        }
        self.error_at(self.current_token, message, scanner)
    }
    fn error_at(&mut self, token: Token, message: &str, scanner: &Scanner) {
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
    fn advance(&mut self, scanner: &mut Scanner) {
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
    fn expression(&mut self, scanner: &mut Scanner, chunk: &mut Chunk) {
        self.parse_precedence(Precedence::Assignment, scanner, chunk)
    }
}