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

#[derive(Debug, Clone, Copy)]
pub struct VarPointer {
    idx: u32,
    offset: u32,
}

impl VarPointer {
    pub fn new_stack(idx: u32, offset: u32) -> VarPointer {
        if idx & !(1u32 << 31) != idx {
            panic!("idx is too large");
        }

        let idx = idx | (1u32 << 31);
        Self { idx, offset }
    }

    pub fn new_heap(idx: u32, offset: u32) -> VarPointer {
        if idx & !(1u32 << 31) != idx {
            panic!("idx is too large");
        }

        Self { idx, offset }
    }

    pub fn var_idx(self) -> u32 {
        self.idx & !(1u32 << 31)
    }

    pub fn is_stack(self) -> bool {
        self.idx & (1u32 << 31) != 0
    }
}

impl From<u64> for VarPointer {
    fn from(val: u64) -> Self {
        Self {
            idx: (val >> 32) as u32,
            offset: val as u32,
        }
    }
}

impl Into<u64> for VarPointer {
    fn into(self) -> u64 {
        ((self.idx as u64) << 32) + self.offset as u64
    }
}

pub struct VarBuffer {
    pub data: Vec<u8>,  // Allocator for variables
    pub vars: Vec<Var>, // Tracker for variables
}

impl VarBuffer {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            vars: Vec::new(),
        }
    }

    pub fn get_var_record(&self, var_idx: u32) -> Result<Var, Error> {
        match self.vars.get(var_idx as usize) {
            Some(x) => Ok(*x),
            None => error!(
                "IncorrectVarOffset",
                "a variable offset of {} is incorrect", var_idx as usize
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

    pub fn add_var<T: Copy + Default>(&mut self, t: T) -> Var {
        let idx = self.data.len() as u32;
        let len = mem::size_of::<T>();
        if len > u32::MAX as usize {
            panic!("struct too long");
        }

        let len = len as u32;
        let var = Var { idx, len };
        self.data.extend_from_slice(any_as_u8_slice(&t));
        self.vars.push(var);
        return var;
    }

    pub fn add_var_dyn(&mut self, len: u32) -> Var {
        let idx = self.data.len() as u32;
        if len > u32::MAX {
            panic!("struct too long");
        }

        let var = Var { idx, len };
        self.data.resize((idx + len) as usize, 0);
        self.vars.push(var);
        return var;
    }

    pub fn pop_var(&mut self) -> Option<Var> {
        let var = self.vars.pop();

        if let Some(var) = var {
            self.data.resize(var.idx as usize, 0);
            return Some(var);
        } else {
            return None;
        }
    }

    pub fn shrink_vars_to(&mut self, len: u32) {
        if (len as usize) < self.vars.len() {
            panic!("shrinking to a length larger than the vars array");
        }

        self.vars.resize(len as usize, Var { idx: 0, len: 0 });
        if let Some(var) = self.vars.last() {
            self.data.resize((var.idx + var.len) as usize, 0);
        } else {
            self.data.resize(0, 0);
        }
    }

    pub fn push_word(&mut self, word: u64) {
        self.data.extend_from_slice(any_as_u8_slice(&word));
    }

    pub fn pop_word(&mut self) -> Result<u64, Error> {
        if self.data.len() < 8 {
            return error!("StackIsEmpty", "tried to pop from stack when it is empty");
        }

        let mut out = 0u64;
        let to_bytes = unsafe { any_as_u8_slice_mut(&mut out) };

        if let Some(var) = self.vars.last() {
            let upper = (var.idx + var.len) as usize;
            if self.data.len() - upper < 8 {
                return error!(
                    "StackPopInvalidatesVariable",
                    "popping from the stack would invalidate a variable"
                );
            }
        }

        let upper = self.data.len();

        to_bytes.copy_from_slice(&self.data[(upper - 8)..upper]);
        return Ok(out);
    }

    pub fn set<T: Copy + Default>(&mut self, var_idx: u32, offset: i32, t: T) -> Result<(), Error> {
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
    pub stack: VarBuffer,
    pub heap: VarBuffer,
    pub callstack: Vec<FuncDesc>, // Callstack
    pub stdout: Out,
}

impl<Out> Runtime<Out>
where
    Out: Write,
{
    pub fn new(stdout: Out) -> Self {
        Self {
            stack: VarBuffer::new(),
            heap: VarBuffer::new(),
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

        let fp = self.stack.vars.len() as u32;
        let mut pc = program_counter + 1;

        loop {
            let should_return = match self.run_op(fp, program.ops[pc]) {
                Ok(x) => x,
                Err(mut err) => {
                    err.stack_trace.reserve(self.callstack.len());
                    for frame in self.callstack.iter() {
                        err.stack_trace.push(*frame);
                    }
                    return Err(err);
                }
            };

            if should_return {
                self.stack.shrink_vars_to(fp);
                self.callstack.resize(callstack_len, self.callstack[0]);
                return Ok(());
            }

            pc += 1;
        }
    }

    #[inline]
    pub fn run_op(&mut self, fp: u32, opcode: Opcode) -> Result<bool, Error> {
        match opcode {
            Opcode::Function(_) => {}

            Opcode::StackAlloc(space) => {
                self.stack.add_var_dyn(space);
            }
            Opcode::StackAllocPtr(space) => {
                let var = self.stack.add_var_dyn(space);
                self.stack
                    .push_word(VarPointer::new_stack(var.idx, 0).into());
            }
            Opcode::Alloc(space) => {
                let var = self.heap.add_var_dyn(space);
                self.stack
                    .push_word(VarPointer::new_heap(var.idx, 0).into());
            }

            Opcode::MakeTempIntWord(value) => {
                self.stack.push_word(value as u64);
            }

            Opcode::GetLocalWord { var_offset, offset } => {
                let global_idx = if var_offset < 0 {
                    let var_offset = (var_offset * -1) as u32;
                    fp - var_offset
                } else {
                    fp + var_offset as u32
                };

                self.stack
                    .push_word(self.stack.get_var(global_idx, offset as i32)?);
            }
            Opcode::SetLocalWord { var_offset, offset } => {
                let global_idx = if var_offset < 0 {
                    let var_offset = (var_offset * -1) as u32;
                    fp - var_offset
                } else {
                    fp + var_offset as u32
                };

                let word = self.stack.pop_word()?;
                self.stack.set(global_idx, offset as i32, word)?;
            }

            Opcode::GetWord { offset } => {
                let ptr: VarPointer = self.stack.pop_word()?.into();
                let offset = if offset < 0 {
                    let offset = (offset * -1) as u32;
                    ptr.offset - offset
                } else {
                    ptr.offset + offset as u32
                } as i32;

                let word = if ptr.is_stack() {
                    self.stack.get_var(ptr.var_idx(), offset)?
                } else {
                    self.heap.get_var(ptr.var_idx(), offset)?
                };

                self.stack.push_word(word);
            }
            Opcode::SetWord { offset } => {
                let ptr: VarPointer = self.stack.pop_word()?.into();
                let offset = if offset < 0 {
                    let offset = (offset * -1) as u32;
                    ptr.offset - offset
                } else {
                    ptr.offset + offset as u32
                } as i32;

                let word = self.stack.pop_word()?;

                if ptr.is_stack() {
                    self.stack.set(ptr.var_idx(), offset, word)?
                } else {
                    self.heap.set(ptr.var_idx(), offset, word)?
                }
            }

            Opcode::Ret => return Ok(true),

            Opcode::AddCallstackDesc(desc) => {
                self.callstack.push(desc);
            }
            Opcode::RemoveCallstackDesc => {
                self.callstack.pop();
            }
        }

        return Ok(false);
    }
}
