use super::Compiler;
use crate::chunk::Chunk;
use crate::compiler::rules::{Precedence, get_rule};
use crate::scanner::Scanner;
use crate::token::{Token, TokenType};

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

    pub(super) fn parse_precedence(
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
    pub(super) fn expression(&mut self, scanner: &mut Scanner, chunk: &mut Chunk) {
        self.parse_precedence(Precedence::Assignment, scanner, chunk)
    }
}
