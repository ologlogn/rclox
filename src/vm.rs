use crate::chunk::{Chunk, OpCode};
use crate::value::Value;
use std::fmt::Debug;

macro_rules! binary_op {
    ($vm:expr, $op:tt) => {
        if let Some(Value::Number(b)) = $vm.stack.pop() {
             if let Some(Value::Number(a)) = $vm.stack.pop() {
                $vm.stack.push(Value::Number(a $op b));
             } else {
                    println!("Operand must be a number.");
                    return InterpretResult::InterpretRuntimeError;
                }
        } else {
                println!("Operand must be a number.");
                return InterpretResult::InterpretRuntimeError;
        }
    };
}
pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

pub struct Vm {
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
            match OpCode::from(self.read_byte(chunk)) {
                OpCode::OpReturn => {
                    println!("{:?}", self.stack.pop().unwrap());
                    return InterpretResult::InterpretOk;
                }
                OpCode::OpConstant => {
                    let value = self.read_constant(chunk);
                    self.stack.push(value);
                }
                OpCode::OpNegate => {
                    if let Some(Value::Number(number)) = self.stack.pop() {
                        self.stack.push(Value::Number(-number));
                    } else {
                        return InterpretResult::InterpretRuntimeError;
                    }
                }
                OpCode::OpAdd => binary_op!(self, +),
                OpCode::OpSubtract => binary_op!(self, -),
                OpCode::OpMultiply => binary_op!(self, *),
                OpCode::OpDivide => binary_op!(self, /),
            }
        }
    }
}
