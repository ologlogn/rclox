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
    strings: HashMap<String, *mut Object>,
    globals: HashMap<String, Value>,
}

impl Vm {
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
    pub fn new() -> Vm {
        Vm {
            ip: 0, // instruction pointer
            stack: Vec::new(),
            heap: Heap::new(),
            strings: HashMap::new(),
            globals: HashMap::new(),
        }
    }
    fn peek(&self, index: usize) -> Value {
        self.stack[index].clone()
    }
    pub fn interpret(&mut self, chunk: &Chunk) -> Result<(), InterpretResult> {
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

    fn read_string(&mut self, chunk: &Chunk) -> String {
        let name = self.read_constant(chunk);
        format!("{}", name)
    }

    fn run(&mut self, chunk: &Chunk) -> Result<(), InterpretResult> {
        loop {
            match OpCode::try_from(self.read_byte(chunk)).unwrap() {
                OpCode::OpReturn => {
                    return Ok(());
                }
                OpCode::OpConstant => {
                    let value = self.read_constant(chunk);
                    self.stack.push(value);
                }
                OpCode::OpPop => {
                    self.stack.pop();
                }
                OpCode::OpDefineGlobal => {
                    let name = self.read_string(chunk);
                    self.globals.insert(name, self.stack.pop().unwrap());
                }
                OpCode::OpGetGlobal => {
                    let name = self.read_string(chunk);
                    if let Some(obj) = self.globals.get(&name) {
                        self.stack.push(obj.clone());
                    } else {
                        self.runtime_error(format!("Undefined variable {}", name).as_str(), chunk);
                        return Err(InterpretResult::InterpretRuntimeError);
                    }
                }
                OpCode::OpSetGlobal => {
                    let name = self.read_string(chunk);
                    if self.globals.contains_key(&name) {
                        self.globals.insert(name, self.peek(self.stack.len() - 1));
                    } else {
                        self.runtime_error(format!("Undefined variable {}", name).as_str(), chunk);
                        return Err(InterpretResult::InterpretRuntimeError);
                    }
                }
                OpCode::OpNegate => match self.stack.last_mut() {
                    Some(Value::Number(n)) => {
                        *n = -*n;
                    }
                    _ => {
                        return Err(InterpretResult::InterpretRuntimeError);
                    }
                },
                OpCode::OpAdd => {
                    let len = self.stack.len();
                    if len < 2 {
                        self.runtime_error("Stack underflow", chunk);
                        return Err(InterpretResult::InterpretRuntimeError);
                    }
                    let a = &self.stack[len - 2];
                    let b = &self.stack[len - 1];
                    let val;
                    match (a, b) {
                        (Value::Number(a_num), Value::Number(b_num)) => {
                            val = Value::Number(a_num + b_num);
                        }
                        // Handle Strings
                        (Value::Object(a_ptr), Value::Object(b_ptr)) => unsafe {
                            let obj_a = &**a_ptr;
                            let obj_b = &**b_ptr;
                            match (&obj_a.obj_type, &obj_b.obj_type) {
                                (ObjectType::String(str_a), ObjectType::String(str_b)) => {
                                    let mut new_string = String::with_capacity(str_a.len() + str_b.len());
                                    new_string.push_str(str_a);
                                    new_string.push_str(str_b);
                                    let new_ptr = self.allocate_string(new_string.as_str());
                                    val = Value::Object(new_ptr);
                                }
                            }
                        },
                        _ => {
                            self.runtime_error("Operands must be two numbers or two strings", chunk);
                            return Err(InterpretResult::InterpretRuntimeError);
                        }
                    }
                    self.stack.pop();
                    self.stack.pop();
                    self.stack.push(val);
                }
                OpCode::OpSubtract => self.binary_op(chunk, |a, b| a - b)?,
                OpCode::OpMultiply => self.binary_op(chunk, |a, b| a * b)?,
                OpCode::OpDivide => self.binary_op(chunk, |a, b| a / b)?,
                OpCode::OpGreater => self.binary_op(chunk, |a, b| a.greater_than(b))?,
                OpCode::OpLess => self.binary_op(chunk, |a, b| a.less_than(b))?,
                OpCode::OpEqual => self.binary_op(chunk, |a, b| Ok(Value::Bool(a == b)))?,
                OpCode::OpNil => self.stack.push(Value::Nil),
                OpCode::OpTrue => self.stack.push(Value::Bool(true)),
                OpCode::OpFalse => self.stack.push(Value::Bool(false)),
                OpCode::OpNot => {
                    let value = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(value.is_falsey()));
                }
                OpCode::OpPrint => println!("{}", self.stack.pop().unwrap()),
            }
        }
    }
    fn runtime_error(&mut self, string: &str, chunk: &Chunk) {
        eprintln!("Runtime Error: {}: on line {}", string, chunk.get_line(self.ip - 1));
        self.ip = 0;
        self.stack.clear();
    }
    pub fn allocate_object(&mut self, obj_type: ObjectType) -> *mut Object {
        self.heap.allocate(obj_type)
    }
    pub fn allocate_string(&mut self, string: &str) -> *mut Object {
        if let Some(&ptr) = self.strings.get(string) {
            return ptr;
        }
        let str_ptr = self.allocate_object(ObjectType::String(string.to_string()));
        self.strings.insert(string.to_string(), str_ptr);
        str_ptr
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
