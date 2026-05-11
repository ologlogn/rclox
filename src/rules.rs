use crate::chunk::Chunk;
use crate::compiler::Compiler;
use crate::scanner::Scanner;
use crate::token::TokenType;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Precedence {
    None,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}
impl TryFrom<u8> for Precedence {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Precedence::None),
            1 => Ok(Precedence::Assignment),
            2 => Ok(Precedence::Or),
            3 => Ok(Precedence::And),
            4 => Ok(Precedence::Equality),
            5 => Ok(Precedence::Comparison),
            6 => Ok(Precedence::Term),
            7 => Ok(Precedence::Factor),
            8 => Ok(Precedence::Unary),
            9 => Ok(Precedence::Call),
            10 => Ok(Precedence::Primary),
            _ => Err(()), // Fails if the number is out of bounds
        }
    }
}

pub type ParseFn = fn(&mut Compiler, &mut Scanner, &mut Chunk);

pub struct ParseRule {
    pub prefix: Option<ParseFn>,
    pub infix: Option<ParseFn>,
    pub precedence: Precedence,
}

pub fn get_rule(token_type: TokenType) -> ParseRule {
    println!("GET RULE {:?}", token_type);
    match token_type {
        TokenType::LeftParen => ParseRule {
            prefix: Some(Compiler::grouping),
            infix: None,
            precedence: Precedence::None,
        },
        TokenType::Minus => ParseRule {
            prefix: Some(Compiler::unary),
            infix: Some(Compiler::binary),
            precedence: Precedence::Term,
        },
        TokenType::Plus => ParseRule {
            prefix: None,
            infix: Some(Compiler::binary),
            precedence: Precedence::Term,
        },
        TokenType::Star | TokenType::Slash => ParseRule {
            prefix: None,
            infix: Some(Compiler::binary),
            precedence: Precedence::Factor,
        },
        TokenType::Number => ParseRule {
            prefix: Some(Compiler::number),
            infix: None,
            precedence: Precedence::None,
        },
        _ => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
    }
}
