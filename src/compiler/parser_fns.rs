use super::Compiler;
use crate::chunk::{Chunk, OpCode};
use crate::compiler::rules::{Precedence, get_rule};
use crate::scanner::Scanner;
use crate::token::TokenType;
use crate::value::Value;

impl Compiler {
    pub(super) fn number(&mut self, scanner: &mut Scanner, chunk: &mut Chunk) {
        let lexeme = scanner.get_lexeme(self.previous_token);
        let val = Value::Number(lexeme.parse().unwrap());
        self.emit_constant(val, chunk);
    }

    pub(super) fn grouping(&mut self, scanner: &mut Scanner, chunk: &mut Chunk) {
        self.expression(scanner, chunk);
        self.consume(
            TokenType::RightParen,
            "Expect ')' after expression",
            scanner,
        );
    }
    pub(super) fn unary(&mut self, scanner: &mut Scanner, chunk: &mut Chunk) {
        let operator_type = self.previous_token.token_type;
        self.parse_precedence(Precedence::Unary, scanner, chunk);
        match operator_type {
            TokenType::Minus => self.emit_byte(OpCode::OpNegate as u8, chunk),
            TokenType::Bang => self.emit_byte(OpCode::OpNot as u8, chunk),
            _ => unreachable!("Unknown unary operator"),
        }
    }
    pub(super) fn binary(&mut self, scanner: &mut Scanner, chunk: &mut Chunk) {
        let operator_type = self.previous_token.token_type;
        let rule = get_rule(operator_type);
        self.parse_precedence(
            Precedence::try_from(rule.precedence as u8 + 1).unwrap(),
            scanner,
            chunk,
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
    pub(super) fn literal(&mut self, _scanner: &mut Scanner, chunk: &mut Chunk) {
        let operator_type = self.previous_token.token_type;
        match operator_type {
            TokenType::Nil => self.emit_byte(OpCode::OpNil as u8, chunk),
            TokenType::True => self.emit_byte(OpCode::OpTrue as u8, chunk),
            TokenType::False => self.emit_byte(OpCode::OpFalse as u8, chunk),
            _ => unreachable!("Unknown literal operator"),
        }
    }
}
