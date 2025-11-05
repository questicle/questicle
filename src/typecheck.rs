use std::collections::BTreeMap;

use crate::ast::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Number,
    String,
    Bool,
    Null,
    List(Box<Type>),
    Map(Box<Type>),
    Func(Vec<Type>, Box<Type>),
    Any,
}

impl Type {
    pub fn from_expr(t: &TypeExpr) -> Type {
        match t {
            TypeExpr::Number => Type::Number,
            TypeExpr::String => Type::String,
            TypeExpr::Bool => Type::Bool,
            TypeExpr::Null => Type::Null,
            TypeExpr::List(i) => Type::List(Box::new(Type::from_expr(i))),
            TypeExpr::Map(i) => Type::Map(Box::new(Type::from_expr(i))),
            TypeExpr::Func(args, ret) => Type::Func(
                args.iter().map(Type::from_expr).collect(),
                Box::new(Type::from_expr(ret)),
            ),
            TypeExpr::Any => Type::Any,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub subject: Option<String>,
}

#[derive(Default)]
pub struct TypeEnv {
    pub vars: BTreeMap<String, Type>,
}

pub struct TypeCheckResult {
    pub errors: Vec<TypeError>,
    pub env: TypeEnv,
}

pub fn check_program(p: &Program) -> TypeCheckResult {
    let mut env = TypeEnv::default();
    // Builtins
    prelude(&mut env);
    let mut errors = Vec::new();
    for s in &p.statements {
        check_stmt(s, &mut env, None, &mut errors);
    }
    TypeCheckResult { errors, env }
}

fn prelude(env: &mut TypeEnv) {
    // print(any) -> null (approx)
    env.vars.insert(
        "print".into(),
        Type::Func(vec![Type::Any], Box::new(Type::Null)),
    );
    env.vars
        .insert("clock".into(), Type::Func(vec![], Box::new(Type::Number)));
    env.vars
        .insert("random".into(), Type::Func(vec![], Box::new(Type::Number)));
    env.vars.insert(
        "len".into(),
        Type::Func(vec![Type::Any], Box::new(Type::Number)),
    );
    env.vars.insert(
        "keys".into(),
        Type::Func(
            vec![Type::Map(Box::new(Type::Any))],
            Box::new(Type::List(Box::new(Type::String))),
        ),
    );
    env.vars.insert(
        "push".into(),
        Type::Func(
            vec![Type::List(Box::new(Type::Any)), Type::Any],
            Box::new(Type::List(Box::new(Type::Any))),
        ),
    );
    env.vars.insert(
        "pop".into(),
        Type::Func(vec![Type::List(Box::new(Type::Any))], Box::new(Type::Any)),
    );
    env.vars.insert(
        "on".into(),
        Type::Func(vec![Type::String, Type::Any], Box::new(Type::Null)),
    );
    env.vars.insert(
        "emit".into(),
        Type::Func(vec![Type::String, Type::Any], Box::new(Type::Null)),
    );
    env.vars.insert(
        "host".into(),
        Type::Func(vec![Type::String, Type::Any], Box::new(Type::Any)),
    );
}

fn check_stmt(
    stmt: &Stmt,
    env: &mut TypeEnv,
    expected_ret: Option<&Type>,
    errors: &mut Vec<TypeError>,
) {
    match stmt {
        Stmt::Let { name, ty, init } => {
            let t_init = infer_expr(init, env, errors);
            if let Some(ann) = ty {
                let ann_t = Type::from_expr(ann);
                if !is_compatible(&t_init, &ann_t) {
                    errors.push(TypeError {
                        message: format!(
                            "Type mismatch: variable '{}' initialized with {} but annotated as {}",
                            name, t_init, ann_t
                        ),
                        subject: Some(name.clone()),
                    });
                }
                env.vars.insert(name.clone(), ann_t);
            } else {
                env.vars.insert(name.clone(), t_init);
            }
        }
        Stmt::Expr(e) => {
            let _ = infer_expr(e, env, errors);
        }
        Stmt::Block(b) => {
            let mut child = TypeEnv {
                vars: env.vars.clone(),
            };
            for s in b {
                check_stmt(s, &mut child, expected_ret, errors);
            }
        }
        Stmt::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let t = infer_expr(cond, env, errors);
            if !is_compatible(&t, &Type::Bool) {
                errors.push(TypeError {
                    message: format!("If condition must be bool, got {}", t),
                    subject: None,
                });
            }
            check_stmt(then_branch, env, expected_ret, errors);
            if let Some(e) = else_branch {
                check_stmt(e, env, expected_ret, errors);
            }
        }
        Stmt::While { cond, body } => {
            let t = infer_expr(cond, env, errors);
            if !is_compatible(&t, &Type::Bool) {
                errors.push(TypeError {
                    message: format!("While condition must be bool, got {}", t),
                    subject: None,
                });
            }
            check_stmt(body, env, expected_ret, errors);
        }
        Stmt::For { name, iter, body } => {
            let it = infer_expr(iter, env, errors);
            match it {
                Type::List(inner) => {
                    let mut child = TypeEnv {
                        vars: env.vars.clone(),
                    };
                    child.vars.insert(name.clone(), *inner.clone());
                    check_stmt(body, &mut child, expected_ret, errors);
                }
                _ => errors.push(TypeError {
                    message: format!("For expects list, got {}", it),
                    subject: Some(name.clone()),
                }),
            }
        }
        Stmt::Return(v) => {
            if let Some(e) = v {
                let t = infer_expr(e, env, errors);
                if let Some(exp) = expected_ret {
                    if !is_compatible(&t, exp) {
                        errors.push(TypeError {
                            message: format!("Return type {} does not match expected {}", t, exp),
                            subject: None,
                        });
                    }
                }
            } else if let Some(exp) = expected_ret {
                if !is_compatible(&Type::Null, exp) {
                    errors.push(TypeError {
                        message: format!("Return type null does not match expected {}", exp),
                        subject: None,
                    });
                }
            }
        }
        Stmt::Break | Stmt::Continue => {}
    }
}

fn infer_expr(expr: &Expr, env: &mut TypeEnv, errors: &mut Vec<TypeError>) -> Type {
    match expr {
        Expr::Literal(Lit::Number(_)) => Type::Number,
        Expr::Literal(Lit::Bool(_)) => Type::Bool,
        Expr::Literal(Lit::String(_)) => Type::String,
        Expr::Literal(Lit::Null) => Type::Null,
        Expr::Var(name) => env.vars.get(name).cloned().unwrap_or(Type::Any),
        Expr::Assign { name, value } => {
            let vt = infer_expr(value, env, errors);
            if let Some(existing) = env.vars.get(name) {
                if !is_compatible(&vt, existing) {
                    errors.push(TypeError {
                        message: format!(
                            "Cannot assign {} to variable '{}' of type {}",
                            vt, name, existing
                        ),
                        subject: Some(name.clone()),
                    });
                }
            }
            env.vars.insert(name.clone(), vt.clone());
            vt
        }
        Expr::Binary { left, op, right } => {
            let l = infer_expr(left, env, errors);
            let r = infer_expr(right, env, errors);
            match op {
                BinOp::Add => {
                    if (l == Type::Number && r == Type::Number)
                        || (l == Type::String && r == Type::String)
                    {
                        if l == Type::Number {
                            Type::Number
                        } else {
                            Type::String
                        }
                    } else {
                        errors.push(TypeError {
                            message: format!("Invalid types for +: {} and {}", l, r),
                            subject: None,
                        });
                        Type::Any
                    }
                }
                BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                    if l == Type::Number && r == Type::Number {
                        Type::Number
                    } else {
                        errors.push(TypeError {
                            message: format!("Number operands required, got {} and {}", l, r),
                            subject: None,
                        });
                        Type::Any
                    }
                }
                BinOp::Eq | BinOp::Ne => Type::Bool,
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    if l == Type::Number && r == Type::Number {
                        Type::Bool
                    } else {
                        errors.push(TypeError {
                            message: format!(
                                "Number operands required for comparison, got {} and {}",
                                l, r
                            ),
                            subject: None,
                        });
                        Type::Any
                    }
                }
                BinOp::And | BinOp::Or => {
                    if l == Type::Bool && r == Type::Bool {
                        Type::Bool
                    } else {
                        errors.push(TypeError {
                            message: format!(
                                "Boolean operands required for logical operation, got {} and {}",
                                l, r
                            ),
                            subject: None,
                        });
                        Type::Any
                    }
                }
            }
        }
        Expr::Unary { op, expr } => {
            let t = infer_expr(expr, env, errors);
            match op {
                UnOp::Neg => {
                    if t == Type::Number {
                        Type::Number
                    } else {
                        errors.push(TypeError {
                            message: format!("Unary - expects number, got {}", t),
                            subject: None,
                        });
                        Type::Any
                    }
                }
                UnOp::Not => {
                    if t == Type::Bool {
                        Type::Bool
                    } else {
                        errors.push(TypeError {
                            message: format!("Unary ! expects bool, got {}", t),
                            subject: None,
                        });
                        Type::Any
                    }
                }
            }
        }
        Expr::Call { callee, args } => {
            let ct = infer_expr(callee, env, errors);
            let arg_ts: Vec<Type> = args.iter().map(|a| infer_expr(a, env, errors)).collect();
            match ct {
                Type::Func(params, ret) => {
                    if params.len() != arg_ts.len() {
                        errors.push(TypeError {
                            message: format!(
                                "Function expects {} args, got {}",
                                params.len(),
                                arg_ts.len()
                            ),
                            subject: None,
                        });
                    } else {
                        for (i, (p, a)) in params.iter().zip(arg_ts.iter()).enumerate() {
                            if !is_compatible(a, p) {
                                errors.push(TypeError {
                                    message: format!(
                                        "Argument {} type {} incompatible with parameter type {}",
                                        i + 1,
                                        a,
                                        p
                                    ),
                                    subject: None,
                                });
                            }
                        }
                    }
                    *ret
                }
                _ => Type::Any,
            }
        }
        Expr::Fn { params, ret, body } => {
            // Create child env
            let mut child = TypeEnv {
                vars: env.vars.clone(),
            };
            let param_types: Vec<Type> = params
                .iter()
                .map(|(n, t)| {
                    let ty = t.as_ref().map(Type::from_expr).unwrap_or(Type::Any);
                    child.vars.insert(n.clone(), ty.clone());
                    ty
                })
                .collect();
            let ret_t = ret.as_ref().map(Type::from_expr).unwrap_or(Type::Any);
            for s in body {
                check_stmt(s, &mut child, Some(&ret_t), errors);
            }
            Type::Func(param_types, Box::new(ret_t))
        }
        Expr::List(items) => {
            let mut t: Option<Type> = None;
            for e in items {
                let et = infer_expr(e, env, errors);
                t = Some(match t {
                    None => et,
                    Some(prev) => unify(prev, et),
                });
            }
            Type::List(Box::new(t.unwrap_or(Type::Any)))
        }
        Expr::Map(props) => {
            let mut t: Option<Type> = None;
            for (_k, e) in props {
                let et = infer_expr(e, env, errors);
                t = Some(match t {
                    None => et,
                    Some(prev) => unify(prev, et),
                });
            }
            Type::Map(Box::new(t.unwrap_or(Type::Any)))
        }
        Expr::Index { target, index } => {
            let tt = infer_expr(target, env, errors);
            let it = infer_expr(index, env, errors);
            match (tt, it) {
                (Type::List(inner), Type::Number) => *inner,
                (Type::Map(inner), Type::String) => *inner,
                (t, i) => {
                    errors.push(TypeError {
                        message: format!("Invalid index types: target {} indexed by {}", t, i),
                        subject: None,
                    });
                    Type::Any
                }
            }
        }
        Expr::Field { target, name: _ } => {
            let _tt = infer_expr(target, env, errors);
            // Dynamic maps only; unknown type for field
            Type::Any
        }
    }
}

fn unify(a: Type, b: Type) -> Type {
    if a == b {
        return a;
    }
    match (a, b) {
        (Type::Any, t) | (t, Type::Any) => t,
        (Type::List(x), Type::List(y)) => Type::List(Box::new(unify(*x, *y))),
        (Type::Map(x), Type::Map(y)) => Type::Map(Box::new(unify(*x, *y))),
        _ => Type::Any,
    }
}

fn is_compatible(a: &Type, b: &Type) -> bool {
    if a == b || *b == Type::Any || *a == Type::Any {
        return true;
    }
    match (a, b) {
        (Type::List(x), Type::List(y)) => is_compatible(x, y),
        (Type::Map(x), Type::Map(y)) => is_compatible(x, y),
        (Type::Null, _) => true, // allow null to flow anywhere
        _ => false,
    }
}

use std::fmt::{Display, Formatter};
impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Number => write!(f, "number"),
            Type::String => write!(f, "string"),
            Type::Bool => write!(f, "bool"),
            Type::Null => write!(f, "null"),
            Type::Any => write!(f, "any"),
            Type::List(i) => write!(f, "list<{}>", i),
            Type::Map(i) => write!(f, "map<{}>", i),
            Type::Func(args, ret) => {
                let parts: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                write!(f, "fn({}) -> {}", parts.join(", "), ret)
            }
        }
    }
}
