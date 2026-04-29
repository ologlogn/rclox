use crate::value::Value;

pub fn add(a: Value, b: Value) -> Value {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
        _ => panic!("operands must be numbers"),
    }
}

pub fn sub(a: Value, b: Value) -> Value {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => Value::Number(a - b),
        _ => panic!("operands must be numbers"),
    }
}

pub fn mul(a: Value, b: Value) -> Value {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => Value::Number(a * b),
        _ => panic!("operands must be numbers"),
    }
}

pub fn div(a: Value, b: Value) -> Value {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => Value::Number(a / b),
        _ => panic!("operands must be numbers"),
    }
}
