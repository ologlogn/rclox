use crate::closure::CompilerUpvalue;
use crate::function::FunctionType;
use crate::token::{Token, TokenType};
use crate::value::Object;

pub struct Local {
    pub token: Token,
    pub depth: usize,
    pub is_initialized: bool,
    pub is_captured: bool,
}

pub struct FunctionCompiler {
    pub locals: Vec<Local>,
    pub scope_depth: usize,
    pub jumps: Vec<(usize, usize, Vec<usize>)>,
    pub function: *mut Object,
    pub function_type: FunctionType,
    pub upvalues: Vec<CompilerUpvalue>,
    pub upvalue_count: usize,
}

impl FunctionCompiler {
    pub fn new(function: *mut Object, function_type: FunctionType) -> Self {
        let locals = vec![Local {
            token: Token {
                token_type: TokenType::Identifier,
                length: 0,
                start: 0,
                line: 0,
            },
            depth: 0,
            is_initialized: true,
            is_captured: false,
        }];
        FunctionCompiler {
            locals,
            scope_depth: 0,
            jumps: Vec::new(),
            function,
            function_type,
            upvalues: vec![],
            upvalue_count: 0,
        }
    }

    pub fn add_local(&mut self, token: Token) {
        self.locals.push(Local {
            token,
            depth: self.scope_depth,
            is_initialized: false,
            is_captured: false,
        });
    }
}
