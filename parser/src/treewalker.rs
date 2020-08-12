use crate::ast::{ExprT, Function, Loc, Name, Op, ProgramT, StmtT, UnaryOp, Value};
use crate::lexer::LocationRange;
use crate::utils::PRINT_INDEX;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

struct Scope {
    parent: Option<usize>,
    variables: HashMap<Name, Value>,
}

pub struct TreeWalker {
    scopes: Vec<Scope>,
    functions: Arc<HashMap<Name, Function>>,
    current_scope: usize,
}

#[derive(Debug, Fail, PartialEq, Clone, Serialize, Deserialize)]
pub enum WalkerError {
    #[fail(display = "Not implemented: {}", reason)]
    NotImplemented {
        location: LocationRange,
        reason: &'static str,
    },
    #[fail(display = "Not reachable. Internal error")]
    NotReachable { location: LocationRange },
}

impl TreeWalker {
    pub fn new(functions: HashMap<Name, Function>) -> Self {
        TreeWalker {
            scopes: vec![Scope {
                parent: None,
                variables: HashMap::new(),
            }],
            current_scope: 0,
            functions: Arc::new(functions),
        }
    }

    pub fn interpret_program(&mut self, program: ProgramT) -> Result<(), WalkerError> {
        for stmt in program.stmts {
            self.interpret_stmt(&stmt)?;
        }
        Ok(())
    }

    fn lookup_in_scope(&self, name: &Name) -> Option<&Value> {
        let mut scope_index = Some(self.current_scope);
        while let Some(s) = scope_index {
            let entry = self.scopes[s].variables.get(name);
            if entry.is_some() {
                return entry;
            }
            scope_index = self.scopes[s].parent;
        }
        None
    }

    fn interpret_stmt(&mut self, stmt: &Loc<StmtT>) -> Result<(), WalkerError> {
        match &stmt.inner {
            StmtT::Def(name, rhs) => {
                let rhs_val = self.interpret_expr(rhs)?;
                self.scopes[self.current_scope]
                    .variables
                    .insert(*name, rhs_val);
            }
            StmtT::Expr(expr) => {
                let res = self.interpret_expr(expr)?;
                println!("{:?}", res);
            }
            StmtT::Function(_) => {}
            _ => {
                return Err(WalkerError::NotImplemented {
                    location: stmt.location,
                    reason: "Statements",
                })
            }
        }
        Ok(())
    }

    fn interpret_expr(&mut self, expr: &Loc<ExprT>) -> Result<Value, WalkerError> {
        match &expr.inner {
            ExprT::Primary { value, type_: _ } => Ok(value.clone()),
            ExprT::BinOp {
                op,
                lhs,
                rhs,
                type_: _,
            } => {
                let lhs_val = self.interpret_expr(lhs)?;
                let rhs_val = self.interpret_expr(rhs)?;
                match (op, lhs_val, rhs_val) {
                    (Op::Plus, Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l + r)),
                    (Op::Times, Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l * r)),
                    (Op::Minus, Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l - r)),
                    (Op::Div, Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l / r)),
                    (Op::BangEqual, Value::Integer(l), Value::Integer(r)) => {
                        Ok(Value::Bool(l != r))
                    }
                    (Op::EqualEqual, Value::Integer(l), Value::Integer(r)) => {
                        Ok(Value::Bool(l == r))
                    }
                    (Op::Greater, Value::Integer(l), Value::Integer(r)) => Ok(Value::Bool(l > r)),
                    (Op::GreaterEqual, Value::Integer(l), Value::Integer(r)) => {
                        Ok(Value::Bool(l >= r))
                    }
                    (Op::Less, Value::Integer(l), Value::Integer(r)) => Ok(Value::Bool(l < r)),
                    (Op::LessEqual, Value::Integer(l), Value::Integer(r)) => {
                        Ok(Value::Bool(l <= r))
                    }
                    (Op::Plus, Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
                    (Op::Times, Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
                    (Op::Minus, Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
                    (Op::Div, Value::Float(l), Value::Float(r)) => Ok(Value::Float(l / r)),
                    (Op::BangEqual, Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l != r)),
                    (Op::EqualEqual, Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l == r)),
                    (Op::Greater, Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l > r)),
                    (Op::GreaterEqual, Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l >= r)),
                    (Op::Less, Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l < r)),
                    (Op::LessEqual, Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l <= r)),
                    (Op::Plus, Value::String(l), Value::String(r)) => {
                        Ok(Value::String(format!("{}{}", l, r)))
                    }
                    (Op::BangEqual, Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l != r)),
                    (Op::EqualEqual, Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l == r)),
                    _ => Err(WalkerError::NotReachable {
                        location: expr.location,
                    }),
                }
            }
            ExprT::If(cond, then_clause, else_clause, _) => {
                let cond_val = self.interpret_expr(cond)?;
                if cond_val == Value::Bool(true) {
                    self.interpret_expr(then_clause)
                } else if let Some(else_clause) = else_clause {
                    self.interpret_expr(else_clause)
                } else {
                    Ok(Value::Empty)
                }
            }
            ExprT::Block {
                stmts,
                end_expr,
                scope_index: _,
                type_: _,
            } => {
                self.scopes.push(Scope {
                    parent: Some(self.current_scope),
                    variables: HashMap::new(),
                });
                let old_scope = self.current_scope;
                self.current_scope = self.scopes.len() - 1;
                for stmt in stmts {
                    self.interpret_stmt(stmt)?;
                }
                let val = end_expr
                    .as_ref()
                    .map(|expr| self.interpret_expr(expr))
                    .unwrap_or(Ok(Value::Empty));
                self.current_scope = old_scope;
                val
            }
            ExprT::Call {
                callee,
                args,
                type_: _,
            } => {
                if *callee == PRINT_INDEX {
                    for arg in args {
                        let value = self.interpret_expr(arg)?;
                        println!("{}", value);
                    }
                    Ok(Value::Empty)
                } else {
                    let functions = self.functions.clone();
                    let func = functions
                        .get(&callee)
                        .expect("Internal error: function is not defined");
                    self.scopes.push(Scope {
                        parent: Some(self.current_scope),
                        variables: HashMap::new(),
                    });
                    let old_scope = self.current_scope;
                    self.current_scope = self.scopes.len() - 1;
                    for (i, param) in func.params.iter().enumerate() {
                        let name = param.inner.0;
                        let arg_val = self.interpret_expr(&args[i])?;
                        self.scopes[self.current_scope]
                            .variables
                            .insert(name, arg_val);
                    }
                    let val = self.interpret_expr(&func.body);
                    self.current_scope = old_scope;
                    val
                }
            }
            ExprT::Tuple(entries, _) => {
                let mut values = Vec::new();
                for entry in entries {
                    values.push(self.interpret_expr(entry)?);
                }
                Ok(Value::Tuple(values))
            }
            ExprT::Var { name, type_: _ } => Ok(self
                .lookup_in_scope(name)
                .expect("Internal error: variable is not defined")
                .clone()),
            ExprT::UnaryOp { op, rhs, type_: _ } => {
                let rhs_val = self.interpret_expr(rhs)?;
                match (op, rhs_val) {
                    (UnaryOp::Minus, Value::Integer(r)) => Ok(Value::Integer(-r)),
                    (UnaryOp::Minus, Value::Float(r)) => Ok(Value::Float(-r)),
                    (UnaryOp::Not, Value::Bool(r)) => Ok(Value::Bool(!r)),
                    (_, _) => Err(WalkerError::NotReachable {
                        location: expr.location,
                    }),
                }
            }
            _ => Err(WalkerError::NotImplemented {
                location: expr.location,
                reason: "Expressions",
            }),
        }
    }
}
