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

use crate::parser::Parser;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use std::env;
use std::fs;
use std::io;

mod ast;
mod lexer;
mod parser;
mod printer;
mod symbol_table;
mod typechecker;
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
        let lexer = lexer::Lexer::new(&contents);
        let mut parser = Parser::new(lexer);
        if let Ok(program) = parser.program() {
            for error in &program.errors {
                let loc = error.get_location();
                let start = (loc.0).0;
                let end = (loc.1).0;
                let diagnostic = Diagnostic::error()
                    .with_message("Parse Error")
                    .with_labels(vec![
                        Label::primary((), (start)..(end)).with_message(error.to_string())
                    ]);
                term::emit(&mut writer.lock(), &config, &file, &diagnostic)?;
            }
            let json = serde_json::to_string_pretty(&program.stmts)?;
            println!("{}", json);
        }
    };
    Ok(())
}
