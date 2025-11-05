use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::value::{EnvRef, Function, Value};

pub trait HostApi {
    fn call(&self, op: &str, payload: Value) -> Result<Value, String>;
}

#[derive(Default, Clone)]
pub struct Host {
    // simple in-process event bus
    pub events: Rc<RefCell<HashMap<String, Vec<Rc<Function>>>>>,
}

impl Host {
    pub fn on(&self, name: &str, func: Rc<Function>) {
        self.events
            .borrow_mut()
            .entry(name.to_string())
            .or_default()
            .push(func);
    }

    pub fn emit(&self, name: &str, data: Value, env: EnvRef) -> Result<Value, String> {
        if let Some(list) = self.events.borrow().get(name) {
            for f in list.iter() {
                match f.as_ref() {
                    Function::User { .. } => {
                        // call via interpreter will handle
                    }
                    Function::Native { fun, .. } => {
                        fun(vec![data.clone()], env.clone())?;
                    }
                }
            }
        }
        Ok(Value::Null)
    }
}

impl HostApi for Host {
    fn call(&self, op: &str, payload: Value) -> Result<Value, String> {
        // Default stub: just echo as map { ok: true, op, payload }
        let mut m = std::collections::BTreeMap::new();
        m.insert("ok".into(), Value::Bool(true));
        m.insert("op".into(), Value::String(op.to_string()));
        m.insert("payload".into(), payload);
        Ok(Value::Map(m))
    }
}
