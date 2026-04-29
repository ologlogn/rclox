use crate::value::Value;

pub struct Chunk {
    code: Vec<u8>,
    constants: Vec<Value>,
    lines: Vec<i32>,
}
impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: vec![],
            constants: vec![],
            lines: vec![],
        }
    }

    pub fn write_byte(&mut self, byte: u8, line: i32) {
        self.code.push(byte);
        self.lines.push(line);
    }
    pub fn write_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn read_byte(&self, offset: usize) -> u8 {
        self.code[offset]
    }
    pub fn read_constant(&self, offset: usize) -> Value {
        self.constants[offset].clone()
    }
}
use std::fmt;

impl fmt::Debug for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "== chunk ==")?;

        let mut offset = 0;
        while offset < self.code.len() {
            write!(f, "{:04} ", offset)?;

            if offset > 0 && self.lines[offset] == self.lines[offset - 1] {
                write!(f, "   | ")?;
            } else {
                write!(f, "{:4} ", self.lines[offset])?;
            }

            let instruction = self.code[offset];

            offset = match OpCode::from(instruction) {
                OpCode::OpReturn => self.debug_simple_instruction(f, "OP_RETURN", offset)?,
                OpCode::OpConstant => self.debug_constant_instruction(f, "OP_CONSTANT", offset)?,
                OpCode::OpNegate => self.debug_simple_instruction(f, "OP_NEGATE", offset)?,
                OpCode::OpAdd => self.debug_simple_instruction(f, "OP_ADD", offset)?,
                OpCode::OpSubtract => self.debug_simple_instruction(f, "OP_SUBTRACT", offset)?,
                OpCode::OpMultiply => self.debug_simple_instruction(f, "OP_MULTIPLY", offset)?,
                OpCode::OpDivide => self.debug_simple_instruction(f, "OP_DIVIDE", offset)?,
            };
        }

        Ok(())
    }
}
impl Chunk {
    fn debug_simple_instruction(
        &self,
        f: &mut fmt::Formatter<'_>,
        name: &str,
        offset: usize,
    ) -> Result<usize, fmt::Error> {
        writeln!(f, "{}", name)?;
        Ok(offset + 1)
    }

    fn debug_constant_instruction(
        &self,
        f: &mut fmt::Formatter<'_>,
        name: &str,
        offset: usize,
    ) -> Result<usize, fmt::Error> {
        let constant_index = self.code[offset + 1] as usize;
        let value = &self.constants[constant_index];
        writeln!(f, "{:<16} {:4} {:?}", name, constant_index, value)?;
        Ok(offset + 2)
    }
}

#[repr(u8)]
pub enum OpCode {
    OpReturn,
    OpConstant,
    OpNegate,
    OpAdd,
    OpSubtract,
    OpMultiply,
    OpDivide,
}
impl From<u8> for OpCode {
    fn from(byte: u8) -> Self {
        match byte {
            0 => OpCode::OpReturn,
            1 => OpCode::OpConstant,
            2 => OpCode::OpNegate,
            3 => OpCode::OpAdd,
            4 => OpCode::OpSubtract,
            5 => OpCode::OpMultiply,
            6 => OpCode::OpDivide,
            _ => panic!("Unknown opcode: {}", byte),
        }
    }
}
