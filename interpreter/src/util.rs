#[derive(Debug, Clone, Copy)]
pub struct FuncDesc {
    pub file: u32,
    pub line: u32,
    pub name: u32,
}

pub fn u32_to_u16_tup(value: u32) -> (u16, u16) {
    ((value >> 16) as u16, value as u16)
}

pub struct Error {
    pub short_name: String,
    pub message: String,
    pub stack_trace: Vec<FuncDesc>,
}

impl Error {
    pub fn new(short_name: &str, message: String) -> Self {
        Self {
            short_name: short_name.to_string(),
            message,
            stack_trace: Vec::new(),
        }
    }
}
