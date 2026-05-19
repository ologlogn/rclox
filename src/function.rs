use crate::chunk::Chunk;

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
#[derive(PartialEq, Debug)]
pub enum FunctionType {
    TypeFunction,
    TypeScript,
}

pub struct CallFrame {
    pub function: *mut FunctionObject,
    pub ip: usize,
    pub stack_base: usize,
}
