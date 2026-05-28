use crate::native::{NativeFn, NativeFunction};
use crate::value::Value;
use rand::RngExt;

pub fn random() -> NativeFn {
    |args| {
        let Value::Number(min) = args[0] else {
            panic!("random: expected a number for min, got {}", args[0])
        };
        let Value::Number(max) = args[1] else {
            panic!("random: expected a number for max, got {}", args[1])
        };

        let mut rng = rand::rng();
        Value::Number(rng.random_range(min.floor() as i64..=max.floor() as i64) as f64)
    }
}
pub fn new() -> NativeFunction {
    NativeFunction {
        arity: 2,
        name: "random".to_string(),
        is_variadic: false,
        fun: random(),
    }
}
