use super::Compiler;
use crate::token::TokenType;

// ── Precedence ───────────────────────────────────────────────────────────────

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
            _ => Err(()),
        }
    }
}

// ── Parse rules ──────────────────────────────────────────────────────────────

pub type ParseFn = fn(&mut Compiler);

pub struct ParseRule {
    pub prefix: Option<ParseFn>,
    pub infix: Option<ParseFn>,
    pub precedence: Precedence,
}

impl ParseRule {
    fn new(prefix: Option<ParseFn>, infix: Option<ParseFn>, precedence: Precedence) -> Self {
        ParseRule { prefix, infix, precedence }
    }
}

pub fn get_rule(token_type: TokenType) -> ParseRule {
    match token_type {
        TokenType::LeftParen => ParseRule::new(Some(Compiler::grouping), Some(Compiler::call), Precedence::Call),
        TokenType::Bang => ParseRule::new(Some(Compiler::unary), None, Precedence::None),
        TokenType::Minus => ParseRule::new(Some(Compiler::unary), Some(Compiler::binary), Precedence::Term),
        TokenType::Plus => ParseRule::new(None, Some(Compiler::binary), Precedence::Term),
        TokenType::Star | TokenType::Slash => ParseRule::new(None, Some(Compiler::binary), Precedence::Factor),
        TokenType::BangEqual | TokenType::EqualEqual => ParseRule::new(None, Some(Compiler::binary), Precedence::Equality),
        TokenType::Greater | TokenType::GreaterEqual | TokenType::Less | TokenType::LessEqual => {
            ParseRule::new(None, Some(Compiler::binary), Precedence::Comparison)
        }
        TokenType::Number => ParseRule::new(Some(Compiler::number), None, Precedence::None),
        TokenType::String => ParseRule::new(Some(Compiler::string), None, Precedence::None),
        TokenType::Identifier => ParseRule::new(Some(Compiler::variable), None, Precedence::None),
        TokenType::Nil | TokenType::False | TokenType::True => ParseRule::new(Some(Compiler::literal), None, Precedence::None),
        TokenType::And => ParseRule::new(None, Some(Compiler::and_), Precedence::And),
        TokenType::Or => ParseRule::new(None, Some(Compiler::or_), Precedence::Or),
        TokenType::Switch => ParseRule::new(Some(Compiler::switch), None, Precedence::None),
        TokenType::PlusPlus => ParseRule::new(Some(Compiler::prefix_), None, Precedence::Unary),
        TokenType::MinusMinus => ParseRule::new(Some(Compiler::prefix_), None, Precedence::Unary),
        TokenType::LeftBracket => ParseRule::new(Some(Compiler::array), None, Precedence::None),
        TokenType::Array => ParseRule::new(Some(Compiler::make_array), None, Precedence::None),
        TokenType::Len => ParseRule::new(Some(Compiler::len_), None, Precedence::None),
        _ => ParseRule::new(None, None, Precedence::None),
    }
}
