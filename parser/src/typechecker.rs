use crate::ast::{
    Expr, ExprT, Function, Loc, Name, Op, Program, ProgramT, Stmt, StmtT, Type, TypeDef, TypeId,
    TypeSig, UnaryOp, Value,
};
use crate::lexer::LocationRange;
use crate::printer::type_to_string;
use crate::symbol_table::SymbolTable;
use crate::utils::{
    NameTable, TypeTable, ANY_INDEX, BOOL_INDEX, CHAR_INDEX, FLOAT_INDEX, INT_INDEX, PRINT_INDEX,
    STR_INDEX, UNIT_INDEX,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Fail, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeError {
    #[fail(display = "Variable not defined: '{}'", name)]
    VarNotDefined {
        location: LocationRange,
        name: String,
    },
    #[fail(
        display = "{}: Could not find operation {} with arguments of type {} and {}",
        location, op, lhs_type, rhs_type
    )]
    OpFailure {
        location: LocationRange,
        op: Op,
        lhs_type: Type,
        rhs_type: Type,
    },
    #[fail(display = "Could not unify {} with {}", type1, type2)]
    UnificationFailure {
        location: LocationRange,
        type1: String,
        type2: String,
    },
    #[fail(display = "{}: Type {} does not exist", location, type_name)]
    TypeDoesNotExist {
        location: LocationRange,
        type_name: String,
    },
    #[fail(display = "Field {} does not exist in record", name)]
    FieldDoesNotExist {
        location: LocationRange,
        name: String,
    },
    #[fail(display = "Type {} is not a record", type_)]
    NotARecord {
        location: LocationRange,
        type_: String,
    },
    #[fail(display = "{} Cannot apply unary operator to {:?}", location, expr)]
    InvalidUnaryExpr {
        location: LocationRange,
        expr: ExprT,
    },
    #[fail(display = "Function '{}' is not defined", name)]
    FunctionNotDefined {
        location: LocationRange,
        name: String,
    },
    #[fail(display = "{}: Cannot return at top level", location)]
    TopLevelReturn { location: LocationRange },
    #[fail(
        display = "{}: Function appears to be shadowed by var of same name",
        location
    )]
    ShadowingFunction { location: LocationRange },
    #[fail(display = "{}: Functions are not values", location)]
    FuncValues { location: LocationRange },
}

impl TypeError {
    pub fn get_location(&self) -> LocationRange {
        match self {
            TypeError::VarNotDefined { location, name: _ } => *location,
            TypeError::OpFailure {
                location,
                op: _,
                lhs_type: _,
                rhs_type: _,
            } => *location,
            TypeError::UnificationFailure {
                location,
                type1: _,
                type2: _,
            } => *location,
            TypeError::TypeDoesNotExist {
                location,
                type_name: _,
            } => *location,
            TypeError::FieldDoesNotExist { location, name: _ } => *location,
            TypeError::NotARecord { location, type_: _ } => *location,
            TypeError::FunctionNotDefined { location, name: _ } => *location,
            TypeError::InvalidUnaryExpr { location, expr: _ } => *location,
            TypeError::TopLevelReturn { location } => *location,
            TypeError::ShadowingFunction { location } => *location,
            TypeError::FuncValues { location } => *location,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct FunctionInfo {
    params_type: Vec<TypeId>,
    return_type: TypeId,
}

pub struct TypeChecker {
    symbol_table: SymbolTable,
    // Type names. Right now just has the primitives like string,
    // integer, float, char
    type_names: HashMap<Name, TypeId>,
    // The return type for the typing context
    return_type: Option<TypeId>,
    // Type table
    type_table: TypeTable,
    // Symbol table
    name_table: NameTable,
    function_types: HashMap<Name, FunctionInfo>,
    functions: HashMap<Name, Function>,
}

fn build_type_names(name_table: &mut NameTable) -> HashMap<Name, TypeId> {
    let primitive_types = vec![
        ("int", INT_INDEX),
        ("float", FLOAT_INDEX),
        ("char", CHAR_INDEX),
        ("string", STR_INDEX),
        ("bool", BOOL_INDEX),
    ];
    let mut type_names = HashMap::new();
    for (name, type_id) in primitive_types {
        let name_id = name_table.insert(name.to_string());
        type_names.insert(name_id, type_id);
    }
    type_names
}

impl TypeChecker {
    pub fn new(mut name_table: NameTable) -> TypeChecker {
        let symbol_table = SymbolTable::new();
        let type_table = TypeTable::new();
        let mut function_types = HashMap::new();
        function_types.insert(
            PRINT_INDEX,
            FunctionInfo {
                params_type: vec![ANY_INDEX],
                return_type: UNIT_INDEX,
            },
        );
        TypeChecker {
            symbol_table,
            type_names: build_type_names(&mut name_table),
            return_type: None,
            type_table,
            name_table,
            function_types,
            functions: HashMap::new(),
        }
    }

    pub fn get_tables(self) -> (SymbolTable, NameTable, TypeTable) {
        (self.symbol_table, self.name_table, self.type_table)
    }

    #[allow(dead_code)]
    pub fn get_name_table(&self) -> &NameTable {
        &self.name_table
    }

    pub fn get_functions(self) -> HashMap<Name, Function> {
        self.functions
    }

    pub fn check_program(&mut self, program: Program) -> ProgramT {
        let mut named_types = Vec::new();
        let mut errors = Vec::new();
        for type_def in program.type_defs {
            match self.type_def(type_def) {
                Ok(named_type) => {
                    named_types.push(named_type);
                }
                Err(err) => {
                    errors.push(err);
                }
            }
        }
        if let Err(err) = self.read_functions(&program.stmts) {
            errors.push(err);
        }
        let mut typed_stmts = Vec::new();
        for stmt in program.stmts {
            match self.stmt(stmt) {
                Ok(stmt_t) => {
                    typed_stmts.push(stmt_t);
                }
                Err(err) => {
                    errors.push(err);
                }
            }
        }
        ProgramT {
            stmts: typed_stmts,
            named_types,
            errors,
        }
    }

    fn func_params(
        &mut self,
        params: &Vec<Loc<(Name, Loc<TypeSig>)>>,
    ) -> Result<Vec<Loc<(Name, TypeId)>>, TypeError> {
        let mut typed_params = Vec::new();
        for param in params {
            let (name, type_sig) = &param.inner;
            let param_type = self.lookup_type_sig(type_sig)?;
            typed_params.push(Loc {
                location: param.location,
                inner: (*name, param_type),
            });
        }
        Ok(typed_params)
    }

    // Reads functions defined in this block
    fn read_functions(&mut self, stmts: &Vec<Loc<Stmt>>) -> Result<(), TypeError> {
        for stmt in stmts {
            if let Stmt::Function {
                name,
                params,
                return_type,
                body: _,
            } = &stmt.inner
            {
                let params_type = self.func_params(params)?;
                let return_type = self.lookup_type_sig(return_type)?;
                self.function_types.insert(
                    *name,
                    FunctionInfo {
                        params_type: params_type.iter().map(|e| e.inner.1).collect(),
                        return_type,
                    },
                );
            }
        }
        Ok(())
    }

    fn type_def(&mut self, type_def: Loc<TypeDef>) -> Result<(Name, TypeId), TypeError> {
        match type_def.inner {
            TypeDef::Struct(name, fields) => {
                let mut typed_fields = Vec::new();
                for (name, type_sig) in fields {
                    let field_type = self.lookup_type_sig(&type_sig)?;
                    typed_fields.push((name, field_type));
                }
                let type_id = self.type_table.insert(Type::Record(typed_fields));
                self.type_names.insert(name, type_id);
                Ok((name, type_id))
            }
        }
    }

    pub fn stmt(&mut self, stmt: Loc<Stmt>) -> Result<Loc<StmtT>, TypeError> {
        let location = stmt.location;
        match stmt.inner {
            Stmt::Expr(expr) => {
                let typed_expr = self.expr(expr)?;
                Ok(Loc {
                    location,
                    inner: StmtT::Expr(typed_expr),
                })
            }
            Stmt::Function {
                name,
                params,
                return_type,
                body,
            } => {
                let params = self.func_params(&params)?;
                let return_type = self.lookup_type_sig(&return_type)?;
                self.function(name, params, *body, return_type, location)
            }
            Stmt::Def(name, type_sig, rhs) => Ok(self.def(name, type_sig, rhs, location)?),
            Stmt::Asgn(name, rhs) => Ok(self.asgn(name, rhs, location)?),
            Stmt::Return(expr) => {
                let typed_expr = self.expr(expr)?;
                match self.return_type {
                    Some(return_type) => {
                        if self.is_unifiable(typed_expr.inner.get_type(), return_type) {
                            Ok(Loc {
                                location,
                                inner: StmtT::Return(typed_expr),
                            })
                        } else {
                            let type1 = type_to_string(
                                &self.name_table,
                                &self.type_table,
                                typed_expr.inner.get_type(),
                            );
                            let type2 =
                                type_to_string(&self.name_table, &self.type_table, return_type);
                            Err(TypeError::UnificationFailure {
                                location,
                                type1,
                                type2,
                            })
                        }
                    }
                    None => Err(TypeError::TopLevelReturn {
                        location: stmt.location,
                    }),
                }
            }
        }
    }

    fn value(&self, value: Value) -> Option<ExprT> {
        match value {
            Value::Integer(_i) => Some(ExprT::Primary {
                value,
                type_: INT_INDEX,
            }),
            Value::Float(_f) => Some(ExprT::Primary {
                value,
                type_: FLOAT_INDEX,
            }),
            Value::Bool(_b) => Some(ExprT::Primary {
                value,
                type_: BOOL_INDEX,
            }),
            Value::String(s) => Some(ExprT::Primary {
                value: Value::String(s),
                type_: STR_INDEX,
            }),
            Value::Empty => Some(ExprT::Primary {
                value: Value::Empty,
                type_: UNIT_INDEX,
            }),
            _ => None,
        }
    }

    fn lookup_type_sig(&mut self, sig: &Loc<TypeSig>) -> Result<TypeId, TypeError> {
        match &sig.inner {
            TypeSig::Array(sig) => {
                let type_ = self.lookup_type_sig(sig)?;
                Ok(self.type_table.insert(Type::Array(type_)))
            }
            TypeSig::Tuple(entries) => {
                let mut entry_types = Vec::new();
                for entry in entries {
                    entry_types.push(self.lookup_type_sig(entry)?);
                }
                Ok(self.type_table.insert(Type::Tuple(entry_types)))
            }
            TypeSig::Name(name) => self
                .type_names
                .get(name)
                .ok_or(TypeError::TypeDoesNotExist {
                    location: sig.location,
                    type_name: self.name_table.get_str(name).to_string(),
                })
                .map(|t| *t),
            TypeSig::Empty => Ok(UNIT_INDEX),
        }
    }

    fn def(
        &mut self,
        name: Name,
        type_sig: Loc<TypeSig>,
        rhs: Loc<Expr>,
        location: LocationRange,
    ) -> Result<Loc<StmtT>, TypeError> {
        if self.function_types.contains_key(&name) {
            return Err(TypeError::ShadowingFunction { location });
        }
        let typed_rhs = self.expr(rhs)?;
        let type_sig_type = self.lookup_type_sig(&type_sig)?;
        if let Some(type_) = self.unify(type_sig_type, typed_rhs.inner.get_type()) {
            self.symbol_table.insert_var(name, type_);
            Ok(Loc {
                location,
                inner: StmtT::Def(name, typed_rhs),
            })
        } else {
            let type1 = type_to_string(&self.name_table, &self.type_table, type_sig_type);
            let type2 = type_to_string(
                &self.name_table,
                &self.type_table,
                typed_rhs.inner.get_type(),
            );
            Err(TypeError::UnificationFailure {
                location,
                type1,
                type2,
            })
        }
    }

    fn asgn(
        &mut self,
        name: Name,
        rhs: Loc<Expr>,
        location: LocationRange,
    ) -> Result<Loc<StmtT>, TypeError> {
        let var_type = self
            .symbol_table
            .lookup_name(name)
            .ok_or(TypeError::VarNotDefined {
                location,
                name: self.name_table.get_str(&name).to_string(),
            })?
            .var_type;
        let rhs_t = self.expr(rhs)?;
        if self.unify(var_type, rhs_t.inner.get_type()).is_some() {
            Ok(Loc {
                location,
                inner: StmtT::Asgn(name, rhs_t),
            })
        } else {
            Err(TypeError::UnificationFailure {
                location,
                type1: type_to_string(&self.name_table, &self.type_table, var_type),
                type2: type_to_string(&self.name_table, &self.type_table, rhs_t.inner.get_type()),
            })
        }
    }

    fn function(
        &mut self,
        name: Name,
        params: Vec<Loc<(Name, TypeId)>>,
        body: Loc<Expr>,
        return_type: TypeId,
        location: LocationRange,
    ) -> Result<Loc<StmtT>, TypeError> {
        let previous_scope = self.symbol_table.push_scope(true);
        let old_var_types = self.symbol_table.reset_vars();
        for param in &params {
            let (name, type_) = &param.inner;
            self.symbol_table.insert_var(*name, *type_);
        }
        // Save the current return type
        let mut old_return_type = self.return_type;

        self.return_type = Some(return_type);

        let body_location = body.location;
        // Check body
        let body = self.expr(body)?;
        let body_type = body.inner.get_type();
        std::mem::swap(&mut old_return_type, &mut self.return_type);
        // If the body type is unit, we don't try to unify the body type
        // with return type.
        if body_type != UNIT_INDEX {
            self.unify(old_return_type.unwrap(), body_type)
                .ok_or_else(|| {
                    let type1 = type_to_string(
                        &self.name_table,
                        &self.type_table,
                        old_return_type.unwrap(),
                    );
                    let type2 = type_to_string(&self.name_table, &self.type_table, body_type);
                    TypeError::UnificationFailure {
                        location: body_location,
                        type1,
                        type2,
                    }
                })?
        } else {
            old_return_type.unwrap()
        };

        let local_variables = self.symbol_table.restore_vars(old_var_types);
        let scope_index = self.symbol_table.restore_scope(previous_scope);
        self.functions.insert(
            name,
            Function {
                params,
                body: Box::new(body),
                local_variables,
                scope_index,
            },
        );
        Ok(Loc {
            location,
            inner: StmtT::Function(name),
        })
    }

    fn expr(&mut self, expr: Loc<Expr>) -> Result<Loc<ExprT>, TypeError> {
        let location = expr.location;
        match expr.inner {
            Expr::Primary { value } => Ok(Loc {
                location,
                inner: self.value(value).unwrap(),
            }),
            Expr::Var { name } => {
                let entry =
                    self.symbol_table
                        .lookup_name(name)
                        .ok_or(TypeError::VarNotDefined {
                            location,
                            name: self.name_table.get_str(&name).to_string(),
                        })?;
                Ok(Loc {
                    location,
                    inner: ExprT::Var {
                        name,
                        type_: entry.var_type,
                    },
                })
            }
            Expr::BinOp { op, lhs, rhs } => {
                let typed_lhs = self.expr(*lhs)?;
                let typed_rhs = self.expr(*rhs)?;
                let lhs_type = typed_lhs.inner.get_type();
                let rhs_type = typed_rhs.inner.get_type();
                match self.op(&op, lhs_type, rhs_type) {
                    Some(op_type) => Ok(Loc {
                        location,
                        inner: ExprT::BinOp {
                            op,
                            lhs: Box::new(typed_lhs),
                            rhs: Box::new(typed_rhs),
                            type_: op_type,
                        },
                    }),
                    None => {
                        let lhs_type = self.type_table.get_type(typed_lhs.inner.get_type()).clone();
                        let rhs_type = self.type_table.get_type(typed_rhs.inner.get_type()).clone();
                        Err(TypeError::OpFailure {
                            location,
                            op: op.clone(),
                            lhs_type,
                            rhs_type,
                        })
                    }
                }
            }
            Expr::Tuple(elems) => {
                let mut typed_elems = Vec::new();
                let mut types = Vec::new();
                for elem in elems {
                    let typed_elem = self.expr(elem)?;
                    types.push(typed_elem.inner.get_type());
                    typed_elems.push(typed_elem);
                }
                Ok(Loc {
                    location,
                    inner: ExprT::Tuple(typed_elems, self.type_table.insert(Type::Tuple(types))),
                })
            }
            Expr::UnaryOp { op, rhs } => {
                let typed_rhs = self.expr(*rhs)?;
                let rhs_type = typed_rhs.inner.get_type();
                let is_valid_types = match op {
                    UnaryOp::Minus => {
                        self.is_unifiable(rhs_type, INT_INDEX)
                            || self.is_unifiable(rhs_type, FLOAT_INDEX)
                    }
                    UnaryOp::Not => self.is_unifiable(rhs_type, BOOL_INDEX),
                };
                if is_valid_types {
                    Ok(Loc {
                        location,
                        inner: ExprT::UnaryOp {
                            op,
                            rhs: Box::new(typed_rhs),
                            type_: rhs_type,
                        },
                    })
                } else {
                    Err(TypeError::InvalidUnaryExpr {
                        location: typed_rhs.location,
                        expr: typed_rhs.inner,
                    })
                }
            }
            Expr::Call { callee, args } => {
                let mut typed_args = Vec::new();
                let mut args_type = Vec::new();
                for arg in args {
                    let arg_t = self.expr(arg)?;
                    args_type.push(arg_t.inner.get_type());
                    typed_args.push(arg_t);
                }
                let (params_type, return_type) = {
                    let entry =
                        self.function_types
                            .get(&callee)
                            .ok_or(TypeError::FunctionNotDefined {
                                location,
                                name: self.name_table.get_str(&callee).to_string(),
                            })?;
                    (entry.params_type.clone(), entry.return_type)
                };

                if self.unify_type_vectors(&params_type, &args_type).is_some() {
                    Ok(Loc {
                        location,
                        inner: ExprT::Call {
                            callee,
                            args: typed_args,
                            type_: return_type,
                        },
                    })
                } else {
                    let type1 = params_type
                        .iter()
                        .map(|t| type_to_string(&self.name_table, &self.type_table, *t))
                        .collect::<Vec<String>>()
                        .join(",");
                    let type2 = args_type
                        .iter()
                        .map(|t| type_to_string(&self.name_table, &self.type_table, *t))
                        .collect::<Vec<String>>()
                        .join(",");
                    Err(TypeError::UnificationFailure {
                        location,
                        type1,
                        type2,
                    })
                }
            }
            Expr::Block(stmts, end_expr) => {
                let mut typed_stmts = Vec::new();
                let previous_scope = self.symbol_table.push_scope(false);
                for stmt in stmts {
                    typed_stmts.push(self.stmt(stmt)?);
                }
                let (type_, typed_end_expr) = if let Some(expr) = end_expr {
                    let typed_expr = self.expr(*expr)?;
                    (typed_expr.inner.get_type(), Some(Box::new(typed_expr)))
                } else {
                    (UNIT_INDEX, None)
                };
                let scope_index = self.symbol_table.restore_scope(previous_scope);
                Ok(Loc {
                    location,
                    inner: ExprT::Block {
                        stmts: typed_stmts,
                        end_expr: typed_end_expr,
                        scope_index,
                        type_,
                    },
                })
            }
            Expr::If(cond, then_block, else_block) => {
                let typed_cond = self.expr(*cond)?;
                let typed_then_block = self.expr(*then_block)?;
                let then_type = typed_then_block.inner.get_type();
                if let Some(else_block) = else_block {
                    let typed_else_block = self.expr(*else_block)?;
                    let else_type = typed_else_block.inner.get_type();
                    if !self.is_unifiable(then_type, else_type) {
                        let type1 = type_to_string(&self.name_table, &self.type_table, then_type);
                        let type2 = type_to_string(&self.name_table, &self.type_table, else_type);
                        return Err(TypeError::UnificationFailure {
                            location,
                            type1,
                            type2,
                        });
                    }
                    Ok(Loc {
                        location,
                        inner: ExprT::If(
                            Box::new(typed_cond),
                            Box::new(typed_then_block),
                            Some(Box::new(typed_else_block)),
                            then_type,
                        ),
                    })
                } else if !self.is_unifiable(UNIT_INDEX, then_type) {
                    let type2 = type_to_string(&self.name_table, &self.type_table, then_type);
                    Err(TypeError::UnificationFailure {
                        location,
                        type1: "()".to_string(),
                        type2,
                    })
                } else {
                    Ok(Loc {
                        location,
                        inner: ExprT::If(
                            Box::new(typed_cond),
                            Box::new(typed_then_block),
                            None,
                            UNIT_INDEX,
                        ),
                    })
                }
            }
            Expr::Record { name, fields } => {
                let type_id = if let Some(id) = self.type_names.get(&name) {
                    *id
                } else {
                    let name_str = self.name_table.get_str(&name);
                    return Err(TypeError::TypeDoesNotExist {
                        location,
                        type_name: name_str.to_string(),
                    });
                };

                let mut field_types = Vec::new();
                let mut fields_t = Vec::new();
                for (name, expr) in fields {
                    let expr_t = self.expr(expr)?;
                    field_types.push((name, expr_t.inner.get_type()));
                    fields_t.push(expr_t);
                }
                let expr_type = self.type_table.insert(Type::Record(field_types));
                let type_ = self.unify(type_id, expr_type).ok_or_else(|| {
                    TypeError::UnificationFailure {
                        type1: type_to_string(&self.name_table, &self.type_table, expr_type),
                        type2: type_to_string(&self.name_table, &self.type_table, type_id),
                        location,
                    }
                })?;
                Ok(Loc {
                    location,
                    inner: ExprT::Tuple(fields_t, type_),
                })
            }
            Expr::Field(lhs, name) => {
                let lhs_t = self.expr(*lhs)?;
                let type_id = lhs_t.inner.get_type();
                match self.type_table.get_type(type_id) {
                    Type::Record(fields) => {
                        let field = fields.iter().find(|(field_name, _)| *field_name == name);

                        if let Some(field) = field {
                            Ok(Loc {
                                location,
                                inner: ExprT::Field(Box::new(lhs_t), name, field.1),
                            })
                        } else {
                            let name_str = self.name_table.get_str(&name);
                            Err(TypeError::FieldDoesNotExist {
                                location,
                                name: name_str.to_string(),
                            })
                        }
                    }
                    _ => Err(TypeError::NotARecord {
                        location,
                        type_: type_to_string(&self.name_table, &self.type_table, type_id),
                    }),
                }
            }
        }
    }

    fn op(&mut self, op: &Op, lhs_type: TypeId, rhs_type: TypeId) -> Option<TypeId> {
        match op {
            Op::Plus | Op::Minus | Op::Times | Op::Div => {
                if lhs_type == INT_INDEX && rhs_type == INT_INDEX {
                    Some(INT_INDEX)
                } else if lhs_type == FLOAT_INDEX && rhs_type == INT_INDEX {
                    Some(FLOAT_INDEX)
                } else if lhs_type == INT_INDEX && rhs_type == FLOAT_INDEX {
                    Some(FLOAT_INDEX)
                } else if lhs_type == FLOAT_INDEX && rhs_type == FLOAT_INDEX {
                    Some(FLOAT_INDEX)
                } else {
                    None
                }
            }
            Op::BangEqual | Op::EqualEqual => {
                if self.is_unifiable(lhs_type, rhs_type) {
                    Some(BOOL_INDEX)
                } else {
                    None
                }
            }
            Op::GreaterEqual | Op::Greater | Op::Less | Op::LessEqual => {
                // If we can unify lhs and rhs, and lhs with Int or Float then
                // by transitivity we can unify everything with float
                let is_num = self.is_unifiable(lhs_type, FLOAT_INDEX)
                    || self.is_unifiable(lhs_type, INT_INDEX);
                if self.is_unifiable(lhs_type, rhs_type) && is_num {
                    Some(BOOL_INDEX)
                } else {
                    None
                }
            }
        }
    }

    fn unify_type_vectors(
        &mut self,
        type_vector1: &[TypeId],
        type_vector2: &[TypeId],
    ) -> Option<Vec<TypeId>> {
        if type_vector1.len() != type_vector2.len() {
            return None;
        }
        let mut types = Vec::new();
        for (t1, t2) in type_vector1.iter().zip(type_vector2.iter()) {
            if let Some(t) = self.unify(*t1, *t2) {
                types.push(t)
            } else {
                return None;
            }
        }
        Some(types)
    }

    fn unify<'a>(&mut self, type_id1: TypeId, type_id2: TypeId) -> Option<TypeId> {
        if type_id1 == type_id2 {
            return Some(type_id1);
        }
        let type1 = self.type_table.get_type(type_id1).clone();
        let type2 = self.type_table.get_type(type_id2).clone();
        match (type1, type2) {
            (Type::Record(fields), Type::Record(other_fields)) => {
                if fields.len() != other_fields.len() {
                    return None;
                }
                let mut unified_fields = Vec::new();
                for ((n1, t1), (n2, t2)) in fields.iter().zip(other_fields.iter()) {
                    if *n1 != *n2 {
                        return None;
                    }
                    if let Some(t) = self.unify(*t1, *t2) {
                        unified_fields.push((*n1, t));
                    } else {
                        return None;
                    }
                }
                let id = self.type_table.insert(Type::Record(unified_fields));
                self.type_table.update(type_id1, Type::Solved(id));
                self.type_table.update(type_id2, Type::Solved(id));
                Some(id)
            }
            (Type::Tuple(ts), Type::Unit) | (Type::Unit, Type::Tuple(ts)) => {
                if ts.is_empty() {
                    Some(type_id1)
                } else {
                    None
                }
            }
            (Type::Tuple(t1), Type::Tuple(t2)) => {
                if let Some(types) = self.unify_type_vectors(&t1, &t2) {
                    let id = self.type_table.insert(Type::Tuple(types));
                    Some(id)
                } else {
                    None
                }
            }
            (Type::Arrow(param_type1, return_type1), Type::Arrow(param_type2, return_type2)) => {
                match (
                    self.unify_type_vectors(&param_type1, &param_type2),
                    self.unify(return_type1, return_type2),
                ) {
                    (Some(param_type), Some(return_type)) => {
                        let id = self.type_table.insert(Type::Arrow(param_type, return_type));
                        Some(id)
                    }
                    _ => None,
                }
            }
            (Type::Int, Type::Bool) => Some(type_id1),
            (Type::Bool, Type::Int) => Some(type_id2),
            (Type::Any, _) => Some(type_id2),
            (_, Type::Any) => Some(type_id1),
            _ => None,
        }
    }

    fn is_unifiable(&mut self, type1: TypeId, type2: TypeId) -> bool {
        self.unify(type1, type2).is_some()
    }
}
