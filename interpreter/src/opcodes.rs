use crate::util::*;
use codegenerator::opcodes::*;
use core::str;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum Opcode {
    Func(FuncDesc), // Function header used for callstack manipulation

    StackAlloc(u32),    // Allocates space on the stack
    StackAllocPtr(u32), // Allocates space on the stack, then pushes a pointer to that space onto the stack
    Alloc(u32), // Allocates space on the heap, then pushes a pointer to that space onto the stack

    MakeTempIntWord(i64), // Make a temporary integer
    LoadStr(u32),

    GetLocalWord { var: i32, offset: u32, line: u32 }, // Reads a word from a variable on the stack
    SetLocalWord { var: i32, offset: u32, line: u32 }, // Pops a temporary word off the stack, then sets section of variable on the stack to that value
    GetWord { offset: i32, line: u32 }, // Pops a temporary word off the stack, then reads memory at that word's location
    SetWord { offset: i32, line: u32 }, // Pops a temporary word off the stack, then sets the value at that word's location to the value of the next word on the stack (which is also popped)

    Ret, // Returns to caller

    AddCallstackDesc(CallFrame),
    RemoveCallstackDesc,

    Call { func: u32, line: u32 },
    Ecall { call: u32, line: u32 },
}

impl Opcode {
    pub fn line(self) -> u32 {
        match self {
            Opcode::GetLocalWord { line, .. } => line,
            Opcode::SetLocalWord { line, .. } => line,
            Opcode::GetWord { line, .. } => line,
            Opcode::SetWord { line, .. } => line,
            Opcode::Ecall { line, .. } => line,
            Opcode::Call { line, .. } => line,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Program<'a> {
    pub files: &'a [&'a str],
    pub strings: &'a [&'a str],
    pub functions: &'a [&'a str],
    pub ops: &'a [Opcode],
    unused: (), // prevents construction without Program::new
}

impl<'a> Program<'a> {
    pub fn new(program: HashMap<String, HashMap<String, Vec<PseudoOp>>>) -> Self {
        let mut bytes = Vec::new();
        let mut file_ranges = Vec::new();
        let mut string_ranges = Vec::new();
        let mut function_ranges = Vec::new();
        let mut pseudo_functions = Vec::new();

        for (file_number, (file, functions_)) in program.into_iter().enumerate() {
            let start = bytes.len();
            bytes.extend_from_slice(file.as_bytes());
            file_ranges.push(start..bytes.len());

            for (name, ops) in functions_.into_iter() {
                let start = bytes.len();
                bytes.extend_from_slice(name.as_bytes());
                function_ranges.push(start..bytes.len());
                let file = file_number as u32;
                pseudo_functions.push((file, ops));
            }
        }

        let mut ops = Vec::new();
        for (name, (file, body)) in pseudo_functions.into_iter().enumerate() {
            let name = name as u32;
            ops.push(Opcode::Func(FuncDesc { file, name }));

            for op in body.into_iter() {
                let op = match op {
                    PseudoOp::StackAlloc(space) => Opcode::StackAlloc(space),
                    PseudoOp::StackAllocPtr(space) => Opcode::StackAllocPtr(space),
                    PseudoOp::Alloc(space) => Opcode::StackAllocPtr(space),

                    PseudoOp::MakeTempIntWord(int) => Opcode::MakeTempIntWord(int),
                    PseudoOp::LoadString(string) => {
                        let string_index = string_ranges.len();
                        let start = bytes.len();
                        bytes.extend_from_slice(string.as_bytes());
                        string_ranges.push(start..bytes.len());
                        Opcode::LoadStr(string_index as u32)
                    }

                    PseudoOp::GetLocalWord { var, offset, line } => {
                        Opcode::GetLocalWord { var, offset, line }
                    }
                    PseudoOp::SetLocalWord { var, offset, line } => {
                        Opcode::SetLocalWord { var, offset, line }
                    }
                    PseudoOp::GetWord { offset, line } => Opcode::GetWord { offset, line },
                    PseudoOp::SetWord { offset, line } => Opcode::SetWord { offset, line },

                    PseudoOp::Ret => Opcode::Ret,

                    PseudoOp::AddCallstackDesc(desc) => Opcode::AddCallstackDesc(desc),
                    PseudoOp::RemoveCallstackDesc => Opcode::RemoveCallstackDesc,

                    PseudoOp::Call { func, line } => Opcode::Call { func, line },
                    PseudoOp::Ecall { call, line } => Opcode::Ecall { call, line },
                };

                ops.push(op);
            }
        }

        let data: &[u8] = Box::leak(bytes.into());
        let mut refs = Vec::new();

        let mut files: Vec<&str> = file_ranges
            .into_iter()
            .map(|range| unsafe { str::from_utf8_unchecked(&data[range]) })
            .collect();
        let start = refs.len();
        refs.append(&mut files);
        let files_range = start..refs.len();

        let mut strings: Vec<&str> = string_ranges
            .into_iter()
            .map(|range| unsafe { str::from_utf8_unchecked(&data[range]) })
            .collect();
        let start = refs.len();
        refs.append(&mut strings);
        let strings_range = start..refs.len();

        let mut functions: Vec<&str> = function_ranges
            .into_iter()
            .map(|range| unsafe { str::from_utf8_unchecked(&data[range]) })
            .collect();
        let start = refs.len();
        refs.append(&mut functions);
        let functions_range = start..refs.len();

        let refs: &[&str] = Box::leak(refs.into());
        let files = &refs[files_range];
        let strings = &refs[strings_range];
        let functions = &refs[functions_range];
        let ops: &[Opcode] = Box::leak(ops.into());

        Self {
            files,
            strings,
            functions,
            ops,
            unused: (),
        }
    }
}
