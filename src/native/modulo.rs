use crate::native::{NativeFn, NativeFunction};
use crate::value::Value;

pub fn modulo() -> NativeFn {
    |args| {
        let Value::Number(a) = args[0] else {
            panic!("mod: expected a number, got {}", args[0])
        };
        let Value::Number(b) = args[1] else {
            panic!("mod: expected a number, got {}", args[1])
        };
        Value::Number(a % b)
    }
}

pub fn new() -> NativeFunction {
    NativeFunction {
        arity: 2,
        name: "mod".to_string(),
        is_variadic: false,
        fun: modulo(),
    }
}
