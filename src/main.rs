mod chunk;
mod compiler;
mod scanner;
mod token;
mod value;
mod vm;

use crate::chunk::Chunk;
use crate::compiler::Compiler;
use crate::scanner::Scanner;
use crate::vm::{InterpretResult, Vm};
use std::io::Write;
use std::process::exit;
use std::{env, fs, io};

fn main() {
    let mut vm = Vm::new();
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        println!("Usage: rilox [script]");
        exit(-1);
    } else if args.len() == 2 {
        run_file(args[1].clone(), &mut vm);
    } else {
        run_prompt(&mut vm);
    }
}
pub fn interpret(source: String, vm: &mut Vm) -> InterpretResult {
    let mut chunk = Chunk::new();
    let mut scanner = Scanner::new(source);
    let mut parser = Compiler::new();
    if !parser.compile(&mut scanner, &mut chunk) {
        InterpretResult::InterpretCompileError
    } else {
        println!("{:?}", chunk);
        match vm.interpret(&chunk) {
            Ok(()) => InterpretResult::InterpretOk,
            Err(err) => err,
        }
    }
}

fn run_prompt(vm: &mut Vm) {
    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let mut line = String::new();
        let bytes_read = stdin.read_line(&mut line).unwrap();
        if bytes_read == 0 {
            break;
        }
        if line.is_empty() {
            continue;
        }
        interpret(line, vm);
    }
}

fn run_file(file_name: String, vm: &mut Vm) {
    let content: String = fs::read_to_string(&file_name).unwrap();
    let result = interpret(content, vm);
    match result {
        InterpretResult::InterpretOk => exit(0),
        InterpretResult::InterpretCompileError => exit(65),
        InterpretResult::InterpretRuntimeError => exit(70),
    }
}
