use crate::scanner::Scanner;
use crate::token::{Token, TokenType};

// ── Parser ───────────────────────────────────────────────────────────────────

pub struct Parser {
    pub current_token: Token,
    pub previous_token: Token,
    pub scanner: Scanner,
    pub had_error: bool,
    pub panic_mode: bool,
    pub can_assign: bool,
}

impl Parser {
    pub fn new(scanner: Scanner) -> Self {
        Parser {
            current_token: Token::default(),
            previous_token: Token::default(),
            scanner,
            had_error: false,
            panic_mode: false,
            can_assign: false,
        }
    }

    pub fn error_at(&mut self, token: Token, message: &str) {
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

    pub fn synchronize(&mut self) {
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

    pub fn advance(&mut self) {
        self.previous_token = self.current_token;
        loop {
            self.current_token = self.scanner.next_token();
            if let TokenType::Error(message) = self.current_token.token_type {
                let token = self.current_token;
                self.error_at(token, message);
            } else {
                break;
            }
        }
    }

    pub fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.current_token.token_type != token_type {
            let token = self.current_token;
            self.error_at(token, message);
        } else {
            self.advance();
        }
    }

    pub fn check(&self, token_type: TokenType) -> bool {
        self.current_token.token_type == token_type
    }

    pub fn match_token_type(&mut self, token_type: TokenType) -> bool {
        if !self.check(token_type) {
            false
        } else {
            self.advance();
            true
        }
    }
}
