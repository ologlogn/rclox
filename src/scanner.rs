use crate::token::{Token, TokenType};

pub struct Scanner {
    source: String,
    start: usize,
    current: usize,
    line: usize,
}
impl Scanner {
    pub fn new(source: String) -> Scanner {
        Scanner {
            source,
            start: 0,
            current: 0,
            line: 1,
        }
    }
}
fn is_digit(c: char) -> bool {
    c >= '0' && c <= '9'
}
fn is_alpha(c: char) -> bool {
    (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || c == '_'
}
fn is_alphanumeric(c: char) -> bool {
    is_alpha(c) || is_digit(c)
}

impl Scanner {
    pub fn get_lexeme(&self, token: Token) -> &str {
        &self.source[token.start..token.start + token.length]
    }
    fn char_at_nth(&self, start: usize, offset: usize) -> char {
        self.source
            .get(start..)
            .and_then(|slice| slice.chars().nth(offset))
            .unwrap_or('\0')
    }

    fn peek(&self) -> char {
        self.char_at_nth(self.current, 0)
    }

    fn peek_next(&self) -> char {
        self.char_at_nth(self.current, 1)
    }

    fn advance(&mut self) -> char {
        let c = self.peek();
        self.current += c.len_utf8();
        c
    }

    fn if_next_char_then_advance(&mut self, expected: char) -> bool {
        if self.peek() != expected {
            return false;
        }
        self.advance();
        true
    }

    pub fn skip_whitespaces(&mut self) {
        loop {
            let c = self.peek();
            match c {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                '/' => {
                    if self.peek_next() == '/' {
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance(); // ignore comments
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            }
        }
    }

    fn number(&mut self) -> Token {
        while is_digit(self.peek()) {
            self.advance();
        }
        if self.peek() == '.' && is_digit(self.peek_next()) {
            self.advance();
            while is_digit(self.peek()) {
                self.advance();
            }
        }
        self.make_token(TokenType::Number)
    }

    fn identifier(&mut self) -> Token {
        while is_alphanumeric(self.peek()) {
            self.advance();
        }
        let token_type = match self.char_at_nth(self.start, 0) {
            'a' => self.check_keyword(1, "nd", TokenType::And),
            'c' => self.check_keyword(1, "lass", TokenType::Class),
            'e' => self.check_keyword(1, "lse", TokenType::Else),
            'i' => self.check_keyword(1, "f", TokenType::If),
            'n' => self.check_keyword(1, "il", TokenType::Nil),
            'o' => self.check_keyword(1, "r", TokenType::Or),
            'p' => self.check_keyword(1, "rint", TokenType::Print),
            'r' => self.check_keyword(1, "eturn", TokenType::Return),
            's' => self.check_keyword(1, "uper", TokenType::Super),
            'v' => self.check_keyword(1, "ar", TokenType::Var),
            'w' => self.check_keyword(1, "hile", TokenType::While),
            'f' => {
                if self.current - self.start > 1 {
                    match self.char_at_nth(self.start, 1) {
                        'a' => self.check_keyword(2, "lse", TokenType::False),
                        'o' => self.check_keyword(2, "r", TokenType::For),
                        'u' => self.check_keyword(2, "n", TokenType::Fun),
                        _ => TokenType::Identifier,
                    }
                } else {
                    TokenType::Identifier
                }
            }
            't' => {
                if self.current - self.start > 1 {
                    match self.char_at_nth(self.start, 1) {
                        'h' => self.check_keyword(2, "is", TokenType::This),
                        'r' => self.check_keyword(2, "ue", TokenType::True),
                        _ => TokenType::Identifier,
                    }
                } else {
                    TokenType::Identifier
                }
            }
            _ => TokenType::Identifier,
        };
        self.make_token(token_type)
    }

    fn check_keyword(&self, start_offset: usize, rest: &str, token_type: TokenType) -> TokenType {
        let check_start = self.start + start_offset;
        let check_end = check_start + rest.len();
        if check_end <= self.source.len() && self.current == check_end && &self.source[check_start..check_end] == rest {
            token_type
        } else {
            TokenType::Identifier
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespaces();
        self.start = self.current;
        if self.is_at_end() {
            self.make_token(TokenType::EOF)
        } else {
            let c = self.advance();
            if is_digit(c) {
                return self.number();
            }
            if is_alpha(c) {
                return self.identifier();
            }
            match c {
                '(' => self.make_token(TokenType::LeftParen),
                ')' => self.make_token(TokenType::RightParen),
                '{' => self.make_token(TokenType::LeftBrace),
                '}' => self.make_token(TokenType::RightBrace),
                ',' => self.make_token(TokenType::Comma),
                '.' => self.make_token(TokenType::Dot),
                '-' => self.make_token(TokenType::Minus),
                '+' => self.make_token(TokenType::Plus),
                ';' => self.make_token(TokenType::Semicolon),
                '*' => self.make_token(TokenType::Star),
                '/' => self.make_token(TokenType::Slash),
                '!' => {
                    if self.if_next_char_then_advance('=') {
                        self.make_token(TokenType::BangEqual)
                    } else {
                        self.make_token(TokenType::Bang)
                    }
                }
                '=' => {
                    if self.if_next_char_then_advance('=') {
                        self.make_token(TokenType::EqualEqual)
                    } else {
                        self.make_token(TokenType::Equal)
                    }
                }
                '>' => {
                    if self.if_next_char_then_advance('=') {
                        self.make_token(TokenType::GreaterEqual)
                    } else {
                        self.make_token(TokenType::Greater)
                    }
                }
                '<' => {
                    if self.if_next_char_then_advance('=') {
                        self.make_token(TokenType::LessEqual)
                    } else {
                        self.make_token(TokenType::Less)
                    }
                }
                '"' => self.string(),
                _ => self.make_error_token("Unexpected Character"),
            }
        }
    }
    fn string(&mut self) -> Token {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }
        if self.is_at_end() {
            self.make_error_token("Unterminated string")
        } else {
            self.advance();
            self.make_token(TokenType::String)
        }
    }
    pub fn make_error_token(&mut self, message: &'static str) -> Token {
        Token {
            token_type: TokenType::Error(message),
            start: self.start,
            length: self.current - self.start,
            line: self.line,
        }
    }
    pub fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }
    fn make_token(&self, token_type: TokenType) -> Token {
        Token {
            token_type,
            start: self.start,
            length: self.current - self.start,
            line: self.line,
        }
    }
}
