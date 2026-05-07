use crate::scanner::Scanner;
use crate::token::{Token, TokenType};

pub fn compile(source: &str) {
    let mut scanner = Scanner::new(source);
    let mut line = 0;
    loop {
        let token = scanner.next_token();
        if token.line != line {
            print!("{:4} ", token.line);
            line = token.line;
        } else {
            print!("   | ")
        }
        let lexeme = &source[token.start..token.start + token.length];
        println!("{:?} {:?}", token.token_type, lexeme);

        if token.token_type == TokenType::EOF {
            break;
        }
    }
}
