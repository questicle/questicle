use crate::ast::*;

pub fn format_program(p: &Program) -> String {
    let mut out = String::new();
    for (i, s) in p.statements.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        fmt_stmt(s, 0, &mut out);
    }
    out
}

fn indent(n: usize, out: &mut String) {
    for _ in 0..n {
        out.push_str("    ");
    }
}

fn fmt_stmt(s: &Stmt, ind: usize, out: &mut String) {
    match s {
        Stmt::Let { name, ty, init } => {
            indent(ind, out);
            out.push_str("let ");
            out.push_str(name);
            if let Some(t) = ty {
                out.push_str(": ");
                fmt_type(t, out);
            }
            out.push_str(" = ");
            fmt_expr(init, out);
            out.push(';');
        }
        Stmt::Expr(e) => {
            indent(ind, out);
            fmt_expr(e, out);
            out.push(';');
        }
        Stmt::Block(b) => {
            indent(ind, out);
            out.push_str("{\n");
            for st in b {
                fmt_stmt(st, ind + 1, out);
                out.push('\n');
            }
            indent(ind, out);
            out.push('}');
        }
        Stmt::If {
            cond,
            then_branch,
            else_branch,
        } => {
            indent(ind, out);
            out.push_str("if (");
            fmt_expr(cond, out);
            out.push_str(") ");
            match &**then_branch {
                Stmt::Block(_) => fmt_stmt(then_branch, ind, out),
                other => {
                    out.push_str("{\n");
                    fmt_stmt(other, ind + 1, out);
                    out.push('\n');
                    indent(ind, out);
                    out.push('}');
                }
            }
            if let Some(e) = else_branch {
                out.push_str(" else ");
                match &**e {
                    Stmt::Block(_) => fmt_stmt(e, ind, out),
                    other => {
                        out.push_str("{\n");
                        fmt_stmt(other, ind + 1, out);
                        out.push('\n');
                        indent(ind, out);
                        out.push('}');
                    }
                }
            }
        }
        Stmt::While { cond, body } => {
            indent(ind, out);
            out.push_str("while (");
            fmt_expr(cond, out);
            out.push_str(") ");
            fmt_stmt(body, ind, out);
        }
        Stmt::For { name, iter, body } => {
            indent(ind, out);
            out.push_str("for (");
            out.push_str(name);
            out.push_str(" in ");
            fmt_expr(iter, out);
            out.push_str(") ");
            fmt_stmt(body, ind, out);
        }
        Stmt::Return(v) => {
            indent(ind, out);
            out.push_str("return");
            if let Some(e) = v {
                out.push(' ');
                fmt_expr(e, out);
            }
            out.push(';');
        }
        Stmt::Break => {
            indent(ind, out);
            out.push_str("break;");
        }
        Stmt::Continue => {
            indent(ind, out);
            out.push_str("continue;");
        }
    }
}

fn fmt_expr(e: &Expr, out: &mut String) {
    match e {
        Expr::Literal(Lit::Number(n)) => out.push_str(&crate::value::Value::Number(*n).to_string()),
        Expr::Literal(Lit::Bool(b)) => out.push_str(&b.to_string()),
        Expr::Literal(Lit::String(s)) => {
            out.push('"');
            out.push_str(s);
            out.push('"');
        }
        Expr::Literal(Lit::Null) => out.push_str("null"),
        Expr::Var(n) => out.push_str(n),
        Expr::Assign { name, value } => {
            out.push_str(name);
            out.push_str(" = ");
            fmt_expr(value, out);
        }
        Expr::Binary { left, op, right } => {
            fmt_expr(left, out);
            out.push(' ');
            out.push_str(match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%",
                BinOp::Eq => "==",
                BinOp::Ne => "!=",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::And => "&&",
                BinOp::Or => "||",
            });
            out.push(' ');
            fmt_expr(right, out);
        }
        Expr::Unary { op, expr } => {
            out.push_str(match op {
                UnOp::Neg => "-",
                UnOp::Not => "!",
            });
            fmt_expr(expr, out);
        }
        Expr::Call { callee, args } => {
            fmt_expr(callee, out);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                fmt_expr(a, out);
            }
            out.push(')');
        }
        Expr::Fn { params, ret, body } => {
            out.push_str("fn (");
            for (i, (n, t)) in params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(n);
                if let Some(tt) = t {
                    out.push_str(": ");
                    fmt_type(tt, out);
                }
            }
            out.push(')');
            if let Some(r) = ret {
                out.push_str(" -> ");
                fmt_type(r, out);
            }
            out.push_str(" {\n");
            for s in body {
                fmt_stmt(s, 1, out);
                out.push('\n');
            }
            out.push('}');
        }
        Expr::List(items) => {
            out.push('[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                fmt_expr(it, out);
            }
            out.push(']');
        }
        Expr::Map(props) => {
            out.push('{');
            for (i, (k, v)) in props.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(k);
                out.push_str(": ");
                fmt_expr(v, out);
            }
            out.push('}');
        }
        Expr::Index { target, index } => {
            fmt_expr(target, out);
            out.push('[');
            fmt_expr(index, out);
            out.push(']');
        }
        Expr::Field { target, name } => {
            fmt_expr(target, out);
            out.push('.');
            out.push_str(name);
        }
    }
}

fn fmt_type(t: &TypeExpr, out: &mut String) {
    match t {
        TypeExpr::Number => out.push_str("number"),
        TypeExpr::String => out.push_str("string"),
        TypeExpr::Bool => out.push_str("bool"),
        TypeExpr::Null => out.push_str("null"),
        TypeExpr::Any => out.push_str("any"),
        TypeExpr::List(i) => {
            out.push_str("list<");
            fmt_type(i, out);
            out.push('>');
        }
        TypeExpr::Map(i) => {
            out.push_str("map<");
            fmt_type(i, out);
            out.push('>');
        }
        TypeExpr::Func(args, ret) => {
            out.push_str("fn(");
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                fmt_type(a, out);
            }
            out.push(')');
            out.push_str(" -> ");
            fmt_type(ret, out);
        }
    }
}
