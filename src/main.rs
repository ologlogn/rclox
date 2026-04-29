use crate::chunk::{Chunk, OpCode};
use crate::value::Value;
use crate::vm::Vm;

mod chunk;
mod operation;
mod value;
mod vm;

fn main() {
    let mut chunk = Chunk::new();
    chunk.write_byte(OpCode::OpConstant as u8, 1);
    let constant = chunk.write_constant(Value::Number(3.14));
    chunk.write_byte(constant as u8, 1);

    chunk.write_byte(OpCode::OpNegate as u8, 1);

    chunk.write_byte(OpCode::OpConstant as u8, 1);
    let constant = chunk.write_constant(Value::Number(1234f64));
    chunk.write_byte(constant as u8, 1);

    chunk.write_byte(OpCode::OpMultiply as u8, 1);
    chunk.write_byte(OpCode::OpReturn as u8, 1);

    let mut vm = Vm::new();
    println!("{:?}", chunk);
    vm.interpret(chunk);
}
