use serde::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CallFrame {
    pub file: u32,
    pub name: u32,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PseudoOp {
    StackAlloc(u32),
    StackAllocPtr(u32),
    Alloc(u32),

    MakeTempIntWord(i64),
    LoadString(String),

    GetLocalWord { var: i32, offset: u32, line: u32 },
    SetLocalWord { var: i32, offset: u32, line: u32 },
    GetWord { offset: i32, line: u32 },
    SetWord { offset: i32, line: u32 },

    Ret,

    AddCallstackDesc(CallFrame),
    RemoveCallstackDesc,

    Call { func: u32, line: u32 },
    Ecall { call: u32, line: u32 },
}
