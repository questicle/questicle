use rand::Rng;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::host::{Host, HostApi};
use crate::value::{EnvRef, Function, Value};

pub fn install_std(env: &EnvRef, host: Host) {
    let mut e = env.borrow_mut();
    e.define(
        "print".into(),
        native("print", |args, _| {
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                match a {
                    Value::String(s) => print!("{}", s),
                    v => print!("{}", v),
                }
            }
            println!();
            Ok(Value::Null)
        }),
    );
    e.define(
        "clock".into(),
        native("clock", |_args, _| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            Ok(Value::Number(now))
        }),
    );
    e.define(
        "random".into(),
        native("random", |_args, _| {
            let mut rng = rand::thread_rng();
            Ok(Value::Number(rng.gen::<f64>()))
        }),
    );
    e.define(
        "len".into(),
        native("len", |args, _| {
            let n = match args.first() {
                Some(Value::String(s)) => s.chars().count(),
                Some(Value::List(v)) => v.len(),
                Some(Value::Map(m)) => m.len(),
                _ => 0,
            } as f64;
            Ok(Value::Number(n))
        }),
    );
    e.define(
        "keys".into(),
        native("keys", |args, _| {
            if let Some(Value::Map(m)) = args.first() {
                Ok(Value::List(m.keys().cloned().map(Value::String).collect()))
            } else {
                Ok(Value::List(vec![]))
            }
        }),
    );
    e.define(
        "push".into(),
        native("push", |args, _| match (args.first(), args.get(1)) {
            (Some(Value::List(list)), Some(val)) => {
                let mut new = list.clone();
                new.push(val.clone());
                Ok(Value::List(new))
            }
            _ => Err("push expects (list, value)".into()),
        }),
    );
    e.define(
        "pop".into(),
        native("pop", |args, _| {
            if let Some(Value::List(list)) = args.first() {
                let mut v = list.clone();
                Ok(v.pop().unwrap_or(Value::Null))
            } else {
                Ok(Value::Null)
            }
        }),
    );

    let host_clone = host.clone();
    e.define(
        "host".into(),
        native("host", move |args, _| {
            let op = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("host(op, payload)".into()),
            };
            let payload = args.get(1).cloned().unwrap_or(Value::Null);
            host_clone.call(&op, payload)
        }),
    );

    let host_for_on = host.clone();
    e.define(
        "on".into(),
        native("on", move |args, _env| {
            let name = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("on(name, fn)".into()),
            };
            match args.get(1) {
                Some(Value::Function(f)) => {
                    host_for_on.on(&name, f.clone());
                    Ok(Value::Null)
                }
                _ => Err("on(name, fn)".into()),
            }
        }),
    );
    let host_for_emit = host.clone();
    e.define(
        "emit".into(),
        native("emit", move |args, env| {
            let name = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("emit(name, data)".into()),
            };
            let data = args.get(1).cloned().unwrap_or(Value::Null);
            host_for_emit.emit(&name, data, env)
        }),
    );
}

fn native(name: &str, f: impl Fn(Vec<Value>, EnvRef) -> Result<Value, String> + 'static) -> Value {
    Value::Function(Rc::new(Function::Native {
        name: name.to_string(),
        fun: Rc::new(f),
    }))
}
