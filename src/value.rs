use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Nil,
}

impl Add for &Value {
    type Output = Result<Value, &'static str>;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            // later for string and other values
            _ => Err("Operands must be numbers or strings."),
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
