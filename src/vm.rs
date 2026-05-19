use crate::chunk::{Chunk, OpCode};
use crate::heap::Heap;
use crate::value::{Object, ObjectType, Value};
use std::collections::HashMap;
use std::fmt::Debug;

pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

pub struct Vm {
    ip: usize,
    stack: Vec<Value>,
    heap: Heap,
    interned_strings: HashMap<String, *mut Object>,
    globals: HashMap<String, Value>,
}

impl Vm {
    // ── Setup ────────────────────────────────────────────────────────────────

    pub fn new() -> Vm {
        Vm {
            ip: 0,
            stack: Vec::new(),
            heap: Heap::new(),
            interned_strings: HashMap::new(),
            globals: HashMap::new(),
        }
    }

    pub fn interpret(&mut self, chunk: &Chunk) -> Result<(), InterpretResult> {
        self.ip = 0;
        self.stack.clear();
        self.run(chunk)
    }

    // ── Bytecode reading ─────────────────────────────────────────────────────

    fn read_byte(&mut self, chunk: &Chunk) -> u8 {
        let byte = chunk.read_byte(self.ip);
        self.ip += 1;
        byte
    }

    fn read_short(&mut self, chunk: &Chunk) -> u16 {
        let high = chunk.read_byte(self.ip) as u16;
        let low = chunk.read_byte(self.ip + 1) as u16;
        self.ip += 2;
        (high << 8) | low
    }

    fn read_constant(&mut self, chunk: &Chunk) -> Value {
        let index = self.read_byte(chunk);
        chunk.read_constant(index as usize)
    }

    fn read_string(&mut self, chunk: &Chunk) -> String {
        let constant = self.read_constant(chunk);
        format!("{}", constant)
    }

    // ── Stack helpers ────────────────────────────────────────────────────────

    fn peek_top(&self) -> Value {
        self.stack[self.stack.len() - 1].clone()
    }

    // ── Arithmetic helpers ────────────────────────────────────────────────────

    fn binary_op<F>(&mut self, chunk: &Chunk, op: F) -> Result<(), InterpretResult>
    where
        F: FnOnce(&Value, &Value) -> Result<Value, &'static str>,
    {
        let len = self.stack.len();
        if len < 2 {
            self.runtime_error("Stack underflow", chunk);
            return Err(InterpretResult::InterpretRuntimeError);
        }
        let a = &self.stack[len - 2];
        let b = &self.stack[len - 1];
        match op(a, b) {
            Ok(result) => {
                self.stack.pop();
                self.stack.pop();
                self.stack.push(result);
                Ok(())
            }
            Err(msg) => {
                self.runtime_error(msg, chunk);
                Err(InterpretResult::InterpretRuntimeError)
            }
        }
    }

    fn add(&mut self, chunk: &Chunk) -> Result<(), InterpretResult> {
        let len = self.stack.len();
        if len < 2 {
            self.runtime_error("Stack underflow", chunk);
            return Err(InterpretResult::InterpretRuntimeError);
        }
        let a = &self.stack[len - 2];
        let b = &self.stack[len - 1];
        let result = match (a, b) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
            (Value::Object(a_ptr), Value::Object(b_ptr)) => unsafe {
                let obj_a = &**a_ptr;
                let obj_b = &**b_ptr;
                match (&obj_a.obj_type, &obj_b.obj_type) {
                    (ObjectType::String(str_a), ObjectType::String(str_b)) => {
                        let concatenated = format!("{}{}", str_a, str_b);
                        let ptr = self.allocate_string(concatenated.as_str());
                        Value::Object(ptr)
                    }
                }
            },
            _ => {
                self.runtime_error("Operands must be two numbers or two strings", chunk);
                return Err(InterpretResult::InterpretRuntimeError);
            }
        };
        self.stack.pop();
        self.stack.pop();
        self.stack.push(result);
        Ok(())
    }

    // ── Main dispatch loop ───────────────────────────────────────────────────

    fn run(&mut self, chunk: &Chunk) -> Result<(), InterpretResult> {
        loop {
            let opcode = OpCode::try_from(self.read_byte(chunk)).unwrap();
            match opcode {
                // ── Control flow ─────────────────────────────────────────────
                OpCode::OpReturn => return Ok(()),
                OpCode::OpJumpIfFalse => {
                    let offset = self.read_short(chunk);
                    if self.peek_top().is_falsey() {
                        self.ip += offset as usize;
                    }
                }
                OpCode::OpJump => {
                    let offset = self.read_short(chunk);
                    self.ip += offset as usize;
                }
                OpCode::OpLoop => {
                    let offset = self.read_short(chunk);
                    self.ip -= offset as usize;
                }
                // ── Constants ────────────────────────────────────────────────
                OpCode::OpConstant => {
                    let value = self.read_constant(chunk);
                    self.stack.push(value);
                }
                OpCode::OpNil => self.stack.push(Value::Nil),
                OpCode::OpTrue => self.stack.push(Value::Bool(true)),
                OpCode::OpFalse => self.stack.push(Value::Bool(false)),

                // ── Stack ops ────────────────────────────────────────────────
                OpCode::OpPop => {
                    self.stack.pop();
                }
                OpCode::OpPopN => {
                    let to_pop = self.read_byte(chunk);
                    self.stack.truncate(self.stack.len() - to_pop as usize);
                }
                OpCode::OpDup => {
                    let v1 = self.peek_top();
                    self.stack.push(v1.clone());
                }
                OpCode::OpTuckN => {
                    let result = self.stack.pop().unwrap();
                    let to_pop = self.read_byte(chunk);
                    self.stack.truncate(self.stack.len() - to_pop as usize);
                    self.stack.push(result);
                }

                // ── Globals ──────────────────────────────────────────────────
                OpCode::OpDefineGlobal => {
                    let name = self.read_string(chunk);
                    self.globals.insert(name, self.stack.pop().unwrap());
                }
                OpCode::OpGetGlobal => {
                    let name = self.read_string(chunk);
                    match self.globals.get(&name) {
                        Some(value) => self.stack.push(value.clone()),
                        None => {
                            self.runtime_error(&format!("Undefined variable '{}'", name), chunk);
                            return Err(InterpretResult::InterpretRuntimeError);
                        }
                    }
                }
                OpCode::OpSetGlobal => {
                    let name = self.read_string(chunk);
                    if self.globals.contains_key(&name) {
                        let value = self.peek_top();
                        self.globals.insert(name, value);
                    } else {
                        self.runtime_error(&format!("Undefined variable '{}'", name), chunk);
                        return Err(InterpretResult::InterpretRuntimeError);
                    }
                }

                // ── Locals ───────────────────────────────────────────────────
                OpCode::OpGetLocal => {
                    let slot = self.read_byte(chunk) as usize;
                    self.stack.push(self.stack[slot].clone());
                }
                OpCode::OpSetLocal => {
                    let slot = self.read_byte(chunk) as usize;
                    self.stack[slot] = self.peek_top();
                }

                // ── Unary ops ────────────────────────────────────────────────
                OpCode::OpNegate => match self.stack.last_mut() {
                    Some(Value::Number(n)) => *n = -*n,
                    _ => return Err(InterpretResult::InterpretRuntimeError),
                },
                OpCode::OpNot => {
                    let value = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(value.is_falsey()));
                }

                // ── Binary ops ───────────────────────────────────────────────
                OpCode::OpAdd => self.add(chunk)?,
                OpCode::OpSubtract => self.binary_op(chunk, |a, b| a - b)?,
                OpCode::OpMultiply => self.binary_op(chunk, |a, b| a * b)?,
                OpCode::OpDivide => self.binary_op(chunk, |a, b| a / b)?,
                OpCode::OpGreater => self.binary_op(chunk, |a, b| a.greater_than(b))?,
                OpCode::OpLess => self.binary_op(chunk, |a, b| a.less_than(b))?,
                OpCode::OpEqual => self.binary_op(chunk, |a, b| Ok(Value::Bool(a == b)))?,

                // ── Output ───────────────────────────────────────────────────
                OpCode::OpPrint => println!("{}", self.stack.pop().unwrap()),
            }
        }
    }

    // ── Error handling ───────────────────────────────────────────────────────

    fn runtime_error(&mut self, message: &str, chunk: &Chunk) {
        eprintln!("[line {}] Runtime error: {}", chunk.get_line(self.ip - 1), message);
        self.ip = 0;
        self.stack.clear();
    }

    // ── Memory ───────────────────────────────────────────────────────────────

    pub fn allocate_object(&mut self, obj_type: ObjectType) -> *mut Object {
        self.heap.allocate(obj_type)
    }

    pub fn allocate_string(&mut self, string: &str) -> *mut Object {
        if let Some(&ptr) = self.interned_strings.get(string) {
            return ptr;
        }
        let ptr = self.allocate_object(ObjectType::String(string.to_string()));
        self.interned_strings.insert(string.to_string(), ptr);
        ptr
    }
}

// ── Debug ────────────────────────────────────────────────────────────────────

impl Debug for Vm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "        ")?;
        for value in self.stack.iter() {
            write!(f, "[ {:?} ]", value)?;
        }
        writeln!(f)
    }
}
