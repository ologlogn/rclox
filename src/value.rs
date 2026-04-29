#[derive(Clone, Copy)]
#[derive(Debug)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Nil,
}