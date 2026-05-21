use crate::native::{NativeFn, NativeFunction};
use crate::value::Value;

pub fn clock() -> NativeFn {
    |_| {
        use std::time::{SystemTime, UNIX_EPOCH};
        Value::Number(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64())
    }
}

pub fn new() -> NativeFunction {
    NativeFunction {
        arity: 0,
        name: "clock".to_string(),
        is_variadic: false,
        fun: clock(),
    }
}
