use crate::opcodes::*;
use crate::util::*;
use codegenerator::opcodes::*;
use core::{mem, str};
use std::io::Write;

pub const DEBUG: bool = true;

macro_rules! error {
    ($arg1:tt,$($arg:tt)*) => {
        Error::new($arg1, format!($($arg)*))
    };
}

macro_rules! err {
    ($arg1:tt,$($arg:tt)*) => {
        Err(Error::new($arg1, format!($($arg)*)))
    };
}

#[cfg(test)]
macro_rules! map(
    { $($key:expr => $value:expr),* } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )*
            m
        }
     };
);

#[derive(Debug, Clone, Copy)]
pub struct Var {
    pub idx: u32,
    pub len: u32, // len in bytes
}

#[derive(Debug, Clone, Copy)]
pub struct VarPointer {
    idx: u32,
    offset: i32,
}

impl Default for VarPointer {
    fn default() -> Self {
        VarPointer { idx: 0, offset: 0 }
    }
}

impl VarPointer {
    pub fn new_stack(idx: u32, offset: i32) -> VarPointer {
        if idx & !(1u32 << 31) != idx {
            panic!("idx is too large");
        }

        let idx = idx | (1u32 << 31);
        Self { idx, offset }
    }

    pub fn new_heap(idx: u32, offset: i32) -> VarPointer {
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

    pub fn get_var_range_mut(&mut self, ptr: VarPointer, len: u32) -> Result<&mut [u8], Error> {
        if ptr.var_idx() == 0 {
            return err!("NullPointer", "a variable of offset 0 was used");
        }

        let var = match self.vars.get(ptr.var_idx() as usize - 1) {
            Some(x) => *x,
            None => {
                return err!(
                    "IncorrectVarOffset",
                    "a variable offset of {} is incorrect",
                    ptr.idx
                )
            }
        };

        let upper = ptr.offset + len as i32;
        if upper > var.len as i32 || ptr.offset < 0i32 {
            return err!(
                "OutOfValueBounds",
                "tried to access bytes {}..{} in an object of length {}",
                ptr.offset,
                upper,
                var.len
            );
        }

        let begin = var.idx + ptr.offset as u32;
        return Ok(&mut self.data[begin as usize..((begin + len) as usize)]);
    }

    pub fn get_var_range(&self, ptr: VarPointer, len: u32) -> Result<&[u8], Error> {
        if ptr.var_idx() == 0 {
            return err!("NullPointer", "a variable of offset 0 was used");
        }

        let var = match self.vars.get(ptr.var_idx() as usize - 1) {
            Some(x) => *x,
            None => {
                return err!(
                    "IncorrectVarOffset",
                    "a variable offset of {} is incorrect",
                    ptr.idx
                )
            }
        };

        let upper = ptr.offset + len as i32;
        if upper > var.len as i32 || ptr.offset < 0i32 {
            return err!(
                "OutOfValueBounds",
                "tried to access bytes {}..{} in an object of length {}",
                ptr.offset,
                upper,
                var.len
            );
        }

        let begin = (var.idx + ptr.offset as u32) as usize;
        return Ok(&self.data[begin..(begin + upper as usize)]);
    }

    pub fn get_full_var_range(&self, ptr: VarPointer) -> Result<&[u8], Error> {
        if ptr.var_idx() == 0 {
            return err!("NullPointer", "a variable of offset 0 was used");
        }

        let var = match self.vars.get(ptr.var_idx() as usize - 1) {
            Some(x) => *x,
            None => {
                return err!(
                    "IncorrectVarOffset",
                    "a variable offset of {} is incorrect",
                    ptr.idx
                )
            }
        };

        if ptr.offset > var.len as i32 || ptr.offset < 0i32 {
            return err!(
                "OutOfValueBounds",
                "tried to access byte {} in an object of length {}",
                ptr.offset,
                var.len
            );
        }

        return Ok(&self.data[var.idx as usize..((var.idx + var.len) as usize)]);
    }

    pub fn get_var<T: Copy + Default>(&self, ptr: VarPointer) -> Result<T, Error> {
        let len = mem::size_of::<T>();
        if len > u32::MAX as usize {
            panic!("struct too long");
        }

        let from_bytes = self.get_var_range(ptr, len as u32)?;

        let mut t = T::default();
        let to_bytes = unsafe { any_as_u8_slice_mut(&mut t) };
        to_bytes.copy_from_slice(from_bytes);
        return Ok(t);
    }

    pub fn upper_bound(&self) -> u32 {
        return (self.vars.len() + 1) as u32;
    }

    pub fn add_var<T: Copy + Default>(&mut self, t: T) -> u32 {
        let idx = self.data.len() as u32;
        let len = mem::size_of::<T>();
        if len > u32::MAX as usize {
            panic!("struct too long");
        }

        let len = len as u32;
        let var = Var { idx, len };
        self.data.extend_from_slice(any_as_u8_slice(&t));
        let var_idx = self.vars.len() as u32 + 1;
        self.vars.push(var);
        return var_idx;
    }

    pub fn add_var_dyn(&mut self, len: u32) -> u32 {
        let idx = self.data.len() as u32;
        if len > u32::MAX {
            panic!("struct too long");
        }

        let var = Var { idx, len };
        self.data.resize((idx + len) as usize, 0);
        let var_idx = self.vars.len() as u32 + 1;
        self.vars.push(var);
        return var_idx;
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
        let len = (len - 1) as usize;
        if len > self.vars.len() {
            panic!("shrinking to a length larger than the vars array");
        }

        self.vars.resize(len, Var { idx: 0, len: 0 });
        if let Some(var) = self.vars.last() {
            self.data.resize((var.idx + var.len) as usize, 0);
        } else {
            self.data.resize(0, 0);
        }
    }

    pub fn push<T: Copy>(&mut self, value: T) {
        self.data.extend_from_slice(any_as_u8_slice(&value));
    }

    pub fn pop<T: Default + Copy>(&mut self) -> Result<T, Error> {
        if self.data.len() < 8 {
            return err!("StackIsEmpty", "tried to pop from stack when it is empty");
        }

        let mut out = T::default();
        let to_bytes = unsafe { any_as_u8_slice_mut(&mut out) };

        if let Some(var) = self.vars.last() {
            let upper = (var.idx + var.len) as usize;
            if self.data.len() - upper < 8 {
                return err!(
                    "StackPopInvalidatesVariable",
                    "popping from the stack would invalidate a variable"
                );
            }
        }

        let upper = self.data.len();
        let lower = upper - 8;

        to_bytes.copy_from_slice(&self.data[lower..upper]);
        self.data.resize(lower, 0);
        return Ok(out);
    }

    pub fn set<T: Copy>(&mut self, ptr: VarPointer, t: T) -> Result<(), Error> {
        let len = mem::size_of::<T>();
        if len > u32::MAX as usize {
            panic!("struct too long");
        }

        let to_bytes = self.get_var_range_mut(ptr, len as u32)?;
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
    pub callstack: Vec<CallFrame>,
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

    pub fn fp_offset(fp: u32, var: i32) -> u32 {
        if var < 0 {
            let var = (var * -1) as u32;
            fp - var
        } else {
            fp + var as u32
        }
    }

    pub fn get_var<T: Default + Copy>(&self, ptr: VarPointer) -> Result<T, Error> {
        if ptr.is_stack() {
            return self.stack.get_var(ptr);
        } else {
            return self.heap.get_var(ptr);
        }
    }

    pub fn get_var_range(&self, ptr: VarPointer, len: u32) -> Result<&[u8], Error> {
        if ptr.is_stack() {
            return self.stack.get_var_range(ptr, len);
        } else {
            return self.heap.get_var_range(ptr, len);
        }
    }

    pub fn get_var_range_mut(&mut self, ptr: VarPointer, len: u32) -> Result<&mut [u8], Error> {
        if ptr.is_stack() {
            return self.stack.get_var_range_mut(ptr, len);
        } else {
            return self.heap.get_var_range_mut(ptr, len);
        }
    }

    pub fn set<T: Copy>(&mut self, ptr: VarPointer, value: T) -> Result<(), Error> {
        if ptr.is_stack() {
            return self.stack.set(ptr, value);
        } else {
            return self.heap.set(ptr, value);
        }
    }

    pub fn run_program(&mut self, program: Program) -> Result<(), Error> {
        match self.run_func(&program, 0) {
            Ok(()) => Ok(()),
            Err(mut err) => {
                err.stack_trace.reserve(self.callstack.len());
                for callframe in self.callstack.iter() {
                    err.stack_trace.push(*callframe);
                }

                return Err(err);
            }
        }
    }

    pub fn run_func(&mut self, program: &Program, pcounter: usize) -> Result<(), Error> {
        let func_desc = match program.ops[pcounter] {
            Opcode::Func(desc) => desc,
            op => {
                return err!(
                    "InvalidFunctionHeader",
                    "found function header {:?} (this is an error in your compiler)",
                    op
                )
            }
        };

        let callstack_len = self.callstack.len();

        let fp = self.stack.upper_bound();
        let mut pc = pcounter + 1;

        loop {
            let op = program.ops[pc];
            if DEBUG {
                println!("op: {:?}", op);
            }

            let should_return = self.run_op(fp, &program, func_desc, op)?;

            if DEBUG {
                println!("stack: {:?}", self.stack.data);
                println!("heap: {:?}", self.heap.data);
            }

            if should_return {
                self.stack.shrink_vars_to(fp);
                self.callstack.resize(
                    callstack_len,
                    CallFrame {
                        file: 0,
                        name: 0,
                        line: 0,
                    },
                );
                return Ok(());
            }

            pc += 1;
        }
    }

    #[inline]
    pub fn run_op(
        &mut self,
        fp: u32,
        program: &Program,
        func_desc: FuncDesc,
        opcode: Opcode,
    ) -> Result<bool, Error> {
        match opcode {
            Opcode::Func(_) => {}

            Opcode::StackAlloc(space) => {
                self.stack.add_var_dyn(space);
            }
            Opcode::StackAllocPtr(space) => {
                let var = self.stack.add_var_dyn(space);
                self.stack.push(VarPointer::new_stack(var, 0));
            }
            Opcode::Alloc(space) => {
                let var = self.heap.add_var_dyn(space);
                self.stack.push(VarPointer::new_heap(var, 0));
            }

            Opcode::MakeTempInt64(value) => self.stack.push(value),
            Opcode::LoadStr(idx) => {
                let str_value = program.strings[idx as usize].as_bytes();
                let str_len = str_value.len() as u32;

                let idx = self.heap.add_var_dyn(str_len + 1);
                let ptr = VarPointer::new_heap(idx, 0);
                self.heap
                    .get_var_range_mut(ptr, str_len)?
                    .copy_from_slice(str_value);

                let end_ptr = VarPointer::new_heap(idx, str_len as i32);
                self.heap.get_var_range_mut(end_ptr, 1)?[0] = 0;

                self.stack.push(ptr);
            }

            Opcode::GetLocal64 { var, offset, .. } => {
                let ptr = VarPointer::new_stack(Self::fp_offset(fp, var), offset as i32);
                self.stack.push(self.stack.get_var::<u64>(ptr)?);
            }
            Opcode::SetLocal64 { var, offset, .. } => {
                let ptr = VarPointer::new_stack(Self::fp_offset(fp, var), offset as i32);
                let word: u64 = self.stack.pop()?;
                self.stack.set(ptr, word)?;
            }
            Opcode::GetLocal32 { var, offset, .. } => {
                let ptr = VarPointer::new_stack(Self::fp_offset(fp, var), offset as i32);
                self.stack.push(self.stack.get_var::<u32>(ptr)?);
            }
            Opcode::SetLocal32 { var, offset, .. } => {
                let ptr = VarPointer::new_stack(Self::fp_offset(fp, var), offset as i32);
                let word: u32 = self.stack.pop()?;
                self.stack.set(ptr, word)?;
            }
            Opcode::GetLocal16 { var, offset, .. } => {
                let ptr = VarPointer::new_stack(Self::fp_offset(fp, var), offset as i32);
                self.stack.push(self.stack.get_var::<u16>(ptr)?);
            }
            Opcode::SetLocal16 { var, offset, .. } => {
                let ptr = VarPointer::new_stack(Self::fp_offset(fp, var), offset as i32);
                let word: u16 = self.stack.pop()?;
                self.stack.set(ptr, word)?;
            }
            Opcode::GetLocal8 { var, offset, .. } => {
                let ptr = VarPointer::new_stack(Self::fp_offset(fp, var), offset as i32);
                self.stack.push(self.stack.get_var::<u8>(ptr)?);
            }
            Opcode::SetLocal8 { var, offset, .. } => {
                let ptr = VarPointer::new_stack(Self::fp_offset(fp, var), offset as i32);
                let byte = self.stack.pop::<u8>()?;
                self.stack.set(ptr, byte)?;
            }

            Opcode::Get64 { offset, .. } => {
                let mut ptr: VarPointer = self.stack.pop()?;
                ptr.offset += offset;
                self.stack.push(self.get_var::<u64>(ptr)?);
            }
            Opcode::Set64 { offset, .. } => {
                let mut ptr: VarPointer = self.stack.pop()?;
                ptr.offset += offset;
                let word = self.stack.pop::<u64>()?;
                self.set(ptr, word)?;
            }
            Opcode::Get32 { offset, .. } => {
                let mut ptr: VarPointer = self.stack.pop()?;
                ptr.offset += offset;
                self.stack.push(self.get_var::<u32>(ptr)?);
            }
            Opcode::Set32 { offset, .. } => {
                let mut ptr: VarPointer = self.stack.pop()?;
                ptr.offset += offset;
                let word = self.stack.pop::<u32>()?;
                self.set(ptr, word)?;
            }
            Opcode::Get16 { offset, .. } => {
                let mut ptr: VarPointer = self.stack.pop()?;
                ptr.offset += offset;
                self.stack.push(self.get_var::<u16>(ptr)?);
            }
            Opcode::Set16 { offset, .. } => {
                let mut ptr: VarPointer = self.stack.pop()?;
                ptr.offset += offset;
                let word = self.stack.pop::<u16>()?;
                self.set(ptr, word)?;
            }
            Opcode::Get8 { offset, .. } => {
                let mut ptr: VarPointer = self.stack.pop()?;
                ptr.offset += offset;
                self.stack.push(self.get_var::<u8>(ptr)?);
            }
            Opcode::Set8 { offset, .. } => {
                let mut ptr: VarPointer = self.stack.pop()?;
                ptr.offset += offset;
                let word = self.stack.pop::<u8>()?;
                self.set(ptr, word)?;
            }

            Opcode::Ret => return Ok(true),

            Opcode::AddCallstackDesc(desc) => self.callstack.push(desc),
            Opcode::RemoveCallstackDesc => {
                if self.callstack.pop().is_none() {
                    return err!(
                        "CallstackEmpty",
                        "tried to pop callstack when callstack was empty"
                    );
                }
            }

            Opcode::Call { func, line } => {
                self.callstack.push(CallFrame {
                    file: func_desc.file,
                    name: func_desc.name,
                    line,
                });

                self.run_func(program, func as usize)?;

                if self.callstack.pop().is_none() {
                    return err!(
                        "CallstackEmpty",
                        "tried to pop callstack when callstack was empty"
                    );
                }
            }

            Opcode::Ecall {
                call: ECALL_PRINT_INT,
                ..
            } => {
                let word: i64 = self.stack.pop()?;
                write!(self.stdout, "{}", word)
                    .map_err(|err| error!("WriteFailed", "failed to write to stdout ({})", err))?;
            }
            Opcode::Ecall {
                call: ECALL_PRINT_STR,
                ..
            } => {
                let ptr: VarPointer = self.stack.pop()?;
                let str_bytes = if ptr.is_stack() {
                    self.stack.get_full_var_range(ptr)?
                } else {
                    self.heap.get_full_var_range(ptr)?
                };

                let mut idx = str_bytes.len();
                for (idx_, byte) in str_bytes.iter().enumerate() {
                    if *byte == 0 {
                        idx = idx_;
                        break;
                    }
                }

                if idx == str_bytes.len() {
                    return err!("MissingNullTerminator", "string missing null terminator");
                }

                let str_value = unsafe { str::from_utf8_unchecked(&str_bytes[0..idx]) };

                write!(self.stdout, "{}", str_value)
                    .map_err(|err| error!("WriteFailed", "failed to write to stdout ({})", err))?;
            }
            Opcode::Ecall { call, .. } => {
                return err!("InvalidEnviromentCall", "invalid ecall value of {}", call);
            }
        }

        return Ok(false);
    }
}

#[test]
fn simple_read_write() {
    let files = map! {
        "main.c".to_string() => map! {
            "main".to_string() => vec![
                PseudoOp::Call { file: "main.c".to_string(), func: "helper".to_string(), line: 1 },
                PseudoOp::Ret,
            ],
            "helper".to_string() => vec![
                PseudoOp::StackAlloc(8),
                PseudoOp::LoadString("hello".to_string()),
                PseudoOp::SetLocal64 { var: 0, offset: 0, line: 2 },
                PseudoOp::GetLocal64 { var: 0, offset: 0, line: 3 },
                PseudoOp::Ecall { call: ECALL_PRINT_STR, line: 4 },
                PseudoOp::Ret,
            ]
        }
    };

    let program = Program::new(files);
    let mut stdout = StringWriter::new();
    let mut runtime = Runtime::new(&mut stdout);
    runtime.run_program(program).expect("why did this fail?");

    assert_eq!(stdout.to_string(), "hello");
}
