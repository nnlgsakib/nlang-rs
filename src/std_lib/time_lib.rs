use crate::ast::{Expr, Literal};
use std::time::{SystemTime, UNIX_EPOCH, Duration as StdDuration};
use chrono::{TimeZone, Datelike, Timelike, Local, Utc, NaiveDate};

fn fmt_iso(dt: chrono::DateTime<chrono::Utc>) -> String { dt.format("%Y-%m-%d %H:%M:%S").to_string() }
fn fmt_iso_local(dt: chrono::DateTime<chrono::Local>) -> String { dt.format("%Y-%m-%d %H:%M:%S").to_string() }

pub fn builtin_now(_: &[Expr]) -> Result<Expr, String> {
    let dt = Local::now();
    Ok(Expr::Literal(Literal::String(fmt_iso_local(dt))))
}

pub fn builtin_now_utc(_: &[Expr]) -> Result<Expr, String> {
    let dt = Utc::now();
    Ok(Expr::Literal(Literal::String(fmt_iso(dt))))
}

pub fn builtin_now_local(_: &[Expr]) -> Result<Expr, String> {
    let dt = Local::now();
    Ok(Expr::Literal(Literal::String(fmt_iso_local(dt))))
}

pub fn builtin_timestamp(_: &[Expr]) -> Result<Expr, String> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs() as i64;
    Ok(Expr::Literal(Literal::Integer(secs)))
}

pub fn builtin_timestamp_ms(_: &[Expr]) -> Result<Expr, String> {
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis() as i64;
    Ok(Expr::Literal(Literal::Integer(ms)))
}

pub fn builtin_timestamp_us(_: &[Expr]) -> Result<Expr, String> {
    let us = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_micros() as i64;
    Ok(Expr::Literal(Literal::Integer(us)))
}

pub fn builtin_timestamp_ns(_: &[Expr]) -> Result<Expr, String> {
    let ns = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos() as i64;
    Ok(Expr::Literal(Literal::Integer(ns)))
}

pub fn builtin_sleep(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("sleep(seconds) expects 1 argument".into()); }
    let secs = match &args[0] { Expr::Literal(Literal::Integer(i)) => *i, _ => return Err("sleep(seconds) expects integer".into()) };
    std::thread::sleep(StdDuration::from_secs(secs.max(0) as u64));
    Ok(Expr::Literal(Literal::Null))
}

pub fn builtin_sleep_ms(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("sleep_ms(ms) expects 1 argument".into()); }
    let ms = match &args[0] { Expr::Literal(Literal::Integer(i)) => *i, _ => return Err("sleep_ms(ms) expects integer".into()) };
    std::thread::sleep(StdDuration::from_millis(ms.max(0) as u64));
    Ok(Expr::Literal(Literal::Null))
}

pub fn builtin_sleep_ns(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("sleep_ns(ns) expects 1 argument".into()); }
    let ns = match &args[0] { Expr::Literal(Literal::Integer(i)) => *i, _ => return Err("sleep_ns(ns) expects integer".into()) };
    std::thread::sleep(StdDuration::from_nanos(ns.max(0) as u64));
    Ok(Expr::Literal(Literal::Null))
}

pub fn builtin_year(_: &[Expr]) -> Result<Expr, String> { Ok(Expr::Literal(Literal::Integer(chrono::Local::now().year() as i64))) }
pub fn builtin_month(_: &[Expr]) -> Result<Expr, String> { Ok(Expr::Literal(Literal::Integer(chrono::Local::now().month() as i64))) }
pub fn builtin_day(_: &[Expr]) -> Result<Expr, String> { Ok(Expr::Literal(Literal::Integer(chrono::Local::now().day() as i64))) }
pub fn builtin_weekday(_: &[Expr]) -> Result<Expr, String> { Ok(Expr::Literal(Literal::Integer(chrono::Local::now().weekday().num_days_from_monday() as i64))) }
pub fn builtin_hour(_: &[Expr]) -> Result<Expr, String> { Ok(Expr::Literal(Literal::Integer(chrono::Local::now().hour() as i64))) }
pub fn builtin_minute(_: &[Expr]) -> Result<Expr, String> { Ok(Expr::Literal(Literal::Integer(chrono::Local::now().minute() as i64))) }
pub fn builtin_second(_: &[Expr]) -> Result<Expr, String> { Ok(Expr::Literal(Literal::Integer(chrono::Local::now().second() as i64))) }
pub fn builtin_nanosecond(_: &[Expr]) -> Result<Expr, String> { Ok(Expr::Literal(Literal::Integer(0))) }

pub fn builtin_to_local(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("to_local(secs) expects 1 argument".into()); }
    let secs = match &args[0] { Expr::Literal(Literal::Integer(i)) => *i, _ => return Err("to_local expects epoch seconds".into()) };
    let dt = Local.timestamp_opt(secs, 0).single().ok_or("invalid epoch")?;
    Ok(Expr::Literal(Literal::String(fmt_iso_local(dt))))
}

pub fn builtin_to_utc(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("to_utc(secs) expects 1 argument".into()); }
    let secs = match &args[0] { Expr::Literal(Literal::Integer(i)) => *i, _ => return Err("to_utc expects epoch seconds".into()) };
    let dt = Utc.timestamp_opt(secs, 0).single().ok_or("invalid epoch")?;
    Ok(Expr::Literal(Literal::String(fmt_iso(dt))))
}

pub fn builtin_from_timestamp(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("from_timestamp(secs) expects 1 argument".into()); }
    let secs = match &args[0] { Expr::Literal(Literal::Integer(i)) => *i, _ => return Err("from_timestamp expects epoch seconds".into()) };
    let dt = Utc.timestamp_opt(secs, 0).single().ok_or("invalid epoch")?;
    Ok(Expr::Literal(Literal::String(fmt_iso(dt))))
}

pub fn builtin_from_timestamp_ms(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("from_timestamp_ms(ms) expects 1 argument".into()); }
    let ms = match &args[0] { Expr::Literal(Literal::Integer(i)) => *i, _ => return Err("from_timestamp_ms expects integer".into()) };
    let secs = ms / 1000;
    let dt = Utc.timestamp_opt(secs, 0).single().ok_or("invalid epoch")?;
    Ok(Expr::Literal(Literal::String(fmt_iso(dt))))
}

pub fn builtin_format_now_local(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("format(fmt) expects 1 argument".into()); }
    let fmt = match &args[0] { Expr::Literal(Literal::String(s)) => s.clone(), _ => return Err("format expects string".into()) };
    let dt = Local::now();
    Ok(Expr::Literal(Literal::String(dt.format(&fmt).to_string())))
}

pub fn builtin_to_string_now_local(_: &[Expr]) -> Result<Expr, String> { builtin_now_local(&[]) }

pub fn builtin_parse_date(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("parse(date) expects 1 argument".into()); }
    let s = match &args[0] { Expr::Literal(Literal::String(t)) => t.clone(), _ => return Err("parse expects string".into()) };
    let dt = NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let dt = dt.and_hms_opt(0,0,0).ok_or("invalid time")?;
    let ts = Local.from_local_datetime(&dt).single().ok_or("invalid local")?.timestamp();
    Ok(Expr::Literal(Literal::Integer(ts)))
}

pub fn builtin_parse_rfc3339(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("parse_rfc3339(s) expects 1 argument".into()); }
    let s = match &args[0] { Expr::Literal(Literal::String(t)) => t.clone(), _ => return Err("parse_rfc3339 expects string".into()) };
    let dt = chrono::DateTime::parse_from_rfc3339(&s).map_err(|e| e.to_string())?;
    Ok(Expr::Literal(Literal::Integer(dt.timestamp())))
}

pub fn builtin_parse_rfc2822(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("parse_rfc2822(s) expects 1 argument".into()); }
    let s = match &args[0] { Expr::Literal(Literal::String(t)) => t.clone(), _ => return Err("parse_rfc2822 expects string".into()) };
    let dt = chrono::DateTime::parse_from_rfc2822(&s).map_err(|e| e.to_string())?;
    Ok(Expr::Literal(Literal::Integer(dt.timestamp())))
}

fn dur_vault_ns(ns: i64) -> Expr { Expr::VaultLiteral { entries: vec![("nanos".to_string(), Expr::Literal(Literal::Integer(ns)))] } }

pub fn builtin_duration_from_seconds(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("Duration.from_seconds(x) expects 1 argument".into()); }
    let s = match &args[0] { Expr::Literal(Literal::Integer(i)) => *i, _ => return Err("expects integer".into()) };
    Ok(dur_vault_ns(s.saturating_mul(1_000_000_000)))
}

pub fn builtin_duration_from_millis(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("Duration.from_millis(x) expects 1 argument".into()); }
    let ms = match &args[0] { Expr::Literal(Literal::Integer(i)) => *i, _ => return Err("expects integer".into()) };
    Ok(dur_vault_ns(ms.saturating_mul(1_000_000)))
}

pub fn builtin_duration_from_nanos(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("Duration.from_nanos(x) expects 1 argument".into()); }
    let ns = match &args[0] { Expr::Literal(Literal::Integer(i)) => *i, _ => return Err("expects integer".into()) };
    Ok(dur_vault_ns(ns))
}

pub fn builtin_duration_as_secs(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("Duration.as_secs(d) expects 1 argument".into()); }
    let ns = match &args[0] { Expr::VaultLiteral { entries } => entries.iter().find(|(k,_)| k=="nanos").and_then(|(_,v)| if let Expr::Literal(Literal::Integer(i))=v { Some(*i) } else { None }).unwrap_or(0), _ => return Err("expects duration vault".into()) };
    Ok(Expr::Literal(Literal::Integer(ns / 1_000_000_000)))
}

pub fn builtin_duration_as_millis(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("Duration.as_millis(d) expects 1 argument".into()); }
    let ns = match &args[0] { Expr::VaultLiteral { entries } => entries.iter().find(|(k,_)| k=="nanos").and_then(|(_,v)| if let Expr::Literal(Literal::Integer(i))=v { Some(*i) } else { None }).unwrap_or(0), _ => return Err("expects duration vault".into()) };
    Ok(Expr::Literal(Literal::Integer(ns / 1_000_000)))
}

pub fn builtin_duration_add(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=2 { return Err("Duration.add(a,b) expects 2 arguments".into()); }
    let a = match &args[0] { Expr::VaultLiteral { entries } => entries.iter().find(|(k,_)| k=="nanos").and_then(|(_,v)| if let Expr::Literal(Literal::Integer(i))=v { Some(*i) } else { None }).unwrap_or(0), _ => return Err("expects duration vault".into()) };
    let b = match &args[1] { Expr::VaultLiteral { entries } => entries.iter().find(|(k,_)| k=="nanos").and_then(|(_,v)| if let Expr::Literal(Literal::Integer(i))=v { Some(*i) } else { None }).unwrap_or(0), _ => return Err("expects duration vault".into()) };
    Ok(dur_vault_ns(a.saturating_add(b)))
}

pub fn builtin_duration_sub(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=2 { return Err("Duration.sub(a,b) expects 2 arguments".into()); }
    let a = match &args[0] { Expr::VaultLiteral { entries } => entries.iter().find(|(k,_)| k=="nanos").and_then(|(_,v)| if let Expr::Literal(Literal::Integer(i))=v { Some(*i) } else { None }).unwrap_or(0), _ => return Err("expects duration vault".into()) };
    let b = match &args[1] { Expr::VaultLiteral { entries } => entries.iter().find(|(k,_)| k=="nanos").and_then(|(_,v)| if let Expr::Literal(Literal::Integer(i))=v { Some(*i) } else { None }).unwrap_or(0), _ => return Err("expects duration vault".into()) };
    Ok(dur_vault_ns(a.saturating_sub(b)))
}

pub fn builtin_timer_start(_: &[Expr]) -> Result<Expr, String> {
    let ns = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos() as i64;
    Ok(Expr::VaultLiteral { entries: vec![("start_ns".to_string(), Expr::Literal(Literal::Integer(ns)))] })
}

pub fn builtin_timer_elapsed(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("timer.elapsed(timer) expects 1 argument".into()); }
    let start = match &args[0] { Expr::VaultLiteral { entries } => entries.iter().find(|(k,_)| k=="start_ns").and_then(|(_,v)| if let Expr::Literal(Literal::Integer(i))=v { Some(*i) } else { None }).unwrap_or(0), _ => return Err("expects timer vault".into()) };
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos() as i64;
    Ok(Expr::Literal(Literal::Integer((now - start).max(0) / 1_000_000)))
}

pub fn builtin_timer_reset(args: &[Expr]) -> Result<Expr, String> {
    if args.len()!=1 { return Err("timer.reset(timer) expects 1 argument".into()); }
    let _ = match &args[0] { Expr::VaultLiteral { .. } => (), _ => return Err("expects timer vault".into()) };
    let ns = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos() as i64;
    Ok(Expr::VaultLiteral { entries: vec![("start_ns".to_string(), Expr::Literal(Literal::Integer(ns)))] })
}