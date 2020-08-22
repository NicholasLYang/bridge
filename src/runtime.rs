use crate::utils::*;
use core::{fmt, mem, str};
use serde::{Deserialize, Serialize};
use std::io::{Stderr, Stdout, Write};

#[derive(Debug)]
pub struct IError {
    pub short_name: String,
    pub message: String,
}

impl IError {
    pub fn new(short_name: &str, message: String) -> Self {
        Self {
            short_name: short_name.to_string(),
            message,
        }
    }
}

macro_rules! error {
    ($arg1:tt,$($arg:tt)*) => {
        IError::new($arg1, format!($($arg)*))
    };
}

macro_rules! err {
    ($arg1:tt,$($arg:tt)*) => {
        Err(IError::new($arg1, format!($($arg)*)))
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Var {
    pub idx: usize,
    pub len: u32, // len in bytes
    pub meta: u32,
}

impl Var {
    pub fn new() -> Self {
        Self {
            idx: 0,
            len: 0,
            meta: 0,
        }
    }

    pub fn upper(self) -> usize {
        self.idx + self.len as usize
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VarPointer {
    _idx: u32,
    _offset: u32,
}

impl fmt::Display for VarPointer {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        return write!(formatter, "0x{:x}{:0>8x}", self._idx, self._offset);
    }
}

impl fmt::Debug for VarPointer {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        return write!(formatter, "0x{:x}{:0>8x}", self._idx, self._offset);
    }
}

impl Into<u64> for VarPointer {
    fn into(self) -> u64 {
        ((self._idx as u64) << 32) + self._offset as u64
    }
}

impl From<u64> for VarPointer {
    fn from(val: u64) -> Self {
        Self {
            _idx: (val >> 32) as u32,
            _offset: val as u32,
        }
    }
}

impl VarPointer {
    pub fn new_stack(idx: u32, offset: u32) -> VarPointer {
        if idx & !(1u32 << 31) != idx {
            panic!("idx is too large");
        }

        let idx = idx | (1u32 << 31);
        Self {
            _idx: idx,
            _offset: offset,
        }
    }

    pub fn new_heap(idx: u32, offset: u32) -> VarPointer {
        if idx & !(1u32 << 31) != idx {
            panic!("idx is too large");
        }

        Self {
            _idx: idx,
            _offset: offset,
        }
    }

    pub fn var_idx(self) -> usize {
        (self._idx & !(1u32 << 31)) as usize
    }

    pub fn with_offset(self, offset: u32) -> Self {
        let mut ptr = self;
        ptr.set_offset(offset);
        return ptr;
    }

    pub fn offset(self) -> u32 {
        self._offset
    }

    pub fn set_offset(&mut self, offset: u32) {
        self._offset = offset;
    }

    pub fn is_stack(self) -> bool {
        self._idx & (1u32 << 31) != 0
    }
}

pub fn invalid_ptr(ptr: VarPointer) -> IError {
    return error!("InvalidPointer", "the pointer {} is invalid", ptr);
}

pub fn invalid_offset(var: Var, ptr: VarPointer) -> IError {
    let (start, end) = (ptr.with_offset(0), ptr.with_offset(var.len));
    return error!(
        "InvalidPointer",
        "the pointer {} is invalid; the nearest object is in the range {}..{}", ptr, start, end
    );
}

#[derive(Debug, Clone, PartialEq)]
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

    pub fn new_from(data: Vec<u8>, vars: Vec<Var>) -> Self {
        Self { data, vars }
    }

    pub fn get_var_range(&mut self, ptr: VarPointer, len: u32) -> Result<(usize, usize), IError> {
        if ptr.var_idx() == 0 {
            return Err(invalid_ptr(ptr));
        }

        let var = match self.vars.get(ptr.var_idx() - 1) {
            Some(x) => *x,
            None => return Err(invalid_ptr(ptr)),
        };

        if ptr.offset() >= var.len {
            return Err(invalid_offset(var, ptr));
        }

        if ptr.offset() + len > var.len {
            return Err(invalid_offset(var, ptr.with_offset(ptr.offset() + len)));
        }

        let start = var.idx + ptr.offset() as usize;
        return Ok((start, start + len as usize));
    }

    pub fn get_var<T: Copy>(&self, ptr: VarPointer) -> Result<T, IError> {
        let len = mem::size_of::<T>() as u32; // TODO check for overflow
        if ptr.var_idx() == 0 {
            return Err(invalid_ptr(ptr));
        }

        let var = match self.vars.get(ptr.var_idx() - 1) {
            Some(x) => *x,
            None => return Err(invalid_ptr(ptr)),
        };

        if ptr.offset() + len > var.len {
            return Err(invalid_offset(var, ptr));
        }

        let begin = var.idx + ptr.offset() as usize;
        let var_slice = &self.data[begin..(begin + len as usize)];
        return Ok(unsafe { *(var_slice.as_ptr() as *const T) });
    }

    pub fn add_var(&mut self, len: u32) -> u32 {
        let idx = self.data.len();
        self.vars.push(Var { idx, len, meta: 0 });
        self.data.resize(idx + len as usize, 0);
        let var_idx = self.vars.len() as u32; // TODO Check for overflow
        return var_idx;
    }

    pub fn set<T: Copy>(&mut self, ptr: VarPointer, t: T) -> Result<T, IError> {
        let len = mem::size_of::<T>() as u32; // TODO check for overflow
        if ptr.var_idx() == 0 {
            return Err(invalid_ptr(ptr));
        }

        let var = match self.vars.get(ptr.var_idx() - 1) {
            Some(x) => *x,
            None => return Err(invalid_ptr(ptr)),
        };

        if ptr.offset() + len > var.len {
            return Err(invalid_offset(var, ptr));
        }

        let begin = var.idx + ptr.offset() as usize;
        let to_bytes = &mut self.data[begin..(begin + len as usize)];
        let previous_value = unsafe { *(to_bytes.as_ptr() as *const T) };
        to_bytes.copy_from_slice(any_as_u8_slice(&t));
        return Ok(previous_value);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MAKind {
    SetValue {
        ptr: VarPointer,
        value_start: usize,
        value_end_overwrite_start: usize,
        overwrite_end: usize,
    },
    PopStack {
        value_start: usize,
        value_end: usize,
    },
    PushStack {
        value_start: usize,
        value_end: usize,
    },
    PopStackVar {
        var_start: usize,
        var_end_stack_start: usize,
        stack_end: usize,
    },
    AllocStackVar {
        len: u32,
    },
    AllocHeapVar {
        len: u32,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryAction<Tag: Copy> {
    pub kind: MAKind,
    pub tag: Tag,
}

pub struct Memory<Tag: Copy> {
    pub stack: VarBuffer,
    pub heap: VarBuffer,
    pub historical_data: Vec<u8>,
    pub history: Vec<MemoryAction<Tag>>,
}

impl<Tag: Copy> Memory<Tag> {
    pub fn new() -> Self {
        Self {
            stack: VarBuffer::new(),
            heap: VarBuffer::new(),
            historical_data: Vec::new(),
            history: Vec::new(),
        }
    }

    pub fn push_history(&mut self, kind: MAKind, tag: Tag) {
        self.history.push(MemoryAction { kind, tag });
    }

    #[inline]
    pub fn get_var_slice(&self, ptr: VarPointer) -> Result<&[u8], IError> {
        let buffer;
        if ptr.is_stack() {
            buffer = &self.stack;
        } else {
            buffer = &self.heap;
        }

        if ptr.var_idx() == 0 {
            return Err(invalid_ptr(ptr));
        }

        let var = match buffer.vars.get(ptr.var_idx() - 1) {
            Some(x) => *x,
            None => return Err(invalid_ptr(ptr)),
        };

        if ptr.offset() >= var.len {
            return Err(invalid_offset(var, ptr));
        }

        return Ok(&buffer.data[(var.idx + ptr.offset() as usize)..(var.idx + var.len as usize)]);
    }

    #[inline]
    pub fn get_slice(&self, ptr: VarPointer, len: u32) -> Result<&[u8], IError> {
        let buffer;
        if ptr.is_stack() {
            buffer = &self.stack;
        } else {
            buffer = &self.heap;
        }

        if ptr.var_idx() == 0 {
            return Err(invalid_ptr(ptr));
        }

        let var = match buffer.vars.get(ptr.var_idx() - 1) {
            Some(x) => *x,
            None => return Err(invalid_ptr(ptr)),
        };

        if ptr.offset() >= var.len {
            return Err(invalid_offset(var, ptr));
        }

        if ptr.offset() + len > var.len {
            return Err(invalid_offset(var, ptr.with_offset(ptr.offset() + len)));
        }

        return Ok(&buffer.data[(var.idx + ptr.offset() as usize)..((ptr.offset() + len) as usize)]);
    }

    #[inline]
    pub fn get_var<T: Default + Copy>(&self, ptr: VarPointer) -> Result<T, IError> {
        if ptr.is_stack() {
            return self.stack.get_var(ptr);
        } else {
            return self.heap.get_var(ptr);
        }
    }

    #[inline]
    pub fn set<T: Copy>(&mut self, ptr: VarPointer, value: T, tag: Tag) -> Result<(), IError> {
        let value_start = self.historical_data.len();
        self.historical_data
            .extend_from_slice(any_as_u8_slice(&value));

        let previous_value;
        if ptr.is_stack() {
            previous_value = self.stack.set(ptr, value)?;
        } else {
            previous_value = self.heap.set(ptr, value)?;
        }

        let value_end_overwrite_start = self.historical_data.len();
        self.historical_data
            .extend_from_slice(any_as_u8_slice(&previous_value));
        let overwrite_end = self.historical_data.len();
        self.push_history(
            MAKind::SetValue {
                ptr,
                value_start,
                value_end_overwrite_start,
                overwrite_end,
            },
            tag,
        );

        return Ok(());
    }

    #[inline]
    pub fn add_stack_var(&mut self, len: u32, tag: Tag) -> VarPointer {
        let ptr = VarPointer::new_stack(self.stack.add_var(len), 0);
        self.push_history(MAKind::AllocStackVar { len }, tag);
        return ptr;
    }

    #[inline]
    pub fn add_heap_var(&mut self, len: u32, tag: Tag) -> VarPointer {
        let ptr = VarPointer::new_heap(self.heap.add_var(len), 0);
        self.push_history(MAKind::AllocHeapVar { len }, tag);
        return ptr;
    }

    #[inline]
    pub fn write_bytes(&mut self, ptr: VarPointer, bytes: &[u8], tag: Tag) -> Result<(), IError> {
        let value_start = self.historical_data.len();
        self.historical_data.extend_from_slice(bytes);

        let to_bytes = if ptr.is_stack() {
            let (start, end) = self.stack.get_var_range(ptr, bytes.len() as u32)?;
            &mut self.stack.data[start..end]
        } else {
            let (start, end) = self.heap.get_var_range(ptr, bytes.len() as u32)?;
            &mut self.heap.data[start..end]
        };

        let value_end_overwrite_start = self.historical_data.len();
        self.historical_data.extend_from_slice(to_bytes);
        let overwrite_end = self.historical_data.len();
        to_bytes.copy_from_slice(bytes);
        self.push_history(
            MAKind::SetValue {
                ptr,
                value_start,
                value_end_overwrite_start,
                overwrite_end,
            },
            tag,
        );

        return Ok(());
    }

    #[inline]
    pub fn stack_length(&self) -> u32 {
        return (self.stack.vars.len() + 1) as u32; // TODO check for overflow
    }

    pub fn pop_stack_var(&mut self, tag: Tag) -> Result<Var, IError> {
        if let Some(var) = self.stack.vars.pop() {
            let var_start = self.historical_data.len();
            let var_end_stack_start = var_start + var.len as usize;
            self.historical_data
                .extend_from_slice(&self.stack.data[var.idx..]);
            let stack_end = self.historical_data.len();

            self.stack.data.resize(var.idx, 0);
            self.push_history(
                MAKind::PopStackVar {
                    var_start,
                    var_end_stack_start,
                    stack_end,
                },
                tag,
            );

            return Ok(var);
        } else {
            return err!("StackIsEmpty", "tried to pop from stack when it is empty");
        }
    }

    #[inline]
    pub fn push_stack<T: Copy>(&mut self, value: T, tag: Tag) {
        let from_bytes = any_as_u8_slice(&value);
        let value_start = self.historical_data.len();
        self.historical_data.extend_from_slice(from_bytes);
        let value_end = self.historical_data.len();

        self.stack.data.extend_from_slice(from_bytes);
        self.push_history(
            MAKind::PushStack {
                value_start,
                value_end,
            },
            tag,
        );
    }

    pub fn push_stack_bytes(&mut self, from_bytes: &[u8], tag: Tag) {
        let value_start = self.historical_data.len();
        self.historical_data.extend_from_slice(from_bytes);
        let value_end = self.historical_data.len();

        self.stack.data.extend_from_slice(from_bytes);
        self.push_history(
            MAKind::PushStack {
                value_start,
                value_end,
            },
            tag,
        );
    }

    pub fn pop_stack_bytes_into(
        &mut self,
        ptr: VarPointer,
        len: u32,
        tag: Tag,
    ) -> Result<(), IError> {
        if self.stack.data.len() < len as usize {
            return err!(
                "StackTooShort",
                "tried to pop {} bytes from stack when stack is only {} bytes long",
                len,
                self.stack.data.len(),
            );
        }

        let break_idx = if let Some(var) = self.stack.vars.last() {
            if self.stack.data.len() - var.upper() < len as usize {
                return err!(
                    "StackPopInvalidatesVariable",
                    "popping from the stack would invalidate a variable"
                );
            }
            var.upper()
        } else {
            0
        };

        let (to_bytes, from_bytes) = if ptr.is_stack() {
            let (start, end) = self.stack.get_var_range(ptr, len)?;
            let (stack_vars, stack) = self.stack.data.split_at_mut(break_idx);
            let pop_lower = stack.len() - len as usize;

            (&mut stack_vars[start..end], &stack[pop_lower..])
        } else {
            let (start, end) = self.heap.get_var_range(ptr, len)?;
            let (stack_vars, stack) = self.stack.data.split_at_mut(break_idx);
            let pop_lower = stack.len() - len as usize;

            (&mut self.heap.data[start..end], &stack[pop_lower..])
        };

        let value_start = self.historical_data.len();
        self.historical_data.extend_from_slice(from_bytes);
        let value_end = self.historical_data.len();
        self.historical_data.extend_from_slice(to_bytes);
        let overwrite_end = self.historical_data.len();
        to_bytes.copy_from_slice(from_bytes);
        self.push_history(
            MAKind::SetValue {
                ptr,
                value_start,
                value_end_overwrite_start: value_end,
                overwrite_end,
            },
            tag,
        );

        self.stack
            .data
            .resize(self.stack.data.len() - len as usize, 0);
        self.push_history(
            MAKind::PopStack {
                value_start,
                value_end,
            },
            tag,
        );

        return Ok(());
    }

    pub fn push_stack_bytes_from(
        &mut self,
        ptr: VarPointer,
        len: u32,
        tag: Tag,
    ) -> Result<(), IError> {
        let break_idx = if let Some(var) = self.stack.vars.last() {
            var.upper()
        } else {
            0
        };

        let data = &mut self.stack.data;
        data.resize(data.len() + len as usize, 0);

        let (from_bytes, stack) = if ptr.is_stack() {
            let (start, end) = self.stack.get_var_range(ptr, len)?;
            let (stack_vars, stack) = self.stack.data.split_at_mut(break_idx);
            (&stack_vars[start..end], stack)
        } else {
            let (start, end) = self.heap.get_var_range(ptr, len)?;
            let (stack_vars, stack) = self.stack.data.split_at_mut(break_idx);
            (&self.heap.data[start..end], stack)
        };

        let pop_lower = stack.len() - len as usize;
        let to_bytes = &mut stack[pop_lower..];

        let value_start = self.historical_data.len();
        self.historical_data.extend_from_slice(from_bytes);
        let value_end = self.historical_data.len();
        to_bytes.copy_from_slice(from_bytes);
        self.push_history(
            MAKind::PushStack {
                value_start,
                value_end,
            },
            tag,
        );

        return Ok(());
    }

    pub fn pop_bytes(&mut self, len: u32, tag: Tag) -> Result<(), IError> {
        if self.stack.data.len() < len as usize {
            return err!(
                "StackTooShort",
                "tried to pop {} bytes from stack when stack is only {} bytes long",
                len,
                self.stack.data.len(),
            );
        }

        if let Some(var) = self.stack.vars.last() {
            if self.stack.data.len() - var.upper() < len as usize {
                return err!(
                    "StackPopInvalidatesVariable",
                    "popping from the stack would invalidate a variable"
                );
            }
        }

        let upper = self.stack.data.len();
        let lower = upper - len as usize;
        let from_bytes = &self.stack.data[lower..upper];

        let value_start = self.historical_data.len();
        self.historical_data.extend_from_slice(from_bytes);
        let value_end = self.historical_data.len();

        self.stack.data.resize(lower, 0);
        self.push_history(
            MAKind::PopStack {
                value_start,
                value_end,
            },
            tag,
        );

        return Ok(());
    }

    pub fn pop_keep_bytes(&mut self, keep: u32, pop: u32, tag: Tag) -> Result<(), IError> {
        let len = keep + pop;
        if self.stack.data.len() < len as usize {
            return err!(
                "StackTooShort",
                "tried to pop {} bytes from stack when stack is only {} bytes long",
                len,
                self.stack.data.len(),
            );
        }

        if let Some(var) = self.stack.vars.last() {
            if self.stack.data.len() - var.upper() < len as usize {
                return err!(
                    "StackPopInvalidatesVariable",
                    "popping from the stack would invalidate a variable"
                );
            }
        }

        let keep_start = self.stack.data.len() - keep as usize;
        let pop_start = keep_start - pop as usize;
        let pop_value_start = self.historical_data.len();
        self.historical_data
            .extend_from_slice(&self.stack.data[pop_start..]);
        let pop_value_end = self.historical_data.len();
        self.historical_data
            .extend_from_slice(&self.stack.data[keep_start..]);
        let push_value_end = self.historical_data.len();

        let mutate_slice = &mut self.stack.data[pop_start..];
        for i in 0..mutate_slice.len() {
            mutate_slice[i] = mutate_slice[i + pop as usize];
        }
        self.stack.data.resize(pop_start + keep as usize, 0);

        self.push_history(
            MAKind::PopStack {
                value_start: pop_value_start,
                value_end: pop_value_end,
            },
            tag,
        );
        self.push_history(
            MAKind::PushStack {
                value_start: pop_value_end,
                value_end: push_value_end,
            },
            tag,
        );

        return Ok(());
    }

    pub fn pop_stack<T: Copy>(&mut self, tag: Tag) -> Result<T, IError> {
        let len = mem::size_of::<T>();
        if self.stack.data.len() < len {
            return err!(
                "StackTooShort",
                "tried to pop {} bytes from stack when stack is only {} bytes long",
                len,
                self.stack.data.len(),
            );
        }

        if let Some(var) = self.stack.vars.last() {
            if self.stack.data.len() - var.upper() < len {
                return err!(
                    "StackPopInvalidatesVariable",
                    "popping from the stack would invalidate a variable"
                );
            }
        }

        let upper = self.stack.data.len();
        let lower = upper - len;
        let from_bytes = &self.stack.data[lower..upper];

        let value_start = self.historical_data.len();
        self.historical_data.extend_from_slice(from_bytes);
        let value_end = self.historical_data.len();

        let out = unsafe { *(from_bytes.as_ptr() as *const T) };
        self.stack.data.resize(lower, 0);
        self.push_history(
            MAKind::PopStack {
                value_start,
                value_end,
            },
            tag,
        );

        return Ok(out);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemorySnapshot<'a> {
    pub stack_data: &'a [u8],
    pub stack_vars: &'a [Var],
    pub heap_data: &'a [u8],
    pub heap_vars: &'a [Var],
}

#[derive(Debug, Clone)]
struct MockMemory {
    stack: VarBuffer,
    heap: VarBuffer,
}

impl MockMemory {
    pub fn new() -> Self {
        Self {
            stack: VarBuffer::new(),
            heap: VarBuffer::new(),
        }
    }

    pub fn new_from(stack: VarBuffer, heap: VarBuffer) -> Self {
        Self {
            stack: stack,
            heap: heap,
        }
    }

    pub fn var_buffer(&mut self, ptr: VarPointer) -> &mut VarBuffer {
        if ptr.is_stack() {
            &mut self.stack
        } else {
            &mut self.heap
        }
    }

    pub fn snapshot(&self) -> MemorySnapshot {
        MemorySnapshot {
            stack_data: &self.stack.data,
            stack_vars: &self.stack.vars,
            heap_data: &self.heap.data,
            heap_vars: &self.heap.vars,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemorySnapshotWalker<'a, Tag: Copy> {
    memory: MockMemory,
    historical_data: &'a [u8],
    history: &'a [MemoryAction<Tag>],
    index: usize,
}

impl<'a, Tag: Copy> MemorySnapshotWalker<'a, Tag> {
    pub fn next(&mut self) -> Option<MemorySnapshot> {
        if self.index > self.history.len() {
            return None;
        }

        if self.index > 0 {
            match self.history[self.index - 1].kind {
                MAKind::SetValue {
                    ptr,
                    value_start,
                    value_end_overwrite_start,
                    overwrite_end,
                } => {
                    let value_bytes = &self.historical_data[value_start..value_end_overwrite_start];
                    let buffer = self.memory.var_buffer(ptr);
                    let result = buffer.get_var_range(ptr, value_bytes.len() as u32);
                    let (start, end) = result.expect("this should never error");
                    buffer.data[start..end].copy_from_slice(value_bytes);
                }
                MAKind::PopStack {
                    value_start,
                    value_end,
                } => {
                    let popped_len = value_end - value_start;
                    let data = &mut self.memory.stack.data;
                    data.resize(data.len() - popped_len, 0);
                }
                MAKind::PushStack {
                    value_start,
                    value_end,
                } => {
                    let popped_len = value_end - value_start;
                    let data = &mut self.memory.stack.data;
                    data.extend_from_slice(&self.historical_data[value_start..value_end]);
                }
                MAKind::PopStackVar {
                    var_start,
                    var_end_stack_start,
                    stack_end,
                } => {
                    let var = self.memory.stack.vars.pop().unwrap();
                    self.memory.stack.data.resize(var.idx, 0);
                }
                MAKind::AllocHeapVar { len } => {
                    self.memory.heap.add_var(len);
                }
                MAKind::AllocStackVar { len } => {
                    self.memory.stack.add_var(len);
                }
            }
        }
        self.index += 1;

        Some(self.memory.snapshot())
    }

    pub fn prev(&mut self) -> Option<MemorySnapshot> {
        if self.index == 0 {
            return None;
        }

        if self.index <= self.history.len() {
            match self.history[self.index - 1].kind {
                MAKind::SetValue {
                    ptr,
                    value_start,
                    value_end_overwrite_start,
                    overwrite_end,
                } => {
                    let value_bytes =
                        &self.historical_data[value_end_overwrite_start..overwrite_end];
                    let buffer = self.memory.var_buffer(ptr);
                    let result = buffer.get_var_range(ptr, value_bytes.len() as u32);
                    let (start, end) = result.expect("this should never error");
                    buffer.data[start..end].copy_from_slice(value_bytes);
                }
                MAKind::PopStack {
                    value_start,
                    value_end,
                } => {
                    let popped_len = value_end - value_start;
                    let data = &mut self.memory.stack.data;
                    data.extend_from_slice(&self.historical_data[value_start..value_end]);
                }
                MAKind::PushStack {
                    value_start,
                    value_end,
                } => {
                    let popped_len = value_end - value_start;
                    let data = &mut self.memory.stack.data;
                    data.resize(data.len() - popped_len, 0);
                }
                MAKind::PopStackVar {
                    var_start,
                    var_end_stack_start,
                    stack_end,
                } => {
                    let data = &mut self.memory.stack.data;
                    let idx = data.len(); // TODO check for overflow
                    let len = (var_end_stack_start - var_start) as u32;
                    data.extend_from_slice(&self.historical_data[var_start..stack_end]);
                    let vars = &mut self.memory.stack.vars;
                    vars.push(Var { idx, len, meta: 0 });
                }
                MAKind::AllocHeapVar { len } => {
                    let var = self.memory.heap.vars.pop().unwrap();
                    self.memory.heap.data.resize(var.idx, 0);
                }
                MAKind::AllocStackVar { len } => {
                    let var = self.memory.stack.vars.pop().unwrap();
                    self.memory.stack.data.resize(var.idx, 0);
                }
            }
        }

        self.index -= 1;

        Some(self.memory.snapshot())
    }
}

impl<Tag: Copy> Memory<Tag> {
    pub fn forwards_walker(&self) -> MemorySnapshotWalker<Tag> {
        MemorySnapshotWalker {
            memory: MockMemory::new(),
            historical_data: &self.historical_data,
            history: &self.history,
            index: 0,
        }
    }

    pub fn backwards_walker(&self) -> MemorySnapshotWalker<Tag> {
        MemorySnapshotWalker {
            memory: MockMemory::new_from(self.stack.clone(), self.heap.clone()),
            historical_data: &self.historical_data,
            history: &self.history,
            index: self.history.len() + 1,
        }
    }
}

#[test]
fn test_walker() {
    let mut memory = Memory::new();
    let ptr = memory.add_stack_var(12, 0);
    memory.push_stack(12u64.to_be(), 0);
    memory.push_stack(4u32.to_be(), 0);
    memory
        .pop_stack_bytes_into(ptr, 12, 0)
        .expect("should not fail");

    println!("history: {:?}", memory.history);

    let mut walker = memory.forwards_walker();
    while let Some(snapshot) = walker.next() {
        println!("{:?}", snapshot.stack_data);
        println!("{:?}\n", snapshot.stack_vars);
    }

    let mut walker = memory.backwards_walker();

    let expected = MockMemory::new_from(
        VarBuffer::new_from(
            vec![0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 4],
            vec![Var {
                idx: 0,
                len: 12,
                meta: 0,
            }],
        ),
        VarBuffer::new(),
    );
    assert_eq!(walker.prev().unwrap(), expected.snapshot());

    let expected = MockMemory::new_from(
        VarBuffer::new_from(
            vec![
                0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 4,
            ],
            vec![Var {
                idx: 0,
                len: 12,
                meta: 0,
            }],
        ),
        VarBuffer::new(),
    );
    assert_eq!(walker.prev().unwrap(), expected.snapshot());

    let expected = MockMemory::new_from(
        VarBuffer::new_from(
            vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 4,
            ],
            vec![Var {
                idx: 0,
                len: 12,
                meta: 0,
            }],
        ),
        VarBuffer::new(),
    );
    assert_eq!(walker.prev().unwrap(), expected.snapshot());
    panic!();
}

pub trait RuntimeIO {
    type Out: Write;
    type Log: Write;
    type Err: Write;

    fn out(&mut self) -> &mut Self::Out;
    fn log(&mut self) -> &mut Self::Log;
    fn err(&mut self) -> &mut Self::Err;
}

pub struct InMemoryIO {
    pub out: StringWriter,
    pub log: StringWriter,
    pub err: StringWriter,
}

impl InMemoryIO {
    pub fn new() -> Self {
        Self {
            out: StringWriter::new(),
            log: StringWriter::new(),
            err: StringWriter::new(),
        }
    }
}

impl RuntimeIO for InMemoryIO {
    type Out = StringWriter;
    type Log = StringWriter;
    type Err = StringWriter;

    fn out(&mut self) -> &mut StringWriter {
        return &mut self.out;
    }
    fn err(&mut self) -> &mut StringWriter {
        return &mut self.err;
    }
    fn log(&mut self) -> &mut StringWriter {
        return &mut self.log;
    }
}

pub struct DefaultIO {
    pub out: Stdout,
    pub log: StringWriter,
    pub err: Stderr,
}

impl RuntimeIO for DefaultIO {
    type Out = Stdout;
    type Log = StringWriter;
    type Err = Stderr;

    fn out(&mut self) -> &mut Stdout {
        return &mut self.out;
    }
    fn log(&mut self) -> &mut StringWriter {
        return &mut self.log;
    }
    fn err(&mut self) -> &mut Stderr {
        return &mut self.err;
    }
}
