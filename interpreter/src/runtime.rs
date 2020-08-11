use crate::opcodes::*;
use crate::util::*;
use std::io::Write;

pub struct Runtime<Out>
where
    Out: Write,
{
    pub stack: Vec<u32>,
    pub heap: Vec<u32>,
    pub pc: u32,
    pub fp: u32,
    pub stdout: Out,
}

impl<Out> Runtime<Out>
where
    Out: Write,
{
    pub fn new(stdout: Out) -> Self {
        Self {
            stack: Vec::new(),
            heap: Vec::new(),
            pc: 0,
            fp: 0,
            stdout,
        }
    }

    pub fn run_program(&mut self, program: Program) {}

    pub fn run_func(&mut self, program: Program, idx: u32) -> Result<(), Error> {
        Ok(())
    }
}
