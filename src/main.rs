#![allow(dead_code)]
#![allow(unused_variables)]

extern crate base64;
extern crate bimap;
extern crate byteorder;
extern crate codespan_reporting;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate itertools;
extern crate leb128;
extern crate notify;
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
use std::io::{stdout, stdin};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use failure::Error;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::{env, fs, mem};

mod ast;
mod lexer;
mod parser;
mod printer;
mod runtime;
mod symbol_table;
mod treewalker;
mod typechecker;
mod unparser;
mod utils;
mod watcher;

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return run_repl();
    } else {
        let file_name = &args[1];
        let contents = fs::read_to_string(file_name)?;
        interpret_code(&contents, &file_name)?;
    };
    Ok(())
}

fn run_repl() -> Result<(), Error> {
    loop {
        let mut input = String::new();
        print!("> ");
        stdout().flush()?;
        stdin().read_line(&mut input)?;
        match input.trim().chars().last() {
            Some(';') | Some('}') => {
                interpret_code(&input, "<repl>")?;
            }
            c => {
                println!("{:?}", c);
                interpret_expr(&input, "<repl>")
            }
        }

    }
}

fn format_code(code: &str) -> Result<String, Error> {
    fs::write("out.brg", code)?;
    let process = Command::new("rustfmt")
        .arg("out.brg")
        .output().expect("failed to run rustfmt");
    Ok(fs::read_to_string("out.brg")?)
}

fn interpret_expr(code: &str, file_name: &str) {
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();
    let file = SimpleFile::new(file_name, code);
    let lexer = lexer::Lexer::new(code);
    let mut parser = Parser::new(lexer);
    let expr = match parser.expr() {
        Ok(e) => e,
        Err(err) => {
            let diagnostic: Diagnostic<()> = (&err).into();
            term::emit(&mut writer.lock(), &config, &file, &diagnostic).unwrap();
            return;
        }
    };
    let mut typechecker = TypeChecker::new(parser.get_name_table());
    let expr_t = match typechecker.expr(expr) {
        Ok(e) => e,
        Err(err) => {
            let diagnostic: Diagnostic<()> = (&err).into();
            term::emit(&mut writer.lock(), &config, &file, &diagnostic).unwrap();
            return;
        }
    };
    let functions = typechecker.get_functions();
    let mut treewalker = TreeWalker::new(functions);
    treewalker.print_expr(&expr_t);
}


fn interpret_code(code: &str, file_name: &str) -> Result<(), Error> {
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();
    let file = SimpleFile::new(file_name, code);
    let mut diagnostics: Vec<Diagnostic<()>> = Vec::new();
    if let Some((program, name_table)) = parse_file(code) {
        for error in &program.errors {
            diagnostics.push(error.into());
        }
        let (program_t, functions) = typecheck_file(program, name_table);
        for error in &program_t.errors {
            diagnostics.push(error.into());
        }
        let mut treewalker = TreeWalker::new(functions);

        match treewalker.interpret_program(program_t) {
            Err(e) => {
                println!("{:?}", e);
            }
            _ => {}
        };
    }
    for diagnostic in diagnostics {
        term::emit(&mut writer.lock(), &config, &file, &diagnostic)?;
    }
    Ok(())
}

fn unparse_code(program: &Program, name_table: NameTable) -> Result<String, Error> {
    let unparser = Unparser::new(name_table);
    let unparsed_program = unparser.unparse_program(program)?;

    let format_code = |program: String| -> Result<String, Error> {
        let formatter = Command::new("rustfmt")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut stdin = formatter.stdin.unwrap();
        let mut stdout = formatter.stdout.unwrap();

        stdin.write_all(program.as_bytes())?;
        mem::drop(stdin);
        let mut out = String::new();
        stdout.read_to_string(&mut out)?;

        if let Some(stderr) = formatter.stderr {
            let mut stderr = stderr;
            let mut errors = String::new();
            stderr.read_to_string(&mut errors)?;
            println!("{}", errors);
        }

        return Ok(out);
    };

    let functions = format_code(unparsed_program.functions)?;
    let globals_fmt = format_code(unparsed_program.global_stmts)?;
    let functions = functions.replace("print!(", "print(");
    let globals_fmt = globals_fmt.replace("print!(", "print(");

    let start = globals_fmt.find('{').unwrap() + 1;
    let end = globals_fmt.len() - 2;

    let mut globals = String::new();
    for line in globals_fmt[start..end].trim().split('\n') {
        globals += line.trim();
    }

    Ok(format!("{}\n{}", functions, globals))
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

fn parse_file(contents: &str) -> Option<(Program, NameTable)> {
    let lexer = lexer::Lexer::new(contents);
    let mut parser = Parser::new(lexer);
    if let Ok(program) = parser.program() {
        Some((program, parser.get_name_table()))
    } else {
        None
    }
}
