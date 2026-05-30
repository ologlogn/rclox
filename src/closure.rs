use crate::function::FunctionObject;

pub struct ClosureObject {
    pub function: *mut FunctionObject,
}
impl ClosureObject {
    pub fn new(function: &mut FunctionObject) -> Self {
        Self { function }
    }
}
pub struct CallFrame {
    pub closure: *mut ClosureObject,
    pub ip: usize,
    pub stack_base: usize,
}
#[derive(Debug, Clone)]
pub struct CompilerUpvalue {
    pub index: u8,
    pub is_local: bool,
}
