pub mod opcodes;
pub mod runtime;
pub mod util;

use core::mem;

fn main() {
    if mem::size_of::<usize>() != mem::size_of::<u64>() {
        panic!("this interpreter can only be run on 64-bit machines");
    }

    println!("Hello, world!");
}
