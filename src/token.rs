#[derive(Debug, Copy, Clone, Default, PartialOrd, PartialEq)]
pub enum TokenType {
    // Single Character tokens
    #[default]
    None,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    PlusPlus,
    MinusMinus,

    // One or two character tokens
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    EqualGreater,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    //Literals
    Identifier,
    String,
    Number,

    //Keywords
    And,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    Break,
    Continue,
    Switch,
    Case,
    Default,
    Yield,
    Array,
    Len,
    Error(&'static str), // a constant message at the compile time
    EOF,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Token {
    pub token_type: TokenType,
    pub start: usize,
    pub length: usize,
    pub line: usize,
}
