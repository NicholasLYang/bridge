use crate::util::*;
use core::ops::Deref;
use core::str;

#[derive(Debug, Clone, Copy)]
pub enum Opcode {
    Function(FuncDesc),
    MakeInt(i32),
}

#[derive(Debug, Clone, Copy)]
pub struct Program<'a> {
    pub data: &'a [u8],
    pub files: &'a [&'a str],
    pub strings: &'a [&'a str],
    pub ops: &'a [Opcode],
    unused: (), // prevents construction without Program::new
}

impl<'a> Program<'a> {
    pub fn new(
        files: Vec<impl Deref<Target = str>>,
        strings: Vec<impl Deref<Target = str>>,
        ops: Vec<Opcode>,
    ) -> Self {
        let mut bytes = Vec::new();
        let mut file_ranges = Vec::new();
        let mut string_ranges = Vec::new();

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

        let ops: &[Opcode] = Box::leak(ops.into());

        Self {
            data,
            files,
            strings,
            ops,
            unused: (),
        }
    }
}
