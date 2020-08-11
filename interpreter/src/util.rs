#[derive(Debug)]
pub struct FuncDesc {
    pub file: u32,
    pub line: u32,
    pub name: u32,
}

pub fn u32_to_u16_tup(value: u32) -> (u16, u16) {
    ((value >> 16) as u16, value as u16)
}

pub struct Error {
    pub message: String,
    pub stack_trace: Vec<FuncDesc>,
}
