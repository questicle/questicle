use std::rc::Rc;

use crate::ast::*;
use crate::env::Env;
use crate::host::Host;
use crate::stdlib::install_std;
use crate::value::{EnvRef, Function, Value};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("{0}")]
    Msg(String),
}

pub struct Interpreter {
    pub env: EnvRef,
    pub host: Host,
}

impl Interpreter {
    pub fn with_host(host: Host) -> Self {
        let env = Env::new_global();
        install_std(&env, host.clone());
        Self { env, host }
    }

    pub fn eval(&mut self, program: Program) -> Result<Option<Value>, RuntimeError> {
        let mut last = None;
        for s in program.statements {
            last = self.exec_stmt(&s)?;
        }
        Ok(last)
    }

    fn exec_block(&mut self, body: &[Stmt]) -> Result<Option<Value>, RuntimeError> {
        let child = crate::env::Env::child_of(&self.env);
        let saved = self.env.clone();
        self.env = child.clone();
        let mut result = None;
        for s in body {
            if let Some(v) = self.exec_stmt(s)? {
                result = Some(v);
                break;
            }
        }
        self.env = saved;
        Ok(result)
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Option<Value>, RuntimeError> {
        match stmt {
            Stmt::Let { name, init } => {
                let v = self.eval_expr(init)?;
                self.env.borrow_mut().define(name.clone(), v);
                Ok(None)
            }
            Stmt::Expr(e) => {
                let v = self.eval_expr(e)?;
                Ok(Some(v))
            }
            Stmt::Block(b) => self.exec_block(b),
            Stmt::If {
                cond,
                then_branch,
                else_branch,
            } => {
                if self.eval_expr(cond)?.truthy() {
                    self.exec_stmt(then_branch)
                } else if let Some(e) = else_branch {
                    self.exec_stmt(e)
                } else {
                    Ok(None)
                }
            }
            Stmt::While { cond, body } => {
                while self.eval_expr(cond)?.truthy() {
                    if let Some(v) = self.exec_stmt(body)? {
                        return Ok(Some(v));
                    }
                }
                Ok(None)
            }
            Stmt::For { name, iter, body } => {
                let it = self.eval_expr(iter)?;
                match it {
                    Value::List(list) => {
                        for item in list {
                            let child = crate::env::Env::child_of(&self.env);
                            child.borrow_mut().define(name.clone(), item);
                            let saved = self.env.clone();
                            self.env = child;
                            let r = self.exec_stmt(body)?;
                            self.env = saved;
                            if r.is_some() {
                                return Ok(r);
                            }
                        }
                        Ok(None)
                    }
                    _ => Err(RuntimeError::Msg("for expects list".into())),
                }
            }
            Stmt::Return(v) => {
                let val = match v {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Null,
                };
                Ok(Some(val))
            }
            Stmt::Break | Stmt::Continue => Ok(None), // simplified control flow
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        use Expr::*;
        Ok(match expr {
            Literal(Lit::Number(n)) => Value::Number(*n),
            Literal(Lit::Bool(b)) => Value::Bool(*b),
            Literal(Lit::String(s)) => Value::String(s.clone()),
            Literal(Lit::Null) => Value::Null,
            Var(name) => self
                .env
                .borrow()
                .get(name)
                .ok_or_else(|| RuntimeError::Msg(format!("Undefined variable '{name}'")))?,
            Assign { name, value } => {
                let v = self.eval_expr(value)?;
                self.env
                    .borrow_mut()
                    .assign(name, v.clone())
                    .map_err(RuntimeError::Msg)?;
                v
            }
            Unary { op, expr } => {
                let v = self.eval_expr(expr)?;
                match op {
                    UnOp::Neg => match v {
                        Value::Number(n) => Value::Number(-n),
                        _ => return Err(RuntimeError::Msg("Unary - expects number".into())),
                    },
                    UnOp::Not => Value::Bool(!v.truthy()),
                }
            }
            Binary { left, op, right } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                match op {
                    BinOp::Add => add(l, r)?,
                    BinOp::Sub => num2(l, r, |a, b| Value::Number(a - b))?,
                    BinOp::Mul => num2(l, r, |a, b| Value::Number(a * b))?,
                    BinOp::Div => num2(l, r, |a, b| Value::Number(a / b))?,
                    BinOp::Mod => num2(l, r, |a, b| Value::Number(a % b))?,
                    BinOp::Eq => Value::Bool(eq(&l, &r)),
                    BinOp::Ne => Value::Bool(!eq(&l, &r)),
                    BinOp::Lt => cmp(l, r, |a, b| a < b)?,
                    BinOp::Le => cmp(l, r, |a, b| a <= b)?,
                    BinOp::Gt => cmp(l, r, |a, b| a > b)?,
                    BinOp::Ge => cmp(l, r, |a, b| a >= b)?,
                    BinOp::And => Value::Bool(l.truthy() && r.truthy()),
                    BinOp::Or => Value::Bool(l.truthy() || r.truthy()),
                }
            }
            Call { callee, args } => {
                let c = self.eval_expr(callee)?;
                let mut a = Vec::new();
                for x in args {
                    a.push(self.eval_expr(x)?);
                }
                self.call_function(c, a)?
            }
            Fn { params, body } => Value::Function(Rc::new(Function::User {
                params: params.clone(),
                body: body.clone(),
                env: self.env.clone(),
            })),
            List(items) => {
                let mut v = Vec::new();
                for e in items {
                    v.push(self.eval_expr(e)?);
                }
                Value::List(v)
            }
            Map(props) => {
                let mut m = std::collections::BTreeMap::new();
                for (k, e) in props {
                    m.insert(k.clone(), self.eval_expr(e)?);
                }
                Value::Map(m)
            }
            Index { target, index } => {
                let t = self.eval_expr(target)?;
                let i = self.eval_expr(index)?;
                match (t, i) {
                    (Value::List(v), Value::Number(n)) => {
                        v.get(n as usize).cloned().unwrap_or(Value::Null)
                    }
                    (Value::Map(m), Value::String(s)) => m.get(&s).cloned().unwrap_or(Value::Null),
                    _ => Value::Null,
                }
            }
            Field { target, name } => {
                let t = self.eval_expr(target)?;
                match t {
                    Value::Map(m) => m.get(name).cloned().unwrap_or(Value::Null),
                    _ => Value::Null,
                }
            }
        })
    }

    fn call_function(&mut self, callee: Value, args: Vec<Value>) -> Result<Value, RuntimeError> {
        match callee {
            Value::Function(f) => match f.as_ref() {
                Function::Native { fun, .. } => {
                    fun(args, self.env.clone()).map_err(RuntimeError::Msg)
                }
                Function::User { params, body, env } => {
                    let child = crate::env::Env::child_of(env);
                    for (i, p) in params.iter().enumerate() {
                        child
                            .borrow_mut()
                            .define(p.clone(), args.get(i).cloned().unwrap_or(Value::Null));
                    }
                    let saved = self.env.clone();
                    self.env = child.clone();
                    let mut ret = Value::Null;
                    for s in body {
                        if let Some(v) = self.exec_stmt(s)? {
                            ret = v;
                            break;
                        }
                    }
                    self.env = saved;
                    Ok(ret)
                }
            },
            _ => Err(RuntimeError::Msg("attempt to call non-function".into())),
        }
    }
}

fn add(l: Value, r: Value) -> Result<Value, RuntimeError> {
    match (l, r) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
        (Value::String(a), Value::String(b)) => Ok(Value::String(a + b.as_str())),
        (Value::String(a), b) => Ok(Value::String(format!("{}{}", a, b))),
        (a, Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
        _ => Err(RuntimeError::Msg("type error for +".into())),
    }
}

fn num2<F: Fn(f64, f64) -> Value>(l: Value, r: Value, f: F) -> Result<Value, RuntimeError> {
    if let (Value::Number(a), Value::Number(b)) = (l, r) {
        Ok(f(a, b))
    } else {
        Err(RuntimeError::Msg("number operands required".into()))
    }
}
fn cmp<F: Fn(f64, f64) -> bool>(l: Value, r: Value, f: F) -> Result<Value, RuntimeError> {
    if let (Value::Number(a), Value::Number(b)) = (l, r) {
        Ok(Value::Bool(f(a, b)))
    } else {
        Err(RuntimeError::Msg("number operands required".into()))
    }
}
fn eq(a: &Value, b: &Value) -> bool {
    use Value::*;
    match (a, b) {
        (Number(x), Number(y)) => x == y,
        (Bool(x), Bool(y)) => x == y,
        (String(x), String(y)) => x == y,
        (Null, Null) => true,
        _ => false,
    }
}
