use crate::ast::{Type, TypeId};
use bimap::BiMap;
use codespan_reporting::term::termcolor::{ColorSpec, WriteColor};
use std::io;

pub fn any_as_u8_slice<T: Sized + Copy>(p: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(p as *const T as *const u8, std::mem::size_of::<T>()) }
}

pub struct StringWriter {
    buf: Vec<u8>,
}

impl StringWriter {
    pub fn new() -> StringWriter {
        StringWriter {
            buf: Vec::with_capacity(8 * 1024),
        }
    }

    pub fn to_string(&self) -> String {
        if let Ok(s) = String::from_utf8(self.buf.clone()) {
            s
        } else {
            String::new()
        }
    }
}

impl io::Write for StringWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for b in buf {
            self.buf.push(*b);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl WriteColor for StringWriter {
    fn supports_color(&self) -> bool {
        false
    }

    fn set_color(&mut self, _color: &ColorSpec) -> io::Result<()> {
        return Ok(());
    }

    fn reset(&mut self) -> io::Result<()> {
        return Ok(());
    }
}

#[derive(Debug)]
pub struct NameTable(BiMap<String, usize>, usize);

pub static PRINT_INDEX: usize = 0;

impl NameTable {
    pub fn new() -> Self {
        let mut map = BiMap::new();
        map.insert("print".to_string(), 0);
        NameTable(map, 1)
    }
    pub fn insert(&mut self, sym: String) -> usize {
        if let Some(id) = self.0.get_by_left(&sym) {
            *id
        } else {
            let id = self.1;
            self.0.insert(sym, id);
            self.1 += 1;
            id
        }
    }

    pub fn get_id(&self, sym: &String) -> Option<&usize> {
        self.0.get_by_left(sym)
    }

    pub fn get_str(&self, id: &usize) -> &str {
        self.0.get_by_right(id).unwrap()
    }

    pub fn contains_str(&self, str: &String) -> bool {
        self.0.get_by_left(str).is_some()
    }
}

// "Table" is a loose term here
pub struct TypeTable {
    table: Vec<Type>,
}

// NOTE: This is very brittle as if
// we change the initial vec in TypeTable
// these constants will break
pub static INT_INDEX: usize = 0;
pub static FLOAT_INDEX: usize = 1;
pub static CHAR_INDEX: usize = 2;
pub static STR_INDEX: usize = 3;
pub static BOOL_INDEX: usize = 4;
pub static UNIT_INDEX: usize = 5;
pub static ANY_INDEX: usize = 6;

impl TypeTable {
    pub fn new() -> TypeTable {
        TypeTable {
            table: vec![
                Type::Int,
                Type::Float,
                Type::Char,
                Type::String,
                Type::Bool,
                Type::Unit,
                Type::Any,
            ],
        }
    }

    pub fn insert(&mut self, type_: Type) -> TypeId {
        let index = self.table.len();
        self.table.push(type_);
        index
    }

    pub fn get_type(&self, id: TypeId) -> &Type {
        &self.table[id]
    }
}
