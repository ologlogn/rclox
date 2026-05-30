use crate::chunk::Chunk;

pub struct FunctionObject {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: String,
    pub upvalue_count: usize,
}

impl FunctionObject {
    pub fn new(chunk: Chunk, arity: usize, name: &str) -> FunctionObject {
        FunctionObject {
            arity,
            chunk,
            name: name.to_string(),
            upvalue_count: 0,
        }
    }
}
#[derive(PartialEq, Debug)]
pub enum FunctionType {
    TypeFunction,
    TypeScript,
}
