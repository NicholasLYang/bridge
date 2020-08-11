#[derive(Debug, Clone, Copy)]
pub struct FuncDesc {
    pub file: u32,
    pub line: u32,
    pub name: u32,
}

// https://stackoverflow.com/questions/28127165/how-to-convert-struct-to-u8
pub unsafe fn any_as_u8_slice_mut<T: Sized + Copy>(p: &mut T) -> &mut [u8] {
    std::slice::from_raw_parts_mut(p as *mut T as *mut u8, std::mem::size_of::<T>())
}

pub fn any_as_u8_slice<T: Sized + Copy>(p: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(p as *const T as *const u8, std::mem::size_of::<T>()) }
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
