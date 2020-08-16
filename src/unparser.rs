use crate::ast::{Expr, Loc, Program, Stmt, TypeSig, Value};
use crate::utils::{NameTable, PRINT_INDEX};
use serde::{Deserialize, Serialize};

pub struct Unparser {
    name_table: NameTable,
    indent_level: usize,
}

#[derive(Debug, Fail, PartialEq, Clone, Serialize, Deserialize)]
pub enum UnparseError {
    #[fail(display = "Not implemented: {}", node)]
    NotImplemented { node: String },
}

pub struct UnparsedProgram {
    pub functions: String,
    pub global_stmts: String
}

impl Unparser {
    pub fn new(name_table: NameTable) -> Self {
        Unparser {
            name_table,
            indent_level: 0,
        }
    }

    fn get_free_name(&self) -> String {
        let mut i = 0;
        loop {
            let name = format!("main{}", i);
            if !self.name_table.contains_str(&name) {
                return name;
            }
            i += 1;
        }
    }

    pub fn unparse_program(&self, program: &Program) -> Result<UnparsedProgram, UnparseError> {
        let mut functions = Vec::new();
        let mut global_stmts = Vec::new();
        for stmt in &program.stmts {
            if let Stmt::Function { name, params, return_type, body } = &stmt.inner {
                functions.push(stmt);
            } else {
                global_stmts.push(stmt);
            }
        }
        let mut unparsed_functions = Vec::new();
        for func in functions {
            unparsed_functions.push(self.unparse_stmt(func)?);
        }
        let mut unparsed_global_stmts = Vec::new();
        for stmt in global_stmts {
            unparsed_global_stmts.push(self.unparse_stmt(stmt)?);
        }
        let main_function = format!("fn {}() {{ {} }}", self.get_free_name(), unparsed_global_stmts.join("\n"));
        Ok(UnparsedProgram { functions: unparsed_functions.join("\n"), global_stmts: main_function })
    }

    fn unparse_stmt(&self, stmt: &Loc<Stmt>) -> Result<String, UnparseError> {
        let indents = "  ".repeat(self.indent_level);
        match &stmt.inner {
            Stmt::Def(name, type_sig, rhs) => Ok(format!(
                "{}let {}: {} = {};",
                indents,
                self.name_table.get_str(name),
                self.unparse_type_sig(type_sig)?,
                self.unparse_expr(rhs)?
            )),
            Stmt::Expr(expr) => Ok(format!("{}{};", indents, self.unparse_expr(expr)?)),
            Stmt::Function {
                name,
                params,
                return_type,
                body,
            } => {
                let params: Result<Vec<_>, _> = params
                    .iter()
                    .map(|span| {
                        let (name, type_sig) = &span.inner;
                        Ok(format!(
                            "{}: {}",
                            self.name_table.get_str(name),
                            self.unparse_type_sig(type_sig)?
                        ))
                    })
                    .collect();
                Ok(format!(
                    "{}fn {}({}) {{\n{}}}",
                    indents,
                    self.name_table.get_str(name),
                    params?.join(", "),
                    self.unparse_expr(body)?
                ))
            }
            s => Err(UnparseError::NotImplemented {
                node: format!("{:?}", s),
            }),
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
            Expr::Call { callee, args } => {
                let args_str: Result<Vec<_>, _> =
                    args.iter().map(|a| self.unparse_expr(a)).collect();
                let str = if *callee == PRINT_INDEX {
                    "print!"
                } else {
                    self.name_table.get_str(callee)
                };
                Ok(format!(
                    "{}({})",
                    str,
                    args_str?.join(", ")
                ))
            }
            Expr::Field(lhs, name) => Ok(format!(
                "{}.{}",
                self.unparse_expr(lhs)?,
                self.name_table.get_str(name)
            )),
            Expr::TupleField(lhs, index) => Ok(format!("{}.{}", self.unparse_expr(lhs)?, *index)),
            Expr::Record { name, fields } => {
                let indents = "  ".repeat(self.indent_level + 1);
                let fields_vec: Result<Vec<_>, _> = fields
                    .iter()
                    .map(|(name, expr)| {
                        Ok(format!(
                            "{}{}: {}",
                            indents,
                            self.name_table.get_str(name),
                            self.unparse_expr(expr)?
                        ))
                    })
                    .collect();

                Ok(format!(
                    "{} {{\n{}\n{}}}",
                    self.name_table.get_str(name),
                    "  ".repeat(self.indent_level),
                    fields_vec?.join(",\n")
                ))
            }
            Expr::Tuple(entries) => {
                let entries: Result<Vec<_>, _> =
                    entries.iter().map(|e| self.unparse_expr(e)).collect();
                Ok(format!("({})", entries?.join(", ")))
            }
            Expr::Block(stmts, end_expr) => {
                let mut unparsed_stmts = Vec::new();
                for stmt in stmts {
                    unparsed_stmts.push(self.unparse_stmt(stmt)?);
                }
                if let Some(end_expr) = end_expr {
                    unparsed_stmts.push(format!("{}\n", self.unparse_expr(end_expr)?));
                }
                Ok(unparsed_stmts.join(""))
            }
            Expr::Var { name } => {
                Ok(self.name_table.get_str(name).to_string())
            },
            Expr::If(cond, then_block, else_block) => {
                let else_str = if let Some(else_block) = else_block {
                    format!(" else {{\n{}}}", self.unparse_expr(else_block)?)
                } else {
                    String::new()
                };
                Ok(format!(
                    "if {} {{\n{}}}{}",
                    self.unparse_expr(cond)?,
                    self.unparse_expr(then_block)?,
                    else_str
                ))
            }
            e => Err(UnparseError::NotImplemented {
                node: format!("{:?}", e),
            }),
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
