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

mod lexer;
mod parser;
mod ast;

fn main() {
  println!("Hello World!");
}