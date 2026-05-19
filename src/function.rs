use crate::chunk::Chunk;
use crate::value::{Object};

pub struct FunctionObject {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: String,
}

impl FunctionObject {
    pub fn new(chunk: Chunk, arity: usize, name: &str) -> FunctionObject {
        FunctionObject {
            arity,
            chunk,
            name: name.to_string(),
        }
    }
}
pub enum FunctionType {
    TypeFunction,
    TypeScript,
}

pub struct CallFrame {
    pub function: *mut Object,
    pub ip: usize,
    pub stack_base: usize,
}
