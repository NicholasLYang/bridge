use crate::opcodes::*;
use crate::util::*;
use std::io::Write;

macro_rules! error{
    ($arg1:tt,$($arg:tt)*) => {
        Err(Error::new($arg1, format!($($arg)*)))
    };
}

pub struct Runtime<Out>
where
    Out: Write,
{
    pub stack: Vec<u32>,
    pub heap: Vec<u32>,
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
            stdout,
        }
    }

    pub fn run_program(&mut self, program: Program) -> Result<(), Error> {
        return self.run_func(program, 0);
    }

    pub fn run_func(&mut self, program: Program, idx: usize) -> Result<(), Error> {
        let func_desc = match program.ops[idx] {
            Opcode::Function(desc) => desc,
            op => {
                return error!(
                    "InvalidFunctionHeader",
                    "found function header {:?} (this is an error in your compiler)", op
                )
            }
        };

        let mut current = idx + 1;
        loop {
            let should_return = match self.run_op(program.ops[current]) {
                Ok(should_return) => should_return,
                Err(mut err) => {
                    err.stack_trace.push(func_desc);
                    return Err(err);
                }
            };

            if should_return {
                return Ok(());
            }

            current += 1;
        }
    }

    #[inline]
    pub fn run_op(&mut self, opcode: Opcode) -> Result<bool, Error> {
        match opcode {
            Opcode::Function(_) => {}
            Opcode::MakeInt(value) => {
                self.stack.push(value as u32);
            }
        }

        return Ok(false);
    }
}
