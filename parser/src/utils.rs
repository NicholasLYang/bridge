use crate::ast::{Type, TypeId};
use bimap::BiMap;

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
