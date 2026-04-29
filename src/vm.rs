use crate::chunk::{Chunk, OpCode};
use crate::operation::{add, div, mul, sub};
use crate::value::Value;
use std::fmt::{Debug, Write};

pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

pub struct Vm {
    chunk: Option<Chunk>, // assign this before doing interpret.
    ip: usize,
    stack: Vec<Value>,
}
impl Debug for Vm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "        ")?;
        for v in self.stack.iter() {
            write!(f, "[ ")?;
            write!(f, "{:?}", v)?;
            write!(f, " ]")?;
        }
        write!(f, "\n")
    }
}

impl Vm {
    pub fn new() -> Vm {
        Vm {
            chunk: None,
            ip: 0, // instruction pointer
            stack: Vec::new(),
        }
    }
    pub fn interpret(&mut self, chunk: Chunk) -> InterpretResult {
        self.chunk = Some(chunk);
        self.ip = 0;
        self.stack.clear();
        self.run()
    }

    fn read_byte(&mut self) -> u8 {
        let b = self.chunk.as_ref().unwrap().read_byte(self.ip); // read only
        self.ip += 1;
        b
    }
    fn read_constant(&mut self) -> Value {
        let b = self.read_byte();
        self.chunk.as_ref().unwrap().read_constant(b as usize)
    }

    fn run(&mut self) -> InterpretResult {
        loop {
            match OpCode::from(self.read_byte()) {
                OpCode::OpReturn => {
                    println!("{:?}", self.stack.pop().unwrap());
                    return InterpretResult::InterpretOk;
                }
                OpCode::OpConstant => {
                    let value = self.read_constant();
                    self.stack.push(value);
                }
                OpCode::OpNegate => {
                    if let Some(Value::Number(number)) = self.stack.pop() {
                        self.stack.push(Value::Number(-number));
                    } else {
                        return InterpretResult::InterpretRuntimeError;
                    }
                }
                OpCode::OpAdd => self.binary_op(add),
                OpCode::OpSubtract => self.binary_op(sub),
                OpCode::OpMultiply => self.binary_op(mul),
                OpCode::OpDivide => self.binary_op(div),
            }
        }
    }
    fn binary_op(&mut self, op: fn(Value, Value) -> Value) {
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        self.stack.push(op(a, b));
    }
}
