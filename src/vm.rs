use crate::chunk::{Chunk, OpCode};
use crate::closure::{CallFrame, ClosureObject, UpValueObject};
use crate::function::FunctionObject;
use crate::heap::Heap;
use crate::native::{NativeFunction, get_native_functions};
use crate::value::{Object, ObjectType, Value};
use std::collections::HashMap;
use std::fmt::Debug;

pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

pub struct Vm {
    stack: Vec<Value>,
    heap: Heap,
    interned_strings: HashMap<String, *mut Object>,
    globals: HashMap<String, Value>,
    call_stack: Vec<CallFrame>,
}

impl Vm {
    // ── Setup ────────────────────────────────────────────────────────────────

    pub fn new() -> Vm {
        let mut vm = Vm {
            stack: Vec::with_capacity(256),
            heap: Heap::new(),
            interned_strings: HashMap::new(),
            globals: HashMap::new(),
            call_stack: Vec::new(),
        };
        let native_functions = get_native_functions();
        for f in native_functions {
            vm.define_native(f.name.clone().as_str(), f);
        }
        vm
    }

    fn current_frame(&mut self) -> &mut CallFrame {
        self.call_stack.last_mut().unwrap()
    }
    fn current_chunk(&mut self) -> &mut Chunk {
        &mut unsafe { (&mut *self.current_frame().closure).function_mut() }.chunk
    }
    pub fn interpret(&mut self, function: *mut Object) -> Result<(), InterpretResult> {
        self.stack.push(Value::Object(function));
        unsafe {
            let func = (*function).as_function_mut();
            let closure = ClosureObject::new(func);
            let closure_obj = self.allocate_object(ObjectType::Closure(closure));
            self.stack.pop();
            self.stack.push(Value::Object(closure_obj));
            self.call((*closure_obj).as_closure_mut(), 0)?;
        }
        self.run()
    }

    fn check_arity(&mut self, arity: usize, arg_count: usize, name: String) -> Result<(), InterpretResult> {
        if arg_count != arity {
            self.runtime_error(format!("Expected {} but got {} arguments for function {}", arity, arg_count, name).as_str());
            Err(InterpretResult::InterpretRuntimeError)
        } else {
            Ok(())
        }
    }
    fn call(&mut self, closure: &mut ClosureObject, arg_count: usize) -> Result<(), InterpretResult> {
        let function = unsafe { closure.function_mut() };
        self.check_arity(function.arity, arg_count, function.name.clone())?;
        let frame = CallFrame {
            closure,
            ip: 0,
            stack_base: self.stack.len() - arg_count - 1,
        };
        self.call_stack.push(frame);
        Ok(())
    }
    fn call_native(&mut self, f: &mut NativeFunction, arg_count: usize) -> Result<(), InterpretResult> {
        self.check_arity(f.arity, arg_count, f.name.clone())?;
        let len = self.stack.len();
        let args = self.stack[len - arg_count..len].to_vec();
        let fun = f.fun;
        match fun(&args) {
            Ok(result) => {
                self.stack.truncate(len - arg_count - 1);
                self.stack.push(result);
                Ok(())
            }
            Err(msg) => {
                self.runtime_error(&msg);
                Err(InterpretResult::InterpretRuntimeError)
            }
        }
    }

    // ── Bytecode reading ─────────────────────────────────────────────────────

    fn read_byte(&mut self) -> u8 {
        let ip = self.current_frame().ip;
        let byte = self.current_chunk().read_byte(ip);
        self.current_frame().ip += 1;
        byte
    }

    fn read_short(&mut self) -> u16 {
        let ip = self.current_frame().ip;
        let high = self.current_chunk().read_byte(ip) as u16;
        let low = self.current_chunk().read_byte(ip + 1) as u16;
        self.current_frame().ip += 2;
        (high << 8) | low
    }

    fn read_constant(&mut self) -> Value {
        let b = self.read_byte();
        self.current_chunk().read_constant(b as usize)
    }

    fn read_string(&mut self) -> String {
        format!("{}", self.read_constant())
    }

    // ── Stack helpers ────────────────────────────────────────────────────────

    fn peek_top(&self) -> Value {
        self.stack[self.stack.len() - 1].clone()
    }

    // ── Array helpers ────────────────────────────────────────────────────────
    fn validate_index(&mut self, n: f64, len: usize) -> Result<usize, InterpretResult> {
        if n < 0.0 || n.fract() != 0.0 {
            self.runtime_error("Index must be a non-negative integer");
            return Err(InterpretResult::InterpretRuntimeError);
        }
        let i = n as usize;
        if i >= len {
            self.runtime_error(&format!("Index {} out of bounds (len {})", i, len));
            return Err(InterpretResult::InterpretRuntimeError);
        }
        Ok(i)
    }

    fn pop_array(&mut self) -> Result<Value, InterpretResult> {
        let value = self.stack.pop().unwrap();
        if !value.is_array() {
            self.runtime_error("Not an array");
            return Err(InterpretResult::InterpretRuntimeError);
        }
        Ok(value)
    }

    fn pop_array_and_index(&mut self) -> Result<(Value, usize), InterpretResult> {
        let index = self.stack.pop().unwrap();
        let Value::Number(n) = index else {
            self.runtime_error("Index must be a number");
            return Err(InterpretResult::InterpretRuntimeError);
        };
        let arr = self.pop_array()?;
        let len = unsafe { arr.as_array() }.len();
        let i = self.validate_index(n, len)?;
        Ok((arr, i))
    }

    // ── Arithmetic helpers ────────────────────────────────────────────────────

    fn binary_op<F>(&mut self, op: F) -> Result<(), InterpretResult>
    where
        F: FnOnce(&Value, &Value) -> Result<Value, &'static str>,
    {
        let len = self.stack.len();
        if len < 2 {
            self.runtime_error("Stack underflow");
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
                self.runtime_error(msg);
                Err(InterpretResult::InterpretRuntimeError)
            }
        }
    }

    fn add(&mut self) -> Result<(), InterpretResult> {
        let len = self.stack.len();
        if len < 2 {
            self.runtime_error("Stack underflow");
            return Err(InterpretResult::InterpretRuntimeError);
        }
        let a = self.stack[len - 2].clone();
        let b = self.stack[len - 1].clone();
        let result = if a.is_number() && b.is_number() {
            Value::Number(a.as_number() + b.as_number())
        } else if a.is_string() && b.is_string() {
            let str_a = unsafe { a.as_string() }.to_string();
            let str_b = unsafe { b.as_string() }.to_string();
            let concatenated = format!("{}{}", str_a, str_b);
            let ptr = self.allocate_string(&concatenated);
            Value::Object(ptr)
        } else {
            self.runtime_error("Operands must be two numbers or two strings");
            return Err(InterpretResult::InterpretRuntimeError);
        };
        self.stack.pop();
        self.stack.pop();
        self.stack.push(result);
        Ok(())
    }

    // ── Main dispatch loop ───────────────────────────────────────────────────

    fn run(&mut self) -> Result<(), InterpretResult> {
        loop {
            let opcode = OpCode::try_from(self.read_byte()).unwrap();
            match opcode {
                // ── Functions ─────────────────────────────────────────────
                OpCode::OpCall => {
                    let arg_count = self.read_byte() as usize;
                    let val = self.stack[self.stack.len() - arg_count - 1].clone();
                    if val.is_closure() {
                        self.call(unsafe { val.as_closure_mut() }, arg_count)?;
                    } else if val.is_native() {
                        self.call_native(unsafe { val.as_native_mut() }, arg_count)?;
                    } else {
                        self.runtime_error("Invalid function type");
                        return Err(InterpretResult::InterpretRuntimeError);
                    }
                }
                OpCode::OpClosure => unsafe {
                    let fun_val = self.read_constant();
                    if !fun_val.is_function() {
                        self.runtime_error("Not a function");
                        return Err(InterpretResult::InterpretRuntimeError);
                    }

                    let function = fun_val.as_function_mut();
                    let mut closure = ClosureObject::new(function);

                    for i in 0..function.upvalue_count {
                        let is_local = self.read_byte() == 1;
                        let index = self.read_byte() as usize;

                        if is_local {
                            let slot = self.current_frame().stack_base  + index;
                            let slot_ptr = &mut self.stack[slot] as *mut Value;
                            let upvalue_obj = self.capture_upvalue(slot_ptr);
                            closure.upvalues[i] = upvalue_obj;
                        } else {
                            let c = self.current_frame().closure;
                            let parent_upvalues: &Vec<*mut Object> = &(*c).upvalues;
                            closure.upvalues[i] = parent_upvalues[index];
                        }
                    }
                    let closure_obj = self.allocate_object(ObjectType::Closure(closure));
                    self.stack.push(Value::Object(closure_obj));
                },
                OpCode::OpGetUpvalue => unsafe {
                    let slot = self.read_byte() as usize;
                    let c = self.current_frame().closure;
                    let upvalues: &Vec<*mut Object> = &(*c).upvalues;
                    let upvalue = upvalues[slot].as_mut().unwrap().as_upvalue_mut();
                    let value = (*upvalue.location).clone(); // dereference the pointer
                    self.stack.push(value);
                },
                OpCode::OpSetUpvalue => unsafe {
                    let slot = self.read_byte() as usize;
                    let c = self.current_frame().closure;
                    let upvalues: &Vec<*mut Object> = &(*c).upvalues;
                    let val = self.peek_top();
                    *upvalues[slot].as_mut().unwrap().as_upvalue_mut().location = val;
                },
                OpCode::OpCloseUpvalue => unsafe {

                }
                OpCode::OpReturn => {
                    let result = self.stack.pop().unwrap();
                    let frame = self.call_stack.pop().expect("call stack underflow");
                    if self.call_stack.is_empty() {
                        return Ok(()); // end program
                    }
                    self.stack.truncate(frame.stack_base);
                    self.stack.push(result);
                }

                OpCode::OpYield => {
                    let result = self.stack.pop().unwrap();
                    let to_pop = self.read_byte();
                    self.stack.truncate(self.stack.len() - to_pop as usize);
                    self.stack.push(result);
                }

                // ── Control flow ─────────────────────────────────────────────
                OpCode::OpJumpIfFalse => {
                    let offset = self.read_short();
                    if self.peek_top().is_falsey() {
                        self.current_frame().ip += offset as usize;
                    }
                }
                OpCode::OpJump => {
                    let offset = self.read_short();
                    self.current_frame().ip += offset as usize;
                }
                OpCode::OpLoop => {
                    let offset = self.read_short();
                    self.current_frame().ip -= offset as usize;
                }
                // ── Array ─────────────────────────────────────────────
                OpCode::OpArray => {
                    let count = self.read_byte() as usize;
                    let len = self.stack.len();
                    let values: Vec<Value> = self.stack.drain(len - count..).collect();
                    let array = self.allocate_object(ObjectType::Array(values));
                    self.stack.push(Value::Object(array));
                }
                OpCode::OpMakeArray => {
                    let len = self.stack.pop().unwrap();
                    let Value::Number(n) = len else {
                        self.runtime_error("Length must be a number");
                        return Err(InterpretResult::InterpretRuntimeError);
                    };
                    if n < 0.0 || n.fract() != 0.0 {
                        self.runtime_error("Length must be a non-negative integer");
                        return Err(InterpretResult::InterpretRuntimeError);
                    }
                    let values = vec![Value::Nil; n as usize];
                    let array = self.allocate_object(ObjectType::Array(values));
                    self.stack.push(Value::Object(array));
                }
                OpCode::OpLen => {
                    let arr = self.pop_array()?;
                    let len = unsafe { arr.as_array() }.len();
                    self.stack.push(Value::Number(len as f64));
                }
                OpCode::OpGetIndex => {
                    let (arr, i) = self.pop_array_and_index()?;
                    let val = unsafe { arr.as_array() }[i].clone();
                    self.stack.push(val);
                }

                OpCode::OpSetIndex => {
                    let value = self.stack.pop().unwrap();
                    let (arr, i) = self.pop_array_and_index()?;
                    let arr_mut = unsafe { arr.as_array_mut() };
                    arr_mut[i] = value.clone();
                    self.stack.push(value);
                }
                // ── Constants ────────────────────────────────────────────────
                OpCode::OpConstant => {
                    let value = self.read_constant();
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
                    let to_pop = self.read_byte();
                    self.stack.truncate(self.stack.len() - to_pop as usize);
                }
                OpCode::OpDup => {
                    let v1 = self.peek_top();
                    self.stack.push(v1.clone());
                }

                // ── Globals ──────────────────────────────────────────────────
                OpCode::OpDefineGlobal => {
                    let name = self.read_string();
                    self.globals.insert(name, self.stack.pop().unwrap());
                }
                OpCode::OpGetGlobal => {
                    let name = self.read_string();
                    match self.globals.get(&name) {
                        Some(value) => self.stack.push(value.clone()),
                        None => {
                            self.runtime_error(&format!("Undefined variable '{}'", name));
                            return Err(InterpretResult::InterpretRuntimeError);
                        }
                    }
                }
                OpCode::OpSetGlobal => {
                    let name = self.read_string();
                    if self.globals.contains_key(&name) {
                        let value = self.peek_top();
                        self.globals.insert(name, value);
                    } else {
                        self.runtime_error(&format!("Undefined variable '{}'", name));
                        return Err(InterpretResult::InterpretRuntimeError);
                    }
                }

                // ── Locals ───────────────────────────────────────────────────
                OpCode::OpGetLocal => {
                    let slot = self.read_byte() as usize;
                    let base = self.current_frame().stack_base;
                    self.stack.push(self.stack[base + slot].clone());
                }
                OpCode::OpSetLocal => {
                    let slot = self.read_byte() as usize;
                    let base = self.current_frame().stack_base;
                    self.stack[slot + base] = self.peek_top();
                }

                // ── Unary ops ────────────────────────────────────────────────
                OpCode::OpNegate => match self.stack.last_mut() {
                    Some(Value::Number(n)) => *n = -*n,
                    _ => {
                        self.runtime_error("Operand must be a number");
                        return Err(InterpretResult::InterpretRuntimeError);
                    }
                },
                OpCode::OpNot => {
                    let value = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(value.is_falsey()));
                }

                // ── Binary ops ───────────────────────────────────────────────
                OpCode::OpAdd => self.add()?,
                OpCode::OpSubtract => self.binary_op(|a, b| a - b)?,
                OpCode::OpMultiply => self.binary_op(|a, b| a * b)?,
                OpCode::OpDivide => self.binary_op(|a, b| a / b)?,
                OpCode::OpGreater => self.binary_op(|a, b| a.greater_than(b))?,
                OpCode::OpLess => self.binary_op(|a, b| a.less_than(b))?,
                OpCode::OpEqual => self.binary_op(|a, b| Ok(Value::Bool(a == b)))?,

                // ── Output ───────────────────────────────────────────────────
                OpCode::OpPrint => println!("{}", self.stack.pop().unwrap()),
            }
        }
    }

    // ── Error handling ───────────────────────────────────────────────────────

    pub fn runtime_error(&mut self, message: &str) {
        eprintln!("{}", message);
        for frame in self.call_stack.iter().rev() {
            let func = unsafe { (&*frame.closure).function_ref() };
            let line = func.chunk.get_line(frame.ip - 1);
            if func.name.is_empty() {
                eprintln!("[line {}] in script", line);
            } else {
                eprintln!("[line {}] in {}()", line, func.name);
            }
        }
        self.stack.clear();
        self.call_stack.clear();
    }

    // ── Memory ───────────────────────────────────────────────────────────────

    pub fn allocate_object(&mut self, obj_type: ObjectType) -> *mut Object {
        self.heap.allocate(obj_type)
    }
    pub fn capture_upvalue(&mut self, slot: *mut Value) -> *mut Object {
        
        self.allocate_object(ObjectType::UpValue(UpValueObject::new(slot)))
    }
    pub fn allocate_string(&mut self, string: &str) -> *mut Object {
        if let Some(&ptr) = self.interned_strings.get(string) {
            return ptr;
        }
        let ptr = self.allocate_object(ObjectType::String(string.to_string()));
        self.interned_strings.insert(string.to_string(), ptr);
        ptr
    }
    pub fn allocate_function(&mut self, func: FunctionObject) -> *mut Object {
        self.allocate_object(ObjectType::Function(func))
    }
    pub fn define_native(&mut self, name: &str, f: NativeFunction) {
        let obj = self.allocate_object(ObjectType::Native(f));
        self.globals.insert(name.to_string(), Value::Object(obj));
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
