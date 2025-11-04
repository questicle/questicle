use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::value::{EnvRef, Value};

#[derive(Default)]
pub struct Env {
    parent: Option<EnvRef>,
    values: BTreeMap<String, Value>,
}

impl Env {
    pub fn new() -> Self {
        Self {
            parent: None,
            values: BTreeMap::new(),
        }
    }
    pub fn with_parent(parent: EnvRef) -> Self {
        Self {
            parent: Some(parent),
            values: BTreeMap::new(),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        if self.values.contains_key(name) {
            self.values.insert(name.to_string(), value);
            return Ok(());
        }
        if let Some(p) = &self.parent {
            return p.borrow_mut().assign(name, value);
        }
        Err(format!("Undefined variable '{name}'"))
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.values.get(name) {
            return Some(v.clone());
        }
        if let Some(p) = &self.parent {
            return p.borrow().get(name);
        }
        None
    }

    pub fn new_global() -> EnvRef {
        Rc::new(RefCell::new(Self::new()))
    }
    pub fn child_of(parent: &EnvRef) -> EnvRef {
        Rc::new(RefCell::new(Self::with_parent(parent.clone())))
    }
}
