use crate::native::{NativeFn, NativeFunction};
use crate::value::Value;

pub fn floor() -> NativeFn {
    |args| {
        let Value::Number(n) = args[0] else {
            return Err(format!("floor: expected a number, got {}", args[0]));
        };
        Ok(Value::Number(n.floor()))
    }
}

pub fn new() -> NativeFunction {
    NativeFunction {
        arity: 1,
        name: "floor".to_string(),
        is_variadic: false,
        fun: floor(),
    }
}
