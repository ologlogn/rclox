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
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        matches!(self, Value::Nil | Value::Bool(false))
    }
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
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
