use crate::function::FunctionObject;
use crate::value::{Object, Value};
use std::ptr::null_mut;

pub struct ClosureObject {
    pub function: *mut FunctionObject,
    pub upvalues: Vec<*mut Object>,
}
impl ClosureObject {
    pub fn new(function: &mut FunctionObject) -> Self {
        let upvalue_count = function.upvalue_count;
        Self {
            function,
            upvalues: vec![null_mut(); upvalue_count],
        }
    }
    pub unsafe fn function_ref(&self) -> &FunctionObject {
        unsafe { &*self.function }
    }
    pub unsafe fn function_mut(&mut self) -> &mut FunctionObject {
        unsafe { &mut *self.function }
    }
}
pub struct CallFrame {
    pub closure: *mut ClosureObject,
    pub ip: usize,
    pub stack_base: usize,
}
#[derive(Debug, Clone)]
pub struct CompilerUpvalue {
    pub index: u8,
    pub is_local: bool,
}

pub struct UpValueObject {
    pub location: *mut Value, // pointer into VM.stack
    pub closed: Value,
    pub next: *mut UpValueObject,
}
impl UpValueObject {
    pub fn new(location: *mut Value) -> Self {
        Self {
            location,
            closed: Value::Nil,
            next: null_mut(),
        }
    }
}
