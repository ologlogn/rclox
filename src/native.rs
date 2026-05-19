use crate::value::Value;

pub type NativeFn = fn(&[Value]) -> Value;

pub fn modulo() -> NativeFn {
    |args| {
        let n = args.len();
        if n != 2 {
            panic!("mod: expected 2 arguments, got {}", n)
        }
        let Value::Number(a) = args[0] else {
            panic!("mod: expected a number, got {}", args[0])
        };
        let Value::Number(b) = args[1] else {
            panic!("mod: expected a number, got {}", args[1])
        };
        Value::Number(a % b)
    }
}
