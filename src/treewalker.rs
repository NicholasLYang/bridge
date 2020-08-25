use crate::ast::{ExprT, Function, Loc, Name, Op, ProgramT, StmtT, UnaryOp, Value};
use crate::lexer::LocationRange;
use crate::runtime::*;
use crate::utils::*;
use std::collections::HashMap;

// macro_rules! error {
//     ($arg1:tt,$($arg:tt)*) => {
//         IError::new($arg1, format!($($arg)*))
//     };
// }

macro_rules! err {
    ($arg1:tt,$($arg:tt)*) => {
        Err(IError::new($arg1, format!($($arg)*)))
    };
}

struct Scope {
    variables: HashMap<Name, u64>,
}

pub struct TreeWalker {
    memory: Memory<LocationRange>,
    scopes: Vec<Scope>,
    functions: HashMap<Name, Function>,
}

impl TreeWalker {
    pub fn new(functions: HashMap<Name, Function>) -> Self {
        TreeWalker {
            memory: Memory::new(),
            scopes: vec![Scope {
                variables: HashMap::new(),
            }],
            functions,
        }
    }

    pub fn interpret_program(&mut self, program: ProgramT) -> Result<(), IError> {
        for stmt in program.stmts {
            if let Some(val) = self.interpret_stmt(&stmt)? {
                return err!(
                    "InvalidReturn",
                    "return in place there shouldn't be a return"
                );
            }
        }

        Ok(())
    }

    fn lookup_in_scope(&self, name: &Name) -> Option<u64> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.variables.get(name) {
                return Some(*value);
            }
        }

        None
    }

    fn update_in_scope(&mut self, name: &Name, value: u64) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(val) = scope.variables.get_mut(name) {
                *val = value;
                return;
            }
        }

        panic!("assigned to variable that doesn't exist");
    }

    // returns whether or not to return
    fn interpret_stmt(&mut self, stmt: &Loc<StmtT>) -> Result<Option<u64>, IError> {
        match &stmt.inner {
            StmtT::Def(name, rhs) => {
                let rhs_val = self.interpret_expr(rhs)?;
                self.scopes
                    .last_mut()
                    .unwrap()
                    .variables
                    .insert(*name, rhs_val);
            }
            StmtT::Asgn(name, rhs) => {
                let rhs_val = self.interpret_expr(rhs)?;
                self.update_in_scope(name, rhs_val);
            }
            StmtT::Expr(expr) => {
                self.interpret_expr(expr)?;
            }
            StmtT::Function(_) => {}
            StmtT::Return(expr) => return Ok(Some(self.interpret_expr(expr)?)),
        }

        Ok(None)
    }

    fn interpret_expr(&mut self, expr: &Loc<ExprT>) -> Result<u64, IError> {
        match &expr.inner {
            ExprT::Primary { value, type_: _ } => self.interpret_value(value, expr.location),
            ExprT::BinOp {
                op,
                lhs,
                rhs,
                type_,
            } => {
                let l = self.interpret_expr(lhs)?;
                let r = self.interpret_expr(rhs)?;
                let (l_i, r_i) = (l as i64, r as i64);
                let (l_f, r_f) = (f64::from_bits(l), f64::from_bits(r));

                let result = match (op, lhs.inner.get_type(), rhs.inner.get_type()) {
                    (Op::Plus, INT_INDEX, INT_INDEX) => (l_i + r_i) as u64,
                    (Op::Plus, FLOAT_INDEX, INT_INDEX) => (l_f + r_i as f64).to_bits(),
                    (Op::Plus, INT_INDEX, FLOAT_INDEX) => (l_i as f64 + r_f).to_bits(),
                    (Op::Plus, FLOAT_INDEX, FLOAT_INDEX) => (l_f + r_f).to_bits(),

                    (Op::Minus, INT_INDEX, INT_INDEX) => (l_i - r_i) as u64,
                    (Op::Minus, FLOAT_INDEX, INT_INDEX) => (l_f - r_i as f64).to_bits(),
                    (Op::Minus, INT_INDEX, FLOAT_INDEX) => (l_i as f64 - r_f).to_bits(),
                    (Op::Minus, FLOAT_INDEX, FLOAT_INDEX) => (l_f - r_f).to_bits(),

                    (Op::Div, INT_INDEX, INT_INDEX) => (l_i / r_i) as u64,
                    (Op::Div, FLOAT_INDEX, INT_INDEX) => (l_f / r_i as f64).to_bits(),
                    (Op::Div, INT_INDEX, FLOAT_INDEX) => (l_i as f64 / r_f).to_bits(),
                    (Op::Div, FLOAT_INDEX, FLOAT_INDEX) => (l_f / r_f).to_bits(),

                    (Op::Times, INT_INDEX, INT_INDEX) => (l_i * r_i) as u64,
                    (Op::Times, FLOAT_INDEX, INT_INDEX) => (l_f * r_i as f64).to_bits(),
                    (Op::Times, INT_INDEX, FLOAT_INDEX) => (l_i as f64 * r_f).to_bits(),
                    (Op::Times, FLOAT_INDEX, FLOAT_INDEX) => (l_f * r_f).to_bits(),

                    // TODO should negative zero be equal to zero?
                    (Op::BangEqual, _, _) => (l != r) as u64,
                    (Op::EqualEqual, _, _) => (l == r) as u64,

                    (Op::Greater, INT_INDEX, INT_INDEX) => (l_i > r_i) as u64,
                    (Op::Greater, FLOAT_INDEX, INT_INDEX) => (l_f > r_i as f64) as u64,
                    (Op::Greater, INT_INDEX, FLOAT_INDEX) => (l_i as f64 > r_f) as u64,
                    (Op::Greater, FLOAT_INDEX, FLOAT_INDEX) => (l_f > r_f) as u64,

                    (Op::GreaterEqual, INT_INDEX, INT_INDEX) => (l_i >= r_i) as u64,
                    (Op::GreaterEqual, FLOAT_INDEX, INT_INDEX) => (l_f >= r_i as f64) as u64,
                    (Op::GreaterEqual, INT_INDEX, FLOAT_INDEX) => (l_i as f64 >= r_f) as u64,
                    (Op::GreaterEqual, FLOAT_INDEX, FLOAT_INDEX) => (l_f >= r_f) as u64,

                    (Op::Less, INT_INDEX, INT_INDEX) => (l_i < r_i) as u64,
                    (Op::Less, FLOAT_INDEX, INT_INDEX) => (l_f < r_i as f64) as u64,
                    (Op::Less, INT_INDEX, FLOAT_INDEX) => ((l_i as f64) < r_f) as u64,
                    (Op::Less, FLOAT_INDEX, FLOAT_INDEX) => (l_f < r_f) as u64,

                    (Op::LessEqual, INT_INDEX, INT_INDEX) => (l_i <= r_i) as u64,
                    (Op::LessEqual, FLOAT_INDEX, INT_INDEX) => (l_f <= r_i as f64) as u64,
                    (Op::LessEqual, INT_INDEX, FLOAT_INDEX) => ((l_i as f64) <= r_f) as u64,
                    (Op::LessEqual, FLOAT_INDEX, FLOAT_INDEX) => (l_f <= r_f) as u64,

                    _ => panic!("unexpected combination of operand types"),
                };

                return Ok(result);
            }
            ExprT::If(cond, then_clause, else_clause, _) => {
                let cond_val = self.interpret_expr(cond)?;
                if cond_val != 0 {
                    return self.interpret_expr(then_clause);
                } else if let Some(else_clause) = else_clause {
                    return self.interpret_expr(else_clause);
                } else {
                    return Ok(0);
                }
            }
            ExprT::Block {
                stmts,
                end_expr,
                scope_index: _,
                type_: _,
            } => {
                self.scopes.push(Scope {
                    variables: HashMap::new(),
                });

                for stmt in stmts {
                    self.interpret_stmt(stmt)?;
                }

                if let Some(expr) = end_expr {
                    let val = self.interpret_expr(expr)?;
                    return Ok(val);
                } else {
                    self.scopes.pop();
                    return Ok(0);
                }
            }
            ExprT::Call {
                callee,
                args,
                type_: _,
            } => {
                if *callee == PRINT_INDEX {
                    for arg in args {
                        let value = self.interpret_expr(arg)?;
                        match arg.inner.get_type() {
                            INT_INDEX => println!("{}", value as i64),
                            FLOAT_INDEX => println!("{}", f64::from_bits(value)),
                            STR_INDEX => {
                                let ptr: VarPointer = value.into();
                                let string = self.memory.get_var_slice(ptr)?;
                                let string = unsafe { std::str::from_utf8_unchecked(string) };
                                println!("{}", string);
                            }
                            UNIT_INDEX => println!("()"),
                            id => panic!("invalid type id: {}", id),
                        }
                    }
                    return Ok(0);
                } else {
                    let functions = self.functions.clone();
                    let func = functions
                        .get(&callee)
                        .expect("Internal error: function is not defined");
                    self.scopes.push(Scope {
                        variables: HashMap::new(),
                    });

                    for (i, param) in func.params.iter().enumerate() {
                        let name = param.inner.0;
                        let arg_val = self.interpret_expr(&args[i])?;
                        let current_scope = self.scopes.last_mut().unwrap();
                        current_scope.variables.insert(name, arg_val);
                    }

                    let val = self.interpret_expr(&func.body)?;
                    self.scopes.pop();
                    return Ok(val);
                }
            }
            ExprT::Tuple(entries, _) => {
                let mut values = Vec::new();

                for value in entries {
                    values.push(self.interpret_expr(value)?);
                }

                let ptr = self
                    .memory
                    .add_heap_var(values.len() as u32 * 8, expr.location);
                for (idx, value) in values.iter().enumerate() {
                    self.memory
                        .set(ptr.with_offset(idx as u32 * 8), value, expr.location)?;
                }

                return Ok(ptr.into());
            }
            ExprT::TupleField(tuple, pos, _) => {
                let pos = (*pos) as u32;
                let ptr: VarPointer = self.interpret_expr(tuple)?.into();
                return Ok(self.memory.get_var(ptr.with_offset(pos))?);
            }
            ExprT::Var { name, type_: _ } => Ok(self
                .lookup_in_scope(name)
                .expect("Internal error: variable is not defined")),
            ExprT::UnaryOp { op, rhs, type_: _ } => {
                let r = self.interpret_expr(rhs)?;
                let r_i = r as i64;
                match op {
                    UnaryOp::Minus => return Ok((-r_i) as u64),
                    UnaryOp::Not => Ok(if r == 0 { 1 } else { 0 }),
                }
            }
        }
    }

    fn interpret_value(&mut self, value: &Value, location: LocationRange) -> Result<u64, IError> {
        match value {
            Value::Integer(i) => return Ok(*i as u64),
            Value::Empty => return Ok(0),
            Value::Float(f) => return Ok(f.to_bits()),
            Value::Bool(val) => {
                if *val {
                    return Ok(1);
                } else {
                    return Ok(0);
                }
            }
            Value::Tuple(tup_values) => {
                let mut values = Vec::new();

                for value in tup_values {
                    values.push(self.interpret_value(value, location)?);
                }

                let ptr = self.memory.add_heap_var(values.len() as u32 * 8, location);
                for (idx, value) in values.iter().enumerate() {
                    self.memory
                        .set(ptr.with_offset(idx as u32 * 8), value, location)?;
                }

                return Ok(ptr.into());
            }
            Value::String(string) => {
                let str_value = string.as_bytes();
                let str_len = str_value.len() as u32; // TODO check for overflow

                let ptr = self.memory.add_heap_var(str_len + 1, location);
                self.memory.write_bytes(ptr, str_value, location)?;
                let mut end_ptr = ptr;
                end_ptr.set_offset(str_len);
                self.memory.write_bytes(end_ptr, &vec![0], location)?;
                return Ok(ptr.into());
            }
        }
    }
}
