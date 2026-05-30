use crate::closure::ClosureObject;
use crate::function::FunctionObject;
use crate::native::NativeFunction;
use std::fmt::{Display, Formatter};
use std::ops::{Div, Mul, Sub};

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Nil,
    Object(*mut Object),
}
impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Value::Bool(b) => write!(f, "{}", b),
            Value::Number(n) => write!(f, "{}", n),
            Value::Nil => write!(f, "nil"),
            Value::Object(ptr) => unsafe {
                if ptr.is_null() {
                    return write!(f, "nil");
                }
                let obj = &**ptr;
                match &obj.obj_type {
                    ObjectType::String(s) => write!(f, "{}", s),
                    ObjectType::Function(fun) => write!(f, "<fun {}>", fun.name),
                    ObjectType::Native(fun) => write!(f, "<native fun {}>", fun.name),
                    ObjectType::Array(values) => {
                        write!(f, "[")?;
                        for (i, v) in values.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}", v)?;
                        }
                        write!(f, "]")
                    }
                    ObjectType::Closure(closure) => write!(f, "<fun {}>", (&*closure.function).name),
                }
            },
        }
    }
}
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Object(a), Value::Object(b)) => a == b,
            _ => false,
        }
    }
}
pub struct Object {
    pub obj_type: ObjectType,
    pub is_marked: bool,
    pub next: *mut Object,
}

pub enum ObjectType {
    String(String),
    Function(FunctionObject),
    Native(NativeFunction),
    Array(Vec<Value>),
    Closure(ClosureObject),
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        matches!(self, Value::Nil | Value::Bool(false))
    }
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    pub fn as_number(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            _ => panic!("as_number: not a number"),
        }
    }
    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            _ => panic!("as_bool: not a bool"),
        }
    }
    pub fn as_object(&self) -> *mut Object {
        match self {
            Value::Object(ptr) => *ptr,
            _ => panic!("as_object: not an object"),
        }
    }

    pub fn is_string(&self) -> bool {
        self.as_object_type().map_or(false, |o| matches!(o, ObjectType::String(_)))
    }
    pub fn is_function(&self) -> bool {
        self.as_object_type().map_or(false, |o| matches!(o, ObjectType::Function(_)))
    }
    pub fn is_array(&self) -> bool {
        self.as_object_type().map_or(false, |o| matches!(o, ObjectType::Array(_)))
    }
    pub fn is_closure(&self) -> bool {
        self.as_object_type().map_or(false, |o| matches!(o, ObjectType::Closure(_)))
    }
    pub fn is_native(&self) -> bool {
        self.as_object_type().map_or(false, |o| matches!(o, ObjectType::Native(_)))
    }

    fn as_object_type(&self) -> Option<&ObjectType> {
        match self {
            Value::Object(ptr) if !ptr.is_null() => Some(unsafe { &(**ptr).obj_type }),
            _ => None,
        }
    }

    // Typed heap access — caller must ensure ptr is valid and of the correct type.
    pub unsafe fn as_function(&self) -> &FunctionObject {
        unsafe { (*self.as_object()).as_function() }
    }
    pub unsafe fn as_string(&self) -> &str {
        unsafe { (*self.as_object()).as_string() }
    }
    pub unsafe fn as_function_mut(&self) -> &mut FunctionObject {
        unsafe { (*self.as_object()).as_function_mut() }
    }
    pub unsafe fn as_closure_mut(&self) -> &mut ClosureObject {
        unsafe { (*self.as_object()).as_closure_mut() }
    }
    pub unsafe fn as_array(&self) -> &Vec<Value> {
        unsafe { (*self.as_object()).as_array() }
    }
    pub unsafe fn as_array_mut(&self) -> &mut Vec<Value> {
        unsafe { (*self.as_object()).as_array_mut() }
    }
    pub unsafe fn as_native_mut(&self) -> &mut NativeFunction {
        unsafe { (*self.as_object()).as_native_mut() }
    }

    pub fn greater_than(&self, other: &Self) -> Result<Value, &'static str> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a > b)),
            _ => Err("Operands must be numbers."),
        }
    }
    pub fn less_than(&self, other: &Self) -> Result<Value, &'static str> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a < b)),
            _ => Err("Operands must be numbers."),
        }
    }
}

impl Object {
    pub fn as_string(&self) -> &str {
        match &self.obj_type {
            ObjectType::String(s) => s,
            _ => panic!("as_string: not a string"),
        }
    }
    pub fn as_function(&self) -> &FunctionObject {
        match &self.obj_type {
            ObjectType::Function(f) => f,
            _ => panic!("as_function: not a function"),
        }
    }
    pub fn as_function_mut(&mut self) -> &mut FunctionObject {
        match &mut self.obj_type {
            ObjectType::Function(f) => f,
            _ => panic!("as_function_mut: not a function"),
        }
    }
    pub fn as_array(&self) -> &Vec<Value> {
        match &self.obj_type {
            ObjectType::Array(v) => v,
            _ => panic!("as_array: not an array"),
        }
    }
    pub fn as_array_mut(&mut self) -> &mut Vec<Value> {
        match &mut self.obj_type {
            ObjectType::Array(v) => v,
            _ => panic!("as_array_mut: not an array"),
        }
    }
    pub fn as_native(&self) -> &NativeFunction {
        match &self.obj_type {
            ObjectType::Native(f) => f,
            _ => panic!("as_native: not a native"),
        }
    }
    pub fn as_native_mut(&mut self) -> &mut NativeFunction {
        match &mut self.obj_type {
            ObjectType::Native(f) => f,
            _ => panic!("as_native_mut: not a native"),
        }
    }
    pub fn as_closure_mut(&mut self) -> &mut ClosureObject {
        match &mut self.obj_type {
            ObjectType::Closure(c) => c,
            _ => panic!("as_closure_mut: not a closure"),
        }
    }
}
macro_rules! impl_numeric_op {
    ($trait_name:ident, $method_name:ident, $op:tt) => {
        impl $trait_name for &Value {
            type Output = Result<Value, &'static str>;

            fn $method_name(self, rhs: Self) -> Self::Output {
                match (self, rhs) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a $op b)),
                    _ => Err("Operands must be numbers."),
                }
            }
        }
    };
}
impl_numeric_op!(Sub, sub, -);
impl_numeric_op!(Mul, mul, *);
impl_numeric_op!(Div, div, /);
