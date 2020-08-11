use crate::opcodes::*;
use crate::util::*;
use core::mem;
use std::io::Write;

macro_rules! error{
    ($arg1:tt,$($arg:tt)*) => {
        Err(Error::new($arg1, format!($($arg)*)))
    };
}

#[derive(Debug, Clone, Copy)]
pub struct Var {
    pub idx: u32,
    pub len: u32, // len in bytes
}

impl From<u64> for Var {
    fn from(val: u64) -> Self {
        Self {
            idx: (val >> 32) as u32,
            len: val as u32,
        }
    }
}

pub struct VarBuffer {
    pub data: Vec<u8>,  // Allocator for variables
    pub vars: Vec<Var>, // Tracker for variables
}

impl VarBuffer {
    pub fn get_var_record(&self, var_idx: u32) -> Result<Var, Error> {
        match self.vars.get(var_idx as usize) {
            Some(x) => Ok(*x),
            None => error!(
                "IncorrectVarOffset",
                "a variable offset of {} is incorrect; \
                    (this is likely a problem with your compiler)",
                var_idx as usize
            ),
        }
    }

    pub fn get_var_range_mut(
        &mut self,
        var: &Var,
        offset: i32,
        len: u32,
    ) -> Result<&mut [u8], Error> {
        if (len as i32) < 0i32 {
            panic!("struct too long");
        }

        let upper = offset + len as i32;
        if upper > var.len as i32 || offset < 0i32 {
            return error!(
                "OutOfValueBounds",
                "tried to access bytes {}..{} in an object of length {}", offset, upper, var.len
            );
        }

        let begin = (var.idx + offset as u32) as usize;

        return Ok(&mut self.data[begin..(begin + upper as usize)]);
    }

    pub fn get_var_range(&self, var: &Var, offset: i32, len: u32) -> Result<&[u8], Error> {
        if (len as i32) < 0i32 {
            panic!("struct too long");
        }

        let upper = offset + len as i32;
        if upper > var.len as i32 || offset < 0i32 {
            return error!(
                "OutOfValueBounds",
                "tried to access bytes {}..{} in an object of length {}", offset, upper, var.len
            );
        }

        let begin = (var.idx + offset as u32) as usize;

        return Ok(&self.data[begin..(begin + upper as usize)]);
    }

    pub fn get_var<T: Copy + Default>(&self, var_idx: u32, offset: i32) -> Result<T, Error> {
        let var = self.get_var_record(var_idx)?;
        let len = mem::size_of::<T>();
        if len > u32::MAX as usize {
            panic!("struct too long");
        }

        let from_bytes = self.get_var_range(&var, offset, len as u32)?;

        let mut t = T::default();
        let to_bytes = unsafe { any_as_u8_slice_mut(&mut t) };
        to_bytes.copy_from_slice(from_bytes);
        return Ok(t);
    }

    pub fn add_var<T: Copy + Default>(&mut self, t: T) -> Result<Var, Error> {
        let idx = self.data.len() as u32;
        let len = mem::size_of::<T>();
        if len > u32::MAX as usize {
            panic!("struct too long");
        }

        let len = len as u32;
        let var = Var { idx, len };
        self.data.extend_from_slice(any_as_u8_slice(&t));
        self.vars.push(var);
        return Ok(var);
    }

    pub fn add_var_stack_overwrite<T: Copy + Default>(&mut self, t: T) -> Result<Var, Error> {
        let idx = match self.vars.last() {
            Some(var) => var.idx + var.len,
            None => 0,
        };
        let len = mem::size_of::<T>();
        if len > u32::MAX as usize {
            panic!("struct too long");
        }

        let len = len as u32;
        let var = Var { idx, len };
        self.data[(idx as usize)..((idx + len) as usize)].copy_from_slice(any_as_u8_slice(&t));
        self.vars.push(var);
        return Ok(var);
    }

    pub fn set_var<T: Copy + Default>(
        &mut self,
        var_idx: u32,
        offset: i32,
        t: T,
    ) -> Result<(), Error> {
        let var = self.get_var_record(var_idx)?;
        let len = mem::size_of::<T>();
        if len > u32::MAX as usize {
            panic!("struct too long");
        }

        let to_bytes = self.get_var_range_mut(&var, offset, len as u32)?;
        to_bytes.copy_from_slice(any_as_u8_slice(&t));

        return Ok(());
    }
}

pub struct Runtime<Out>
where
    Out: Write,
{
    pub stack: Vec<u8>,           // Allocator for stack variables
    pub stack_vars: Vec<Var>,     // Tracker for stack variables
    pub heap: Vec<u8>,            // Allocator for heap data
    pub heap_vars: Vec<Var>,      // Tracker for heap allocations
    pub callstack: Vec<FuncDesc>, // Callstack
    pub stdout: Out,
}

impl<Out> Runtime<Out>
where
    Out: Write,
{
    pub fn new(stdout: Out) -> Self {
        Self {
            stack: Vec::new(),
            stack_vars: Vec::new(),
            heap: Vec::new(),
            heap_vars: Vec::new(),
            callstack: Vec::new(),
            stdout,
        }
    }

    pub fn run_program(&mut self, program: Program) -> Result<(), Error> {
        return self.run_func(program, 0);
    }

    pub fn run_func(&mut self, program: Program, program_counter: usize) -> Result<(), Error> {
        let func_desc = match program.ops[program_counter] {
            Opcode::Function(desc) => desc,
            op => {
                return error!(
                    "InvalidFunctionHeader",
                    "found function header {:?} (this is an error in your compiler)", op
                )
            }
        };

        let callstack_len = self.callstack.len();
        self.callstack.push(func_desc);

        let fp = self.stack.len() as u64;
        let mut pc = program_counter + 1;

        loop {
            let should_return = self.run_op(fp, program.ops[pc])?;

            if should_return {
                self.stack.resize(fp as usize, 0);
                self.callstack.resize(callstack_len, self.callstack[0]);
                return Ok(());
            }

            pc += 1;
        }
    }

    #[inline]
    pub fn run_op(&mut self, fp: u64, opcode: Opcode) -> Result<bool, Error> {
        match opcode {
            Opcode::Function(_) => {}
            _ => {}
        }

        return Ok(false);
    }
}
