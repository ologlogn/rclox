#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Nil,
}
