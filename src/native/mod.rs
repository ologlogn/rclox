pub(crate) mod clock;
pub(crate) mod floor;
pub(crate) mod modulo;

use crate::value::Value;

pub type NativeFn = fn(&[Value]) -> Value;

pub struct NativeFunction {
    pub arity: usize,
    pub name: String,
    pub is_variadic: bool,
    pub fun: NativeFn,
}

pub fn get_native_functions() -> Vec<NativeFunction> {
    vec![clock::new(), floor::new(), modulo::new()]
}
