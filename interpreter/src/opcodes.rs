use crate::util::*;
use core::ops::Deref;
use core::str;

#[derive(Debug, Clone, Copy)]
pub enum Opcode {
    Func(FuncDesc), // Function header used for callstack manipulation

    StackAlloc(u32),    // Allocates space on the stack
    StackAllocPtr(u32), // Allocates space on the stack, then pushes a pointer to that space onto the stack
    Alloc(u32), // Allocates space on the heap, then pushes a pointer to that space onto the stack

    MakeTempIntWord(i64), // Make a temporary integer

    GetLocalWord { var_offset: i32, offset: u32 }, // Reads a word from a variable on the stack
    SetLocalWord { var_offset: i32, offset: u32 }, // Pops a temporary word off the stack, then sets section of variable on the stack to that value

    GetWord { offset: i32 }, // Pops a temporary word off the stack, then reads memory at that word's location
    SetWord { offset: i32 }, // Pops a temporary word off the stack, then sets the value at that word's location to the value of the next word on the stack (which is also popped)

    Ret, // Returns to caller

    AddCallstackDesc(FuncDesc),
    RemoveCallstackDesc,

    Ecall(u32),
}

#[derive(Debug, Clone, Copy)]
pub struct Program<'a> {
    pub data: &'a [u8],
    pub files: &'a [&'a str],
    pub strings: &'a [&'a str],
    pub functions: &'a [&'a str],
    pub ops: &'a [Opcode],
    unused: (), // prevents construction without Program::new
}

impl<'a> Program<'a> {
    pub fn new(
        files: Vec<impl Deref<Target = str>>,
        strings: Vec<impl Deref<Target = str>>,
        functions: Vec<impl Deref<Target = str>>,
        ops: Vec<Opcode>,
    ) -> Self {
        let mut bytes = Vec::new();
        let mut file_ranges = Vec::new();
        let mut string_ranges = Vec::new();
        let mut function_ranges = Vec::new();

        for file in files {
            let start = bytes.len();
            bytes.extend_from_slice(file.as_bytes());
            file_ranges.push(start..bytes.len());
        }

        for string in strings {
            let start = bytes.len();
            bytes.extend_from_slice(string.as_bytes());
            string_ranges.push(start..bytes.len());
        }

        for function in functions {
            let start = bytes.len();
            bytes.extend_from_slice(function.as_bytes());
            function_ranges.push(start..bytes.len());
        }

        let data: &[u8] = Box::leak(bytes.into());

        let files: Vec<&str> = file_ranges
            .into_iter()
            .map(|range| unsafe { str::from_utf8_unchecked(&data[range]) })
            .collect();
        let files: &[&str] = Box::leak(files.into());

        let strings: Vec<&str> = string_ranges
            .into_iter()
            .map(|range| unsafe { str::from_utf8_unchecked(&data[range]) })
            .collect();
        let strings: &[&str] = Box::leak(strings.into());

        let functions: Vec<&str> = function_ranges
            .into_iter()
            .map(|range| unsafe { str::from_utf8_unchecked(&data[range]) })
            .collect();
        let functions: &[&str] = Box::leak(functions.into());

        let ops: &[Opcode] = Box::leak(ops.into());

        Self {
            data,
            files,
            strings,
            functions,
            ops,
            unused: (),
        }
    }
}
