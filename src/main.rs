mod chunk;
mod compiler;
mod function;
mod heap;
mod native;
mod scanner;
mod token;
mod value;
mod vm;

use crate::compiler::Compiler;
use crate::function::FunctionType;
use crate::scanner::Scanner;
use crate::vm::{InterpretResult, Vm};
use rustyline::config::Configurer;
use rustyline::{DefaultEditor, EditMode, error::ReadlineError};
use std::process::exit;
use std::{env, fs};

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
    let scanner = Scanner::new(source);
    let mut compiler = Compiler::new(scanner, vm, FunctionType::TypeScript);
    if let Some(func) = compiler.compile() {
        match vm.interpret(func) {
            Ok(()) => InterpretResult::InterpretOk,
            Err(err) => err,
        }
    } else {
        InterpretResult::InterpretCompileError
    }
}

fn run_prompt(vm: &mut Vm) {
    let mut rl = DefaultEditor::new().expect("failed to init editor");
    rl.set_edit_mode(EditMode::Vi);
    loop {
        match rl.readline("> ") {
            Ok(line) => {
                let cmd = line.trim();
                if cmd.is_empty() {
                    continue;
                }
                rl.add_history_entry(cmd).ok();
                interpret(cmd.to_string(), vm);
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("error: {err}");
                break;
            }
        }
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
