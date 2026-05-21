use crate::value::Value;
use std::collections::HashMap;
use std::fmt;

// ── OpCode ───────────────────────────────────────────────────────────────────

#[repr(u8)]
pub enum OpCode {
    OpReturn,
    OpConstant,
    OpNegate,
    OpAdd,
    OpSubtract,
    OpMultiply,
    OpDivide,
    OpNil,
    OpTrue,
    OpFalse,
    OpNot,
    OpEqual,
    OpGreater,
    OpLess,
    OpPrint,
    OpPop,
    OpDefineGlobal,
    OpGetGlobal,
    OpSetGlobal,
    OpGetLocal,
    OpSetLocal,
    OpPopN,
    OpJumpIfFalse,
    OpJump,
    OpLoop,
    OpDup,
    OpYield,
    OpCall,
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
            7 => Ok(OpCode::OpNil),
            8 => Ok(OpCode::OpTrue),
            9 => Ok(OpCode::OpFalse),
            10 => Ok(OpCode::OpNot),
            11 => Ok(OpCode::OpEqual),
            12 => Ok(OpCode::OpGreater),
            13 => Ok(OpCode::OpLess),
            14 => Ok(OpCode::OpPrint),
            15 => Ok(OpCode::OpPop),
            16 => Ok(OpCode::OpDefineGlobal),
            17 => Ok(OpCode::OpGetGlobal),
            18 => Ok(OpCode::OpSetGlobal),
            19 => Ok(OpCode::OpGetLocal),
            20 => Ok(OpCode::OpSetLocal),
            21 => Ok(OpCode::OpPopN),
            22 => Ok(OpCode::OpJumpIfFalse),
            23 => Ok(OpCode::OpJump),
            24 => Ok(OpCode::OpLoop),
            25 => Ok(OpCode::OpDup),
            26 => Ok(OpCode::OpYield),
            27 => Ok(OpCode::OpCall),
            _ => Err(format!("Unknown opcode: {}", byte)),
        }
    }
}

// ── Chunk ────────────────────────────────────────────────────────────────────

pub struct Chunk {
    code: Vec<u8>,
    constants: Vec<Value>,
    lines: Vec<usize>,
    constant_index: HashMap<(u8, u64), u8>,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: vec![],
            constants: vec![],
            lines: vec![],
            constant_index: HashMap::new(),
        }
    }

    pub fn count(&self) -> usize {
        self.code.len()
    }

    pub fn write_byte_at(&mut self, offset: usize, byte: u8) {
        self.code[offset] = byte;
    }

    // ── Writing ──────────────────────────────────────────────────────────────

    pub fn write_byte(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn write_constant(&mut self, value: Value) -> u8 {
        let key = match &value {
            Value::Number(n) => Some((0u8, n.to_bits())),
            Value::Object(ptr) => Some((1u8, *ptr as u64)),
            _ => None,
        };
        if let Some(k) = key {
            if let Some(&idx) = self.constant_index.get(&k) {
                return idx;
            }
            let idx = self.constants.len() as u8;
            self.constants.push(value);
            self.constant_index.insert(k, idx);
            return idx;
        }
        self.constants.push(value);
        (self.constants.len() - 1) as u8
    }

    // ── Reading ──────────────────────────────────────────────────────────────

    pub fn read_byte(&self, offset: usize) -> u8 {
        self.code[offset]
    }

    pub fn read_constant(&self, offset: usize) -> Value {
        self.constants[offset].clone()
    }

    pub fn get_line(&self, offset: usize) -> usize {
        self.lines[offset]
    }

    // ── Disassembler ─────────────────────────────────────────────────────────

    fn disassemble_instruction(&self, f: &mut fmt::Formatter<'_>, offset: usize) -> Result<usize, fmt::Error> {
        match OpCode::try_from(self.code[offset]).unwrap() {
            OpCode::OpReturn => self.simple_instruction(f, "OP_RETURN", offset),
            OpCode::OpConstant => self.constant_instruction(f, "OP_CONSTANT", offset),
            OpCode::OpNegate => self.simple_instruction(f, "OP_NEGATE", offset),
            OpCode::OpAdd => self.simple_instruction(f, "OP_ADD", offset),
            OpCode::OpSubtract => self.simple_instruction(f, "OP_SUBTRACT", offset),
            OpCode::OpMultiply => self.simple_instruction(f, "OP_MULTIPLY", offset),
            OpCode::OpDivide => self.simple_instruction(f, "OP_DIVIDE", offset),
            OpCode::OpNil => self.simple_instruction(f, "OP_NIL", offset),
            OpCode::OpTrue => self.simple_instruction(f, "OP_TRUE", offset),
            OpCode::OpFalse => self.simple_instruction(f, "OP_FALSE", offset),
            OpCode::OpNot => self.simple_instruction(f, "OP_NOT", offset),
            OpCode::OpEqual => self.simple_instruction(f, "OP_EQUAL", offset),
            OpCode::OpGreater => self.simple_instruction(f, "OP_GREATER", offset),
            OpCode::OpLess => self.simple_instruction(f, "OP_LESS", offset),
            OpCode::OpPrint => self.simple_instruction(f, "OP_PRINT", offset),
            OpCode::OpPop => self.simple_instruction(f, "OP_POP", offset),
            OpCode::OpDup => self.simple_instruction(f, "OP_DUP", offset),
            OpCode::OpDefineGlobal => self.constant_instruction(f, "OP_DEFINE_GLOBAL", offset),
            OpCode::OpGetGlobal => self.constant_instruction(f, "OP_GET_GLOBAL", offset),
            OpCode::OpSetGlobal => self.constant_instruction(f, "OP_SET_GLOBAL", offset),
            OpCode::OpGetLocal => self.byte_instruction(f, "OP_GET_LOCAL", offset),
            OpCode::OpSetLocal => self.byte_instruction(f, "OP_SET_LOCAL", offset),
            OpCode::OpCall => self.byte_instruction(f, "OP_CALL", offset),
            OpCode::OpPopN => self.byte_instruction(f, "OP_POP_N", offset),
            OpCode::OpYield => self.byte_instruction(f, "OP_YIELD", offset),
            OpCode::OpJumpIfFalse => self.jump_instruction(f, "OP_JUMP_IF_FALSE", 1, offset),
            OpCode::OpJump => self.jump_instruction(f, "OP_JUMP", 1, offset),
            OpCode::OpLoop => self.jump_instruction(f, "OP_LOOP", -1, offset),
        }
    }

    fn simple_instruction(&self, f: &mut fmt::Formatter<'_>, name: &str, offset: usize) -> Result<usize, fmt::Error> {
        writeln!(f, "{}", name)?;
        Ok(offset + 1)
    }

    fn constant_instruction(&self, f: &mut fmt::Formatter<'_>, name: &str, offset: usize) -> Result<usize, fmt::Error> {
        let index = self.code[offset + 1] as usize;
        let value = &self.constants[index];
        writeln!(f, "{:<16} {:4} {:?}: {}", name, index, value, value)?;
        Ok(offset + 2)
    }

    fn byte_instruction(&self, f: &mut fmt::Formatter<'_>, name: &str, offset: usize) -> Result<usize, fmt::Error> {
        let slot = self.code[offset + 1];
        writeln!(f, "{:<16} {:4}", name, slot)?;
        Ok(offset + 2)
    }
    fn jump_instruction(&self, f: &mut fmt::Formatter<'_>, name: &str, sign: isize, offset: usize) -> Result<usize, fmt::Error> {
        let jump = ((self.code[offset + 1] as u16) << 8) | (self.code[offset + 2] as u16);
        let target = ((offset + 3) as isize + (sign * jump as isize)) as usize;
        writeln!(f, "{:<16} {:4} -> {}", name, offset, target)?;
        Ok(offset + 3)
    }
}

// ── Debug (disassembler) ─────────────────────────────────────────────────────

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
            offset = self.disassemble_instruction(f, offset)?;
        }
        Ok(())
    }
}
