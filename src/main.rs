use crate::chunk::{Chunk, OpCode};
use crate::value::Value;

pub mod chunk;
mod value;

fn main() {
    let mut chunk = Chunk::new();
    chunk.write_byte(OpCode::OpConstant as u8, 1);
    let constant = chunk.write_constant(Value::Number(3.14));
    chunk.write_byte(constant as u8, 1);

    chunk.write_byte(OpCode::OpConstant as u8, 1);
    let constant = chunk.write_constant(Value::Bool(true));
    chunk.write_byte(constant as u8, 1);

    chunk.write_byte(OpCode::OpReturn as u8, 2);
    chunk.write_byte(OpCode::OpReturn as u8, 2);
    println!("{:?}", chunk);
}
