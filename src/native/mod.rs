mod clock;
mod floor;
mod modulo;
mod random;

use crate::value::Value;

pub type NativeFn = fn(&[Value]) -> Result<Value, String>;

pub struct NativeFunction {
    pub arity: usize,
    pub name: String,
    pub is_variadic: bool,
    pub fun: NativeFn,
}

pub fn get_native_functions() -> Vec<NativeFunction> {
    vec![clock::new(), floor::new(), modulo::new(), random::new()]
}
