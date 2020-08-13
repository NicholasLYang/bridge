use crate::ast::{Expr, Loc, Program, Stmt, TypeSig, Value};
use crate::utils::NameTable;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

pub struct Unparser {
    name_table: NameTable,
}

#[derive(Debug, Fail, PartialEq, Clone, Serialize, Deserialize)]
pub enum UnparseError {
    #[fail(display = "Not implemented")]
    NotImplemented,
}

impl Unparser {
    pub fn new(name_table: NameTable) -> Self {
        Unparser { name_table }
    }

    pub fn unparse_program(&self, program: &Program) -> Result<String, UnparseError> {
        let mut stmts = Vec::new();
        for stmt in &program.stmts {
            stmts.push(self.unparse_stmt(stmt)?);
        }
        Ok(stmts.join("\n"))
    }

    fn unparse_stmt(&self, stmt: &Loc<Stmt>) -> Result<String, UnparseError> {
        match &stmt.inner {
            Stmt::Def(name, type_sig, rhs) => Ok(format!(
                "let {}: {} = {}",
                self.name_table.get_str(name),
                self.unparse_type_sig(type_sig)?,
                self.unparse_expr(rhs)?
            )),
            Stmt::Expr(expr) => Ok(format!("{};", self.unparse_expr(expr)?)),
            _ => Err(UnparseError::NotImplemented),
        }
    }

    fn unparse_expr(&self, expr: &Loc<Expr>) -> Result<String, UnparseError> {
        match &expr.inner {
            Expr::Primary { value } => self.unparse_value(value),
            Expr::BinOp { op, lhs, rhs } => Ok(format!(
                "{} {} {}",
                self.unparse_expr(&**lhs)?,
                op,
                self.unparse_expr(&**rhs)?
            )),
            Expr::Var { name } => Ok(self.name_table.get_str(name).to_string()),
            _ => Err(UnparseError::NotImplemented),
        }
    }

    fn unparse_value(&self, value: &Value) -> Result<String, UnparseError> {
        match value {
            Value::Float(v) => Ok(format!("{}", v)),
            Value::Integer(v) => Ok(format!("{}", v)),
            Value::Bool(b) => {
                if *b {
                    Ok("true".to_string())
                } else {
                    Ok("false".to_string())
                }
            }
            Value::String(s) => Ok(format!("\"{}\"", s)),
            Value::Tuple(entries) => {
                let entries: Result<Vec<_>, _> =
                    entries.iter().map(|e| self.unparse_value(e)).collect();
                Ok(format!("({})", entries?.join(", ")))
            }
            Value::Empty => Ok("()".to_string()),
        }
    }

    fn unparse_type_sig(&self, type_sig: &Loc<TypeSig>) -> Result<String, UnparseError> {
        match &type_sig.inner {
            TypeSig::Name(n) => Ok(self.name_table.get_str(n).to_string()),
            TypeSig::Tuple(entries) => {
                let mut type_sigs = Vec::new();
                for entry in entries {
                    type_sigs.push(self.unparse_type_sig(entry)?);
                }
                Ok(type_sigs.join(", "))
            }
            TypeSig::Array(type_sig) => Ok(format!("[{}]", self.unparse_type_sig(type_sig)?)),
            TypeSig::Empty => Ok("()".to_string()),
        }
    }
}
