use crate::chunk::{Chunk, OpCode};
use crate::value::Value;
use std::fmt::Debug;

pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

pub struct Vm {
    ip: usize,
    stack: Vec<Value>,
}

impl Vm {
    fn binary_op<F>(&mut self, op: F) -> Result<(), &'static str>
    where
        F: FnOnce(&Value, &Value) -> Result<Value, &'static str>,
    {
        let len = self.stack.len();
        if len < 2 {
            return Err("Stack underflow");
        }
        let a = &self.stack[len - 2];
        let b = &self.stack[len - 1];
        let result = op(a, b)?;
        self.stack.truncate(len - 2);
        self.stack.push(result);
        Ok(())
    }
    pub fn new() -> Vm {
        Vm {
            ip: 0, // instruction pointer
            stack: Vec::new(),
        }
    }
    pub fn interpret(&mut self, chunk: &Chunk) -> InterpretResult {
        self.ip = 0;
        self.stack.clear();
        self.run(chunk)
    }

    fn read_byte(&mut self, chunk: &Chunk) -> u8 {
        let b = chunk.read_byte(self.ip); // read only
        self.ip += 1;
        b
    }
    fn read_constant(&mut self, chunk: &Chunk) -> Value {
        let b = self.read_byte(chunk);
        chunk.read_constant(b as usize)
    }

    fn run(&mut self, chunk: &Chunk) -> InterpretResult {
        loop {
            match OpCode::try_from(self.read_byte(chunk)).unwrap() {
                OpCode::OpReturn => {
                    println!("{:?}", self.stack.pop().unwrap());
                    return InterpretResult::InterpretOk;
                }
                OpCode::OpConstant => {
                    let value = self.read_constant(chunk);
                    self.stack.push(value);
                }
                OpCode::OpNegate => match self.stack.last_mut() {
                    Some(Value::Number(n)) => {
                        *n = -*n;
                    }
                    _ => {
                        return InterpretResult::InterpretRuntimeError;
                    }
                },
                OpCode::OpAdd => {
                    if let Err(e) = self.binary_op(|a, b| a + b) {
                        self.runtime_error(e, chunk);
                        return InterpretResult::InterpretRuntimeError;
                    }
                }
                OpCode::OpSubtract => {
                    if let Err(e) = self.binary_op(|a, b| a - b) {
                        self.runtime_error(e, chunk);
                        return InterpretResult::InterpretRuntimeError;
                    }
                }
                OpCode::OpMultiply => {
                    if let Err(e) = self.binary_op(|a, b| a * b) {
                        self.runtime_error(e, chunk);
                        return InterpretResult::InterpretRuntimeError;
                    }
                }
                OpCode::OpDivide => {
                    if let Err(e) = self.binary_op(|a, b| a / b) {
                        self.runtime_error(e, chunk);
                        return InterpretResult::InterpretRuntimeError;
                    }
                }
                OpCode::OPNil => self.stack.push(Value::Nil),
                OpCode::OpTrue => self.stack.push(Value::Bool(true)),
                OpCode::OpFalse => self.stack.push(Value::Bool(false)),
            }
        }
    }
    fn runtime_error(&mut self, string: &str, chunk: &Chunk) {
        eprintln!("Runtime Error: {}: on line {}", string, chunk.get_line(self.ip - 1));
        self.stack.clear();
    }
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
