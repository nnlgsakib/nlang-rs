use crate::ast::{Expr, Literal};

pub fn sse_new(_args: &[Expr]) -> Result<Expr, String> {
    Ok(Expr::Literal(Literal::Integer(0)))
}

pub fn sse_send(_args: &[Expr]) -> Result<Expr, String> {
    Ok(Expr::Literal(Literal::Null))
}

pub fn sse_close(_args: &[Expr]) -> Result<Expr, String> {
    Ok(Expr::Literal(Literal::Null))
}
