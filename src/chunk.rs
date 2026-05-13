use crate::value::Value;

pub struct Chunk {
    code: Vec<u8>,
    constants: Vec<Value>,
    lines: Vec<usize>,
}
impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: vec![],
            constants: vec![],
            lines: vec![],
        }
    }

    pub fn write_byte(&mut self, byte: u8, line: usize) {
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
    pub fn get_line(&self, offset: usize) -> usize {
        self.lines[offset]
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

            offset = match OpCode::try_from(instruction).unwrap() {
                OpCode::OpReturn => self.debug_simple_instruction(f, "OP_RETURN", offset)?,
                OpCode::OpConstant => self.debug_constant_instruction(f, "OP_CONSTANT", offset)?,
                OpCode::OpNegate => self.debug_simple_instruction(f, "OP_NEGATE", offset)?,
                OpCode::OpAdd => self.debug_simple_instruction(f, "OP_ADD", offset)?,
                OpCode::OpSubtract => self.debug_simple_instruction(f, "OP_SUBTRACT", offset)?,
                OpCode::OpMultiply => self.debug_simple_instruction(f, "OP_MULTIPLY", offset)?,
                OpCode::OpDivide => self.debug_simple_instruction(f, "OP_DIVIDE", offset)?,
                OpCode::OPNil => self.debug_simple_instruction(f, "OP_NIL", offset)?,
                OpCode::OpTrue => self.debug_simple_instruction(f, "OP_TRUE", offset)?,
                OpCode::OpFalse => self.debug_simple_instruction(f, "OP_FALSE", offset)?,
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
    OPNil,
    OpTrue,
    OpFalse,
}
impl TryFrom<u8> for OpCode {
    type Error = String;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(OpCode::OpReturn),
            1 => Ok(OpCode::OpConstant),
            2 => Ok(OpCode::OpNegate),
            3 => Ok(OpCode::OpAdd),
            4 => Ok(OpCode::OpSubtract),
            5 => Ok(OpCode::OpMultiply),
            6 => Ok(OpCode::OpDivide),
            7 => Ok(OpCode::OPNil),
            8 => Ok(OpCode::OpTrue),
            9 => Ok(OpCode::OpFalse),
            _ => Err(format!("Unknown opcode: {}", byte)),
        }
    }
}
