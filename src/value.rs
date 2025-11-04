use crate::ast::Stmt;
use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

#[derive(Clone)]
pub enum Value {
    Number(f64),
    Bool(bool),
    String(String),
    Null,
    List(Vec<Value>),
    Map(std::collections::BTreeMap<String, Value>),
    Function(Rc<Function>),
}

#[derive(Clone)]
pub enum Function {
    User {
        params: Vec<String>,
        body: Vec<Stmt>,
        env: EnvRef,
    },
    Native {
        name: String,
        fun: NativeFn,
    },
}

pub type EnvRef = Rc<RefCell<crate::env::Env>>;
pub type NativeFn = Rc<dyn Fn(Vec<Value>, EnvRef) -> Result<Value, String>>;

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::List(v) => !v.is_empty(),
            Value::Map(m) => !m.is_empty(),
            Value::Function(_) => true,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", trim_float(*n)),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Null => write!(f, "null"),
            Value::List(v) => {
                let parts: Vec<String> = v.iter().map(|x| format!("{}", x)).collect();
                write!(f, "[{}]", parts.join(", "))
            }
            Value::Map(m) => {
                let parts: Vec<String> = m.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{{{}}}", parts.join(", "))
            }
            Value::Function(_) => write!(f, "<fn>"),
        }
    }
}

fn trim_float(n: f64) -> String {
    let s = format!("{}", n);
    if s.contains('.') {
        let s2 = s.trim_end_matches('0').trim_end_matches('.').to_string();
        if s2.is_empty() {
            "0".to_string()
        } else {
            s2
        }
    } else {
        s
    }
}
