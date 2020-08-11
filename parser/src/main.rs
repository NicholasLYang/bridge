extern crate base64;
extern crate bimap;
extern crate byteorder;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate im;
extern crate itertools;
extern crate leb128;
extern crate strum;
#[macro_use]
extern crate strum_macros;
extern crate serde;
extern crate serde_json;

use crate::parser::Parser;
use std::env;
use std::fs::{self, File};
use std::io;

mod ast;
mod lexer;
mod parser;
mod printer;
mod symbol_table;
mod utils;

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: saber <file>");
    } else {
        let file_name = &args[1];
        let contents = fs::read_to_string(file_name)?;
        let lexer = lexer::Lexer::new(&contents);
        let mut parser = Parser::new(lexer);
        if let Ok(program) = parser.program() {
            for error in &program.errors {
                println!("{}", error);
            }
            let json = serde_json::to_string_pretty(&program.stmts)?;
            println!("{}", json);
        }
    };
    Ok(())
}
