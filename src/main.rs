#![allow(dead_code)]
#![allow(unused_variables)]

extern crate base64;
extern crate bimap;
extern crate byteorder;
extern crate codespan_reporting;
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

use crate::ast::{Function, Name, Program, ProgramT};
use crate::parser::{ParseError, Parser};
use crate::treewalker::TreeWalker;
use crate::typechecker::{TypeChecker, TypeError};
use crate::unparser::Unparser;
use crate::utils::NameTable;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;

mod ast;
//mod code_generator;
//mod emitter;
mod lexer;
mod parser;
mod printer;
mod runtime;
mod symbol_table;
mod treewalker;
mod typechecker;
mod unparser;
mod utils;

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();
    if args.len() < 2 {
        println!("Usage: saber <file>");
    } else {
        let file_name = &args[1];
        let contents = fs::read_to_string(file_name)?;
        let file = SimpleFile::new(file_name, &contents);
        let mut diagnostics: Vec<Diagnostic<()>> = Vec::new();
        if let Some((program, name_table)) = parse_file(&contents) {
            for error in &program.errors {
                diagnostics.push(error.into());
            }
            let unparser = Unparser::new(name_table);
            let unparser_out = unparser.unparse_program(&program);
            match unparser_out {
                Ok(contents) => println!("{}", contents),
                Err(e) => println!("Unparser error: {:?}", e),
            }

            /*let (program_t, functions) = typecheck_file(program, name_table);
            for error in &program_t.errors {
                diagnostics.push(error.into());
            }
            let mut treewalker = TreeWalker::new(functions);
            match treewalker.interpret_program(program_t) {
                Err(e) => {
                    println!("{:?}", e);
                }
                _ => {}
            };*/
        }
        for diagnostic in diagnostics {
            term::emit(&mut writer.lock(), &config, &file, &diagnostic)?;
        }
    };
    Ok(())
}

impl Into<Diagnostic<()>> for &TypeError {
    fn into(self) -> Diagnostic<()> {
        let loc = self.get_location();
        let start = (loc.0).0;
        let end = (loc.1).0;
        Diagnostic::error()
            .with_message("Type Error")
            .with_labels(vec![
                Label::primary((), (start)..(end)).with_message(self.to_string())
            ])
    }
}

impl Into<Diagnostic<()>> for &ParseError {
    fn into(self) -> Diagnostic<()> {
        let loc = self.get_location();
        let start = (loc.0).0;
        let end = (loc.1).0;
        Diagnostic::error()
            .with_message("Parse Error")
            .with_labels(vec![
                Label::primary((), (start)..(end)).with_message(self.to_string())
            ])
    }
}

fn typecheck_file(program: Program, name_table: NameTable) -> (ProgramT, HashMap<Name, Function>) {
    let mut typechecker = TypeChecker::new(name_table);
    (
        typechecker.check_program(program),
        typechecker.get_functions(),
    )
}

fn parse_file(contents: &String) -> Option<(Program, NameTable)> {
    let lexer = lexer::Lexer::new(contents);
    let mut parser = Parser::new(lexer);
    if let Ok(program) = parser.program() {
        Some((program, parser.get_name_table()))
    } else {
        None
    }
}
