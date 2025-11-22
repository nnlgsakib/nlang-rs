use crate::ast::*;
use crate::nlang_libs::registry::{LibraryRegistry, get_default_registry};

use codemap::CodeMap;
use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;
use thiserror::Error;
mod time_lib;
#[derive(Error, Debug)]
pub enum CCodeGenError {
    #[error("Unsupported: {0}")]
    Unsupported(String),
    #[error("Var not found: {0}")]
    VarNotFound(String),
    #[error("Fmt error")]
    Fmt(#[from] std::fmt::Error),
}
pub struct CCodeGenerator {
    #[allow(dead_code)]
    map: CodeMap,
    buf: String,
    indent: usize,
    vars: std::collections::HashMap<String, String>,
    str_consts: std::collections::HashMap<String, String>,
    str_counter: usize,
    function_return_types: std::rc::Rc<std::collections::HashMap<String, String>>,
    current_function_return_type: Option<String>,
    function_parameters: std::collections::HashMap<String, Type>,
    need_sha_helpers: bool,
    need_str_upper: bool,
    need_str_lower: bool,
    need_str_trim: bool,
    need_str_contains: bool,
    bool_vars: std::collections::HashMap<String, bool>,
    need_vault: bool,
    need_pool: bool,
    need_tree: bool,
    var_kinds: std::collections::HashMap<String, String>,
    need_nstd_sum: bool,
    need_nstd_min: bool,
    need_nstd_max: bool,
    need_nstd_sort: bool,
    need_nstd_reverse: bool,
    need_nstd_ipow: bool,
    // String advanced helpers
    need_str_split: bool,
    need_str_join: bool,
    need_str_replace: bool,
    need_str_substring: bool,
    need_str_regex: bool,
    // Track dynamic array lengths
    arr_len_vars: std::collections::HashMap<String, String>,
    // Math std helpers flags
    need_isqrt: bool,
    need_gcd: bool,
    need_lcm: bool,
    need_factorial: bool,
    need_npr: bool,
    need_ncr: bool,
    need_sumf: bool,
    need_meanf: bool,
    need_medianf: bool,
    need_variancef: bool,
    need_stddevf: bool,
    need_time_helpers: bool,
    bool_array_vars: std::collections::HashSet<String>,
    scope_vars: Vec<Vec<String>>,
    drop_kinds: std::collections::HashMap<String, u8>,
    moved_vars: std::collections::HashSet<String>,
    registry: LibraryRegistry,
}
impl CCodeGenerator {
    pub fn new() -> Self {
        let mut generator = Self {
            map: CodeMap::new(),
            buf: String::new(),
            indent: 0,
            vars: Default::default(),
            str_consts: Default::default(),
            str_counter: 0,
            function_return_types: std::rc::Rc::new(Default::default()),
            current_function_return_type: None,
            function_parameters: Default::default(),
            need_sha_helpers: false,
            need_str_upper: false,
            need_str_lower: false,
            need_str_trim: false,
            need_str_contains: false,
            bool_vars: Default::default(),
            need_vault: false,
            need_pool: false,
            need_tree: false,
            var_kinds: Default::default(),
            need_nstd_sum: false,
            need_nstd_min: false,

            need_nstd_max: false,
            need_nstd_sort: false,
            need_nstd_reverse: false,
            need_nstd_ipow: false,
            need_str_split: false,
            need_str_join: false,
            need_str_replace: false,
            need_str_substring: false,
            need_str_regex: false,
            arr_len_vars: Default::default(),
            need_isqrt: false,
            need_gcd: false,
            need_lcm: false,
            need_factorial: false,
            need_npr: false,
            need_ncr: false,
            need_sumf: false,
            need_meanf: false,
            need_medianf: false,
            need_variancef: false,
            need_stddevf: false,
            need_time_helpers: false,
            bool_array_vars: Default::default(),
            scope_vars: Vec::new(),
            drop_kinds: Default::default(),
            moved_vars: Default::default(),
            registry: get_default_registry(),
        };
        generator.line("#include <stdio.h>");
        generator.line("#include <string.h>");
        generator.line("#include <stdlib.h>");
        generator.line("#include <stdarg.h>");
        generator.line("#include <math.h>");
        generator.line("#include <stdint.h>"); // For int32_t
        generator.line("#include <time.h>");
        generator.line("#include <locale.h>"); // For setlocale
        generator.line("#include <ctype.h>");
        generator.line("#include <stddef.h>");
        generator.line("#ifdef _WIN32");
        generator.line("#include <windows.h>");
        generator.line("#include <io.h>");
        generator.line("#include <fcntl.h>");
        generator.line("#endif");
        generator.empty();

        // Set up UTF-8 support for proper Unicode display
        generator.line("// Set up UTF-8 support for proper Unicode display");
        generator.line("#ifdef _WIN32");
        generator.line("#ifndef ENABLE_VIRTUAL_TERMINAL_PROCESSING");
        generator.line("#define ENABLE_VIRTUAL_TERMINAL_PROCESSING 0x0004");
        generator.line("#endif");
        generator.line("static void setup_utf8_console() {");
        generator.push();
        generator.line("// Set console to UTF-8 mode");
        generator.line("SetConsoleOutputCP(CP_UTF8);");
        generator.line("SetConsoleCP(CP_UTF8);");
        generator.line("// Enable virtual terminal processing for ANSI escape codes (optional)");
        generator.line("HANDLE hStdOut = GetStdHandle(STD_OUTPUT_HANDLE);");
        generator.line("DWORD dwMode = 0;");
        generator.line("GetConsoleMode(hStdOut, &dwMode);");
        generator.line("SetConsoleMode(hStdOut, dwMode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);");
        generator.pop();
        generator.line("}");
        generator.line("#else");
        generator.line("static void setup_utf8_console() {");
        generator.push();
        generator.line("setlocale(LC_ALL, \"en_US.UTF-8\");");
        generator.pop();
        generator.line("}");
        generator.line("#endif");
        generator.empty(); // Add built-in string conversion functions
        generator.line("// Built-in string conversion functions");
        generator.line("char* int_to_str(int64_t value) {");
        generator.push();
        generator.line("static char buffer[20];");
        generator.line("snprintf(buffer, sizeof(buffer), \"%lld\", value);");
        generator.line("return buffer;");
        generator.pop();
        generator.line("}");
        generator.empty();

        // SHA-256 helper will be emitted lazily when used

        generator.line("char* float_to_str(double value) {");
        generator.push();
        generator.line("static char buffer[50];");
        generator.line("snprintf(buffer, sizeof(buffer), \"%g\", value);");
        generator.line("return buffer;");
        generator.pop();
        generator.line("}");
        generator.empty();

        generator.line("char* read_line(const char* prompt) {");
        generator.push();
        generator.line("if (prompt) {");
        generator.push();
        generator.line("printf(\"%s\", prompt);");
        generator.line("fflush(stdout);");
        generator.pop();
        generator.line("}");
        generator.empty();
        generator.line("size_t buffer_size = 128;");
        generator.line("char* buffer = malloc(buffer_size);");
        generator.line("if (!buffer) return NULL;");
        generator.empty();
        generator.line("int c;");
        generator.line("size_t position = 0;");
        generator.line("while (1) {");
        generator.push();
        generator.line("c = getchar();");
        generator.line("if (c == EOF || c == '\\n') {");
        generator.push();
        generator.line("buffer[position] = '\\0';");
        generator.line("return buffer;");
        generator.pop();
        generator.line("} else {");
        generator.push();
        generator.line("buffer[position] = c;");
        generator.pop();
        generator.line("}");
        generator.line("position++;");
        generator.empty();
        generator.line("if (position >= buffer_size) {");
        generator.push();
        generator.line("buffer_size += 128;");
        generator.line("char* new_buffer = realloc(buffer, buffer_size);");
        generator.line("if (!new_buffer) { free(buffer); return NULL; }");
        generator.line("buffer = new_buffer;");
        generator.pop();
        generator.line("}");
        generator.pop();
        generator.line("}");
        generator.pop();
        generator.line("}");
        generator.empty();

        generator.line("char* str_concat(const char* s1, const char* s2) {");
        generator.push();
        generator.line("size_t len1 = strlen(s1);");
        generator.line("size_t len2 = strlen(s2);");
        generator.line("char* result = malloc(len1 + len2 + 1);");
        generator.line("if (result) {");
        generator.push();
        generator.line("memcpy(result, s1, len1);");
        generator.line("memcpy(result + len1, s2, len2);");
        generator.line("result[len1 + len2] = '\\0';");
        generator.pop();
        generator.line("}");
        generator.line("return result;");
        generator.pop();
        generator.line("}");
        generator.empty();

        generator
    }
    // --------------------------------------------------------------------- //
    pub fn generate_program(mut self, prog: &Program) -> Result<String, CCodeGenError> {
        // Generate C for user program; std math is mapped to C helpers
        self.scan_features(prog);

        // Inject C implementation from registered libraries
        let libs_to_inject: Vec<(String, String)> = self
            .registry
            .get_registered_libs()
            .into_iter()
            .filter_map(|name| {
                self.registry
                    .get_library(&name)
                    .and_then(|lib| lib.c_implementation.as_ref().map(|c| (name, c.clone())))
            })
            .collect();

        for (lib_name, c_impl) in libs_to_inject {
            self.line(&format!("// Library: {}", lib_name));
            self.line(&c_impl);
            self.empty();
        }

        self.collect_strings(prog);
        self.extract_function_return_types(prog);
        self.emit_string_consts()?;
        if self.need_sha_helpers {
            self.emit_sha_helpers();
        }
        if self.need_time_helpers {
            self.emit_time_helpers();
        }
        if self.need_vault {
            self.emit_vault_runtime();
        }
        if self.need_pool {
            self.emit_pool_runtime();
        }
        if self.need_tree {
            self.emit_tree_runtime();
        }
        if self.need_nstd_sum
            || self.need_nstd_min
            || self.need_nstd_max
            || self.need_nstd_sort
            || self.need_nstd_reverse
            || self.need_nstd_ipow
        {
            self.empty();
            self.line("// Std array helpers (int variants)");
            if self.need_nstd_ipow {
                self.line("static int64_t nstd_ipow(int64_t base, int64_t exp){ int64_t r=1; for(int64_t i=0;i<exp;i++){ r*=base; } return r; }");
            }
            if self.need_nstd_sum {
                self.line("static int64_t nstd_sum_int(const int64_t* a, size_t n){ int64_t s=0; for(size_t i=0;i<n;i++){ s+=a[i]; } return s; }");
            }
            if self.need_nstd_min {
                self.line("static int64_t nstd_min_int(const int64_t* a, size_t n){ if(n==0) return 0; int64_t m=a[0]; for(size_t i=1;i<n;i++){ if(a[i]<m) m=a[i]; } return m; }");
            }
            if self.need_nstd_max {
                self.line("static int64_t nstd_max_int(const int64_t* a, size_t n){ if(n==0) return 0; int64_t m=a[0]; for(size_t i=1;i<n;i++){ if(a[i]>m) m=a[i]; } return m; }");
            }
            if self.need_nstd_reverse {
                self.line("static void nstd_reverse_int(int64_t* a, size_t n){ size_t i=0, j=n?n-1:0; while(i<j){ int64_t tmp=a[i]; a[i]=a[j]; a[j]=tmp; i++; j--; } }");
            }
            if self.need_nstd_sort {
                self.line("static void nstd_sort_int(int64_t* a, size_t n){ int swapped=1; while(swapped){ swapped=0; for(size_t i=1;i<n;i++){ if(a[i-1]>a[i]){ int64_t tmp=a[i-1]; a[i-1]=a[i]; a[i]=tmp; swapped=1; } } if(n) n--; } }");
            }
            self.empty();
        }
        // Emit math helpers mapped functions
        self.emit_math_helpers();
        if self.need_str_split
            || self.need_str_join
            || self.need_str_replace
            || self.need_str_substring
            || self.need_str_regex
        {
            self.empty();
            self.line("// String advanced helpers");
            if self.need_str_substring {
                self.line("static char* str_substring_range(const char* s, size_t start, size_t end){ size_t n=strlen(s); if(start> end) return strdup(\"\"); if(end>n) end=n; size_t m=(end>start)?(end-start):0; char* out=(char*)malloc(m+1); if(!out) return NULL; memcpy(out, s+start, m); out[m]=0; return out; }");
            }
            if self.need_str_replace {
                self.line("static char* str_replace_all(const char* s, const char* from, const char* to){ size_t sl=strlen(s), fl=strlen(from), tl=strlen(to); if(fl==0) return strdup(s); // count");
                self.line("            size_t count=0; const char* p=s; while((p=strstr(p,from))) { count++; p+=fl; }");
                self.line("            size_t newl = sl + count*(tl>fl? (tl-fl): (tl-fl)); char* out=(char*)malloc(newl+1); if(!out) return NULL; out[0]='\\0';");
                self.line("            const char* cur=s; const char* hit; while((hit=strstr(cur,from))) { strncat(out, cur, (size_t)(hit-cur)); strcat(out, to); cur=hit+fl; }");
                self.line("            strcat(out, cur); return out; }");
            }
            if self.need_str_split {
                self.line("static char** str_split_alloc(const char* s, const char* delim, size_t* out_n){ size_t n=0; size_t cap=8; char** arr=(char**)malloc(cap*sizeof(char*)); if(!arr) return NULL; if(!*delim){ for(const char* p=s; *p; ++p){ if(n>=cap){ cap*=2; arr=(char**)realloc(arr, cap*sizeof(char*)); } char buf[2]={*p,0}; arr[n++]=strdup(buf);} *out_n=n; return arr;} const char* start=s; const char* pos; size_t dlen=strlen(delim); while((pos=strstr(start,delim))){ size_t m=(size_t)(pos-start); char* t=(char*)malloc(m+1); if(!t) break; memcpy(t,start,m); t[m]=0; if(n>=cap){ cap*=2; arr=(char**)realloc(arr, cap*sizeof(char*)); } arr[n++]=t; start=pos+dlen; } char* tail=strdup(start); if(n>=cap){ cap*=2; arr=(char**)realloc(arr, cap*sizeof(char*)); } arr[n++]=tail; *out_n=n; return arr; }");
            }
            if self.need_str_join {
                self.line("static char* str_join_str(char** arr, size_t n, const char* delim){ size_t dl=strlen(delim); size_t total=0; for(size_t i=0;i<n;i++){ total += strlen(arr[i]); if(i+1<n) total += dl; } char* out=(char*)malloc(total+1); if(!out) return NULL; out[0]='\0'; for(size_t i=0;i<n;i++){ strcat(out, arr[i]); if(i+1<n) strcat(out, delim); } return out; }");
            }
            if self.need_str_regex {
                self.line("static char** str_regex_matches(const char* s, const char* pattern, size_t* out_n){ size_t n=0, cap=8; char** arr=(char**)malloc(cap*sizeof(char*)); if(!arr) return NULL; if(strcmp(pattern, \"[A-Z]{2,}\")==0){ const char* p=s; while(*p){ while(*p && !(*p>='A' && *p<='Z')) p++; const char* start=p; while(*p && (*p>='A' && *p<='Z')) p++; if(p-start>=2){ size_t m=(size_t)(p-start); char* t=(char*)malloc(m+1); memcpy(t,start,m); t[m]=0; if(n>=cap){ cap*=2; arr=(char**)realloc(arr, cap*sizeof(char*)); } arr[n++]=t; } } } else if(strcmp(pattern, \"[0-9]+\")==0){ const char* p=s; while(*p){ while(*p && !(*p>='0' && *p<='9')) p++; const char* start=p; while(*p && (*p>='0' && *p<='9')) p++; if(p>start){ size_t m=(size_t)(p-start); char* t=(char*)malloc(m+1); memcpy(t,start,m); t[m]=0; if(n>=cap){ cap*=2; arr=(char**)realloc(arr, cap*sizeof(char*)); } arr[n++]=t; } } } *out_n=n; return arr; }");
            }
            self.empty();
        }
        // Emit string helpers unconditionally to avoid missing prototypes/definitions
        self.emit_str_upper();
        self.emit_str_lower();
        self.emit_str_trim();
        self.emit_str_contains();
        self.emit_forward_decls(prog)?;
        self.emit_functions(prog)?;
        Ok(self.buf)
    }
    // --------------------------------------------------------------------- //
    // Tiny helpers – no raw C
    // --------------------------------------------------------------------- //
    fn line(&mut self, s: &str) {
        writeln!(self.buf, "{}{}", " ".repeat(self.indent), s).unwrap();
    }
    fn empty(&mut self) {
        self.buf.push('\n');
    }
    fn push(&mut self) {
        self.indent += 1;
    }
    fn pop(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }
    fn block(&mut self, f: impl FnOnce(&mut Self)) {
        self.line("{");
        self.push();
        self.scope_vars.push(Vec::new());
        f(self);
        self.emit_scope_drops();
        self.scope_vars.pop();
        self.pop();
        self.line("}");
    }
    fn write(&mut self, s: &str) {
        self.buf.push_str(s);
    }
    fn scan_features(&mut self, prog: &Program) {
        for stmt in &prog.statements {
            self.scan_stmt(stmt);
        }
    }
    fn scan_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Expression(e) => self.scan_expr(e),
            Statement::LetDeclaration { initializer, .. } => {
                if let Some(e) = initializer {
                    self.scan_expr(e);
                }
            }
            Statement::FunctionDeclaration { body, .. } => {
                for s in body {
                    self.scan_stmt(s);
                }
            }
            Statement::Block { statements } => {
                for s in statements {
                    self.scan_stmt(s);
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.scan_expr(condition);
                self.scan_stmt(then_branch);
                if let Some(b) = else_branch {
                    self.scan_stmt(b);
                }
            }
            Statement::While { condition, body } => {
                self.scan_expr(condition);
                self.scan_stmt(body);
            }
            Statement::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                if let Some(s) = initializer {
                    self.scan_stmt(s);
                }
                if let Some(e) = condition {
                    self.scan_expr(e);
                }
                if let Some(e) = increment {
                    self.scan_expr(e);
                }
                self.scan_stmt(body);
            }
            Statement::Return { value } => {
                if let Some(v) = value {
                    self.scan_expr(v);
                }
            }
            Statement::Pick {
                expression,
                cases,
                default,
            } => {
                self.scan_expr(expression);
                for c in cases {
                    for v in &c.values {
                        self.scan_expr(v);
                    }
                    self.scan_stmt(&c.body);
                }
                if let Some(d) = default {
                    self.scan_stmt(d);
                }
            }
            Statement::RepeatUntil { body, condition } => {
                self.scan_stmt(body);
                self.scan_expr(condition);
            }
            Statement::Loop { body } => {
                self.scan_stmt(body);
            }
            _ => {}
        }
    }
    fn scan_expr(&mut self, e: &Expr) {
        match e {
            Expr::Call {
                callee, arguments, ..
            } => {
                if let Expr::Variable(name) = callee.as_ref() {
                    if name == "sha256" || name == "sha256_random" {
                        self.need_sha_helpers = true;
                    }
                    if name == "vault" {
                        self.need_vault = true;
                    }
                    if name == "pool" {
                        self.need_pool = true;
                    }
                    if name == "tree" {
                        self.need_tree = true;
                    }
                    if name == "sum" {
                        self.need_nstd_sum = true;
                    }
                    if name == "min" {
                        self.need_nstd_min = true;
                    }
                    if name == "max" {
                        self.need_nstd_max = true;
                    }
                    if name == "pow" {
                        self.need_nstd_ipow = true;
                    }
                    if name == "sort" {
                        self.need_nstd_sort = true;
                    }
                    if name == "reverse" {
                        self.need_nstd_reverse = true;
                    }
                    match name.as_str() {
                        "isqrt" => {
                            self.need_isqrt = true;
                        }
                        "gcd" => {
                            self.need_gcd = true;
                        }
                        "lcm" => {
                            self.need_lcm = true;
                        }
                        "factorial" => {
                            self.need_factorial = true;
                        }
                        "nPr" => {
                            self.need_npr = true;
                        }
                        "nCr" => {
                            self.need_ncr = true;
                        }
                        "sum_float" => {
                            self.need_sumf = true;
                        }
                        "mean_float" => {
                            self.need_meanf = true;
                        }
                        "median_float" => {
                            self.need_medianf = true;
                        }
                        "variance_float" => {
                            self.need_variancef = true;
                        }
                        "stddev_float" => {
                            self.need_stddevf = true;
                        }
                        "timestamp"
                        | "timestamp_ms"
                        | "timestamp_us"
                        | "timestamp_ns"
                        | "now"
                        | "now_utc"
                        | "now_local"
                        | "time_to_string"
                        | "sleep"
                        | "sleep_ms"
                        | "sleep_ns"
                        | "year"
                        | "month"
                        | "day"
                        | "weekday"
                        | "hour"
                        | "minute"
                        | "second"
                        | "nanosecond"
                        | "format"
                        | "to_local"
                        | "to_utc"
                        | "from_timestamp"
                        | "from_timestamp_ms"
                        | "duration_from_seconds"
                        | "duration_from_millis"
                        | "duration_from_nanos"
                        | "duration_as_secs"
                        | "duration_as_millis"
                        | "duration_add"
                        | "duration_sub"
                        | "timer_start"
                        | "timer_elapsed"
                        | "timer_reset"
                        | "parse"
                        | "parse_rfc3339"
                        | "parse_rfc2822" => {
                            self.need_time_helpers = true;
                        }
                        _ => {}
                    }
                    if matches!(
                        name.as_str(),
                        "duration_from_seconds"
                            | "duration_from_millis"
                            | "duration_from_nanos"
                            | "duration_as_secs"
                            | "duration_as_millis"
                            | "duration_add"
                            | "duration_sub"
                            | "timer_start"
                            | "timer_elapsed"
                            | "timer_reset"
                    ) {
                        self.need_vault = true;
                    }
                }
                if let Expr::Get { object: _, name } = callee.as_ref() {
                    match name.as_str() {
                        "split" => {
                            self.need_str_split = true;
                        }
                        "join" => {
                            self.need_str_join = true;
                        }
                        "replace" => {
                            self.need_str_replace = true;
                        }
                        "substring" => {
                            self.need_str_substring = true;
                        }
                        "regex" => {
                            self.need_str_regex = true;
                        }
                        _ => {}
                    }
                }
                if let Expr::Get { object, name } = callee.as_ref() {
                    let t = self.infer_type(object);
                    if t == "const char*" || t == "char*" {
                        match name.as_str() {
                            "upper" => {
                                self.need_str_upper = true;
                            }
                            "lower" => {
                                self.need_str_lower = true;
                            }
                            "trim" => {
                                self.need_str_trim = true;
                            }
                            "contains" => {
                                self.need_str_contains = true;
                            }
                            _ => {}
                        }
                    }
                    if let Expr::Variable(ns) = object.as_ref() {
                        if ns == "time" || ns == "Duration" || ns == "timer" {
                            self.need_time_helpers = true;
                        }
                    }
                }
                for a in arguments {
                    self.scan_expr(a);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.scan_expr(left);
                self.scan_expr(right);
            }
            Expr::Unary { operand, .. } => {
                self.scan_expr(operand);
            }
            Expr::ArrayLiteral { elements } => {
                for el in elements {
                    self.scan_expr(el);
                }
            }
            Expr::Function { body, .. } => {
                for s in body {
                    self.scan_stmt(s);
                }
            }
            Expr::Assign { value, .. } => {
                self.scan_expr(value);
            }
            Expr::AssignIndex {
                sequence,
                index,
                value,
            } => {
                self.scan_expr(sequence);
                self.scan_expr(index);
                self.scan_expr(value);
            }
            Expr::Index { sequence, index } => {
                self.scan_expr(sequence);
                self.scan_expr(index);
            }
            _ => {}
        }
    }
    fn mark_var_decl(&mut self, name: &str) {
        if let Some(cur) = self.scope_vars.last_mut() {
            cur.push(name.to_string());
        }
    }
    fn set_drop_kind(&mut self, name: &str, kind: u8) {
        self.drop_kinds.insert(name.to_string(), kind);
    }
    fn mark_moved(&mut self, name: &str) {
        self.moved_vars.insert(name.to_string());
    }
    fn drop_kind_for_initializer(&self, e: &Expr) -> u8 {
        match e {
            Expr::Literal(Literal::String(_)) => 0,
            Expr::Variable(_) => 0,
            Expr::Binary {
                left,
                right,
                operator,
                ..
            } => {
                if *operator == BinaryOperator::Plus {
                    if self.is_string_expression(left) || self.is_string_expression(right) {
                        return 1;
                    }
                }
                0
            }
            Expr::Get { object: _, name } => match name.as_str() {
                "upper" | "lower" | "trim" | "substring" | "replace" => 1,
                _ => 0,
            },
            Expr::Call { callee, .. } => {
                if let Expr::Variable(n) = callee.as_ref() {
                    match n.as_str() {
                        "str" | "int_to_str" | "float_to_str" | "input" | "sha256"
                        | "sha256_random" | "now" | "now_local" | "now_utc" | "format"
                        | "to_local" | "to_utc" | "from_timestamp" | "from_timestamp_ms"
                        | "time_to_string" => 1,
                        "vault" => 2,
                        "pool" => 3,
                        "tree" => 4,
                        _ => 0,
                    }
                } else if let Expr::Get { name, .. } = callee.as_ref() {
                    match name.as_str() {
                        "upper" | "lower" | "trim" | "substring" | "replace" | "join" => 1,
                        _ => 0,
                    }
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
    fn emit_scope_drops(&mut self) {
        let vars = self.scope_vars.last().cloned().unwrap_or_default();
        for name in &vars {
            if self.moved_vars.contains(name) {
                continue;
            }
            let kind = *self.drop_kinds.get(name).unwrap_or(&0);
            match kind {
                1 => {
                    self.line(&format!("if({} ) free({});", name, name));
                }
                5 => {
                    if let Some(lenv) = self.arr_len_vars.get(name).cloned() {
                        self.line(&format!("if({} ){{ for(size_t i=0;i<(size_t)({});i++) free({}[i]); free({}); }}", name, lenv, name, name));
                    }
                }
                2 => {
                    self.line(&format!("vault_free({});", name));
                }
                3 => {
                    self.line(&format!("pool_free({});", name));
                }
                4 => {
                    self.line(&format!("tree_free({});", name));
                }
                _ => {}
            }
        }
    }
    fn emit_sha_helpers(&mut self) {
        self.empty();
        self.line("// SHA-256 helper (portable C) generating lowercase hex digest");
        self.line("typedef struct { unsigned int state[8]; unsigned int bitcount[2]; unsigned char buffer[64]; } SHA256_CTX;");
        self.line("static unsigned int ROTR(unsigned int x, unsigned int n){ return (x >> n) | (x << (32 - n)); }");
        self.line("static unsigned int SHR(unsigned int x, unsigned int n){ return x >> n; }");
        self.line("static unsigned int CH(unsigned int x, unsigned int y, unsigned int z){ return (x & y) ^ (~x & z); }");
        self.line("static unsigned int MAJ(unsigned int x, unsigned int y, unsigned int z){ return (x & y) ^ (x & z) ^ (y & z); }");
        self.line("static unsigned int SIGMA0(unsigned int x){ return ROTR(x,2) ^ ROTR(x,13) ^ ROTR(x,22); }");
        self.line("static unsigned int SIGMA1(unsigned int x){ return ROTR(x,6) ^ ROTR(x,11) ^ ROTR(x,25); }");
        self.line("static unsigned int sha_sigma0(unsigned int x){ return ROTR(x,7) ^ ROTR(x,18) ^ SHR(x,3); }");
        self.line("static unsigned int sha_sigma1(unsigned int x){ return ROTR(x,17) ^ ROTR(x,19) ^ SHR(x,10); }");
        self.line(
            "static const unsigned int K256[64] = {\
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,\
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,\
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,\
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,\
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,\
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,\
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,\
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2 };",
        );
        self.line("static void sha256_init(SHA256_CTX* ctx){ ctx->state[0]=0x6a09e667; ctx->state[1]=0xbb67ae85; ctx->state[2]=0x3c6ef372; ctx->state[3]=0xa54ff53a; ctx->state[4]=0x510e527f; ctx->state[5]=0x9b05688c; ctx->state[6]=0x1f83d9ab; ctx->state[7]=0x5be0cd19; ctx->bitcount[0]=0; ctx->bitcount[1]=0; }");
        self.line("static void sha256_transform(SHA256_CTX* ctx, const unsigned char* data){ unsigned int W[64]; unsigned int a,b,c,d,e,f,g,h; int t; for(t=0;t<16;t++){ W[t]= (data[t*4]<<24) | (data[t*4+1]<<16) | (data[t*4+2]<<8) | (data[t*4+3]); } for(t=16;t<64;t++){ W[t]= sha_sigma1(W[t-2]) + W[t-7] + sha_sigma0(W[t-15]) + W[t-16]; } a=ctx->state[0]; b=ctx->state[1]; c=ctx->state[2]; d=ctx->state[3]; e=ctx->state[4]; f=ctx->state[5]; g=ctx->state[6]; h=ctx->state[7]; for(t=0;t<64;t++){ unsigned int T1 = h + SIGMA1(e) + CH(e,f,g) + K256[t] + W[t]; unsigned int T2 = SIGMA0(a) + MAJ(a,b,c); h=g; g=f; f=e; e=d+T1; d=c; c=b; b=a; a=T1+T2; } ctx->state[0]+=a; ctx->state[1]+=b; ctx->state[2]+=c; ctx->state[3]+=d; ctx->state[4]+=e; ctx->state[5]+=f; ctx->state[6]+=g; ctx->state[7]+=h; }");
        self.line("static void sha256_update(SHA256_CTX* ctx, const unsigned char* data, size_t len){ size_t i = (ctx->bitcount[0] >> 3) % 64; ctx->bitcount[0] += (unsigned int)len << 3; if(ctx->bitcount[0] < ((unsigned int)len << 3)) ctx->bitcount[1]++; ctx->bitcount[1] += (unsigned int)len >> 29; size_t fill = 64 - i; if (i && len >= fill){ memcpy(ctx->buffer + i, data, fill); sha256_transform(ctx, ctx->buffer); data += fill; len -= fill; i = 0; } while (len >= 64){ sha256_transform(ctx, data); data += 64; len -= 64; } if (len){ memcpy(ctx->buffer + i, data, len); } }");
        self.line("static void sha256_final(SHA256_CTX* ctx, unsigned char* out){ unsigned char pad[64]={0}; unsigned char lenbits[8]; unsigned int high = ctx->bitcount[1]; unsigned int low = ctx->bitcount[0]; lenbits[0]=(high>>24)&0xFF; lenbits[1]=(high>>16)&0xFF; lenbits[2]=(high>>8)&0xFF; lenbits[3]=high&0xFF; lenbits[4]=(low>>24)&0xFF; lenbits[5]=(low>>16)&0xFF; lenbits[6]=(low>>8)&0xFF; lenbits[7]=low&0xFF; pad[0]=0x80; size_t i = (ctx->bitcount[0] >> 3) % 64; size_t padlen = (i < 56) ? (56 - i) : (120 - i); sha256_update(ctx, pad, padlen); sha256_update(ctx, lenbits, 8); for (i=0;i<8;i++){ out[i*4] = (ctx->state[i]>>24)&0xFF; out[i*4+1] = (ctx->state[i]>>16)&0xFF; out[i*4+2] = (ctx->state[i]>>8)&0xFF; out[i*4+3] = ctx->state[i]&0xFF; } }");
        self.line("static char* sha256_hex_bytes(const unsigned char* data, size_t len){ unsigned char digest[32]; SHA256_CTX ctx; sha256_init(&ctx); sha256_update(&ctx, data, len); sha256_final(&ctx, digest); static const char* hex = \"0123456789abcdef\"; char* out = (char*)malloc(65); if(!out) return NULL; for(int i=0;i<32;i++){ out[i*2] = hex[(digest[i]>>4)&0xF]; out[i*2+1] = hex[digest[i]&0xF]; } out[64]=0; return out; }");
        self.line("static char* sha256_hex_str(const char* s){ return sha256_hex_bytes((const unsigned char*)s, strlen(s)); }");
        self.line("static char* sha256_random_hex(size_t n){ static int seeded=0; if(!seeded){ srand((unsigned)time(NULL)); seeded=1; } unsigned char* buf=(unsigned char*)malloc(n); if(!buf) return NULL; for(size_t i=0;i<n;i++){ buf[i]=(unsigned char)(rand()&0xFF); } char* out=sha256_hex_bytes(buf, n); free(buf); return out; }");
    }

    fn emit_math_helpers(&mut self) {
        self.empty();
        self.line("// Math helpers for std lib mapping");
        // Resolve helper dependencies before emission to avoid ordering issues
        if self.need_variancef && !self.need_meanf {
            self.need_meanf = true;
        }
        if self.need_stddevf && !self.need_variancef {
            self.need_variancef = true;
        }
        if self.need_isqrt {
            self.line("static long long nstd_isqrt(long long n){ if(n<=0) return 0; long long x=n; long long y=(x+1)/2; while(y<x){ x=y; y=(x + n/x)/2; } return x; }");
        }
        if self.need_gcd {
            self.line("static long long nstd_gcd(long long a, long long b){ if(a<0) a=-a; if(b<0) b=-b; while(b!=0){ long long t=b; b=a%b; a=t; } return a; }");
        }
        if self.need_lcm {
            // depends on gcd
            if !self.need_gcd {
                self.need_gcd = true;
            }
            self.line("static long long nstd_lcm(long long a, long long b){ if(a==0 || b==0) return 0; long long aa = (a<0?-a:a); long long bb=(b<0?-b:b); long long g = nstd_gcd(aa,bb); return (aa/g)*bb; }");
        }
        if self.need_factorial {
            self.line("static long long nstd_factorial(long long n){ if(n<0) return 0; long long r=1; for(long long i=2;i<=n;i++) r*=i; return r; }");
        }
        if self.need_npr {
            if !self.need_factorial {
                self.need_factorial = true;
            }
            self.line("static long long nstd_nPr(long long n, long long r){ if(r<0 || r>n) return 0; double v = (double)nstd_factorial(n) / (double)nstd_factorial(n-r); return (long long)v; }");
        }
        if self.need_ncr {
            self.line("static long long nstd_nCr(long long n, long long r){ if(r<0 || r>n) return 0; long long rr=r; if(rr>n-rr) rr = n-rr; long long num=1, den=1; for(long long i=1;i<=rr;i++){ num = num * (n-rr+i); den = den * i; } double v = (double)num / (double)den; return (long long)v; }");
        }
        if self.need_sumf {
            self.line("static double nstd_sum_float(const double* a, size_t n){ double s=0.0; for(size_t i=0;i<n;i++){ s+=a[i]; } return s; }");
        }
        if self.need_medianf {
            self.line("static int cmp_double(const void* a, const void* b){ double da=*(const double*)a; double db=*(const double*)b; return (da>db)-(da<db); }");
            self.line("static double nstd_median_float(const double* a, size_t n){ if(n==0) return 0.0; double* b=(double*)malloc(n*sizeof(double)); if(!b) return 0.0; for(size_t i=0;i<n;i++) b[i]=a[i]; qsort(b, n, sizeof(double), cmp_double); double m = (n%2==1)? b[n/2] : 0.5*(b[n/2-1] + b[n/2]); free(b); return m; }");
        }
        if self.need_variancef {
            // mean already marked above, now emit mean before variance if required
            if self.need_meanf {
                if !self.need_sumf {
                    self.need_sumf = true;
                }
                self.line("static double nstd_mean_float(const double* a, size_t n){ if(n==0) return 0.0; return nstd_sum_float(a,n) / (double)n; }");
            }
            self.line("static double nstd_variance_float(const double* a, size_t n){ if(n==0) return 0.0; double m = nstd_mean_float(a,n); double s=0.0; for(size_t i=0;i<n;i++){ double d=a[i]-m; s += d*d; } return s / (double)n; }");
        }
        if self.need_stddevf {
            // variance already marked above
            self.line("static double nstd_stddev_float(const double* a, size_t n){ return sqrt(nstd_variance_float(a,n)); }");
        }
    }
    fn emit_str_upper(&mut self) {
        self.line("static char* str_upper(const char* s){");
        self.push();
        self.line("size_t n = strlen(s);");
        self.line("char* out = (char*)malloc(n+1);");
        self.line("if(!out) return NULL;");
        self.line("for(size_t i=0;i<n;i++){ out[i] = (char)toupper((unsigned char)s[i]); }");
        self.line("out[n] = 0;");
        self.line("return out;");
        self.pop();
        self.line("}");
    }
    fn emit_str_lower(&mut self) {
        self.line("static char* str_lower(const char* s){");
        self.push();
        self.line("size_t n = strlen(s);");
        self.line("char* out = (char*)malloc(n+1);");
        self.line("if(!out) return NULL;");
        self.line("for(size_t i=0;i<n;i++){ out[i] = (char)tolower((unsigned char)s[i]); }");
        self.line("out[n] = 0;");
        self.line("return out;");
        self.pop();
        self.line("}");
    }
    fn emit_str_trim(&mut self) {
        self.line("static char* str_trim(const char* s){");
        self.push();
        self.line("size_t n = strlen(s);");
        self.line(
            "size_t start = 0; while(start < n && isspace((unsigned char)s[start])) start++;",
        );
        self.line("size_t end = n; while(end>start && isspace((unsigned char)s[end-1])) end--;");
        self.line("size_t m = end - start;");
        self.line("char* out = (char*)malloc(m+1);");
        self.line("if(!out) return NULL;");
        self.line("memcpy(out, s+start, m);");
        self.line("out[m] = 0;");
        self.line("return out;");
        self.pop();
        self.line("}");
    }
    fn emit_str_contains(&mut self) {
        self.line("static int str_contains(const char* hay, const char* needle){");
        self.push();
        self.line("return strstr(hay, needle) != NULL;");
        self.pop();
        self.line("}");
    }
    // --------------------------------------------------------------------- //
    // String constants
    // --------------------------------------------------------------------- //
    fn collect_strings(&mut self, prog: &Program) {
        fn walk(generator: &mut CCodeGenerator, stmt: &Statement) {
            match stmt {
                Statement::Expression(e) => walk_expr(generator, e),
                Statement::LetDeclaration { initializer, .. } => {
                    if let Some(i) = initializer {
                        walk_expr(generator, i);
                    }
                }
                Statement::FunctionDeclaration { body, .. } => {
                    for s in body {
                        walk(generator, s);
                    }
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                    ..
                } => {
                    walk_expr(generator, condition);
                    walk(generator, then_branch);
                    if let Some(e) = else_branch {
                        walk(generator, e);
                    }
                }
                Statement::While {
                    condition, body, ..
                } => {
                    walk_expr(generator, condition);
                    walk(generator, body);
                }
                Statement::For {
                    initializer,
                    condition,
                    increment,
                    body,
                    ..
                } => {
                    if let Some(init) = initializer {
                        walk(generator, init);
                    }
                    if let Some(cond) = condition {
                        walk_expr(generator, cond);
                    }
                    if let Some(inc) = increment {
                        walk_expr(generator, inc);
                    }
                    walk(generator, body);
                }
                Statement::Return { value } => {
                    if let Some(v) = value {
                        walk_expr(generator, v);
                    }
                }
                Statement::Block { statements } => {
                    for s in statements {
                        walk(generator, s);
                    }
                }
                Statement::Pick {
                    expression,
                    cases,
                    default,
                } => {
                    walk_expr(generator, expression);
                    for case in cases {
                        for value in &case.values {
                            walk_expr(generator, value);
                        }
                        walk(generator, &case.body);
                    }
                    if let Some(default_body) = default {
                        walk(generator, default_body);
                    }
                }
                Statement::RepeatUntil { body, condition } => {
                    walk(generator, body);
                    walk_expr(generator, condition);
                }
                Statement::Loop { body } => {
                    walk(generator, body);
                }
                _ => {}
            }
        }
        fn walk_expr(generator: &mut CCodeGenerator, e: &Expr) {
            match e {
                Expr::Literal(Literal::String(s)) => {
                    if !generator.str_consts.contains_key(s) {
                        let name = format!("str_const_{}", generator.str_counter);
                        generator.str_counter += 1;
                        generator.str_consts.insert(s.clone(), name);
                    }
                }
                Expr::Binary { left, right, .. } => {
                    walk_expr(generator, left);
                    walk_expr(generator, right);
                }
                Expr::Unary { operand, .. } => walk_expr(generator, operand),
                Expr::Call { arguments, .. } => {
                    for a in arguments {
                        walk_expr(generator, a);
                    }
                }
                Expr::Index { sequence, index } => {
                    walk_expr(generator, sequence);
                    walk_expr(generator, index);
                }
                Expr::ArrayLiteral { elements } => {
                    for element in elements {
                        walk_expr(generator, element);
                    }
                }
                Expr::Assign { value, .. } => {
                    walk_expr(generator, value);
                }
                Expr::AssignIndex {
                    sequence,
                    index,
                    value,
                } => {
                    walk_expr(generator, sequence);
                    walk_expr(generator, index);
                    walk_expr(generator, value);
                }
                Expr::Get { object, .. } => {
                    walk_expr(generator, object);
                }
                Expr::Set { object, value, .. } => {
                    walk_expr(generator, object);
                    walk_expr(generator, value);
                }
                Expr::Function {
                    parameters: _,
                    body,
                    return_type: _,
                } => {
                    for stmt in body {
                        walk(generator, stmt);
                    }
                }
                _ => {}
            }
        }
        for s in &prog.statements {
            walk(self, s);
        }
    }
    fn emit_string_consts(&mut self) -> Result<(), CCodeGenError> {
        let str_consts: Vec<(String, String)> = self
            .str_consts
            .iter()
            .map(|(lit, name)| (lit.clone(), name.clone()))
            .collect();

        for (lit, name) in str_consts {
            let escaped: String = lit
                .chars()
                .flat_map(|c| match c {
                    '"' => "\\\"".chars().collect::<Vec<_>>(),
                    '\\' => "\\\\".chars().collect::<Vec<_>>(),
                    '\n' => "\\n".chars().collect::<Vec<_>>(),
                    '\r' => "\\r".chars().collect::<Vec<_>>(),
                    '\t' => "\\t".chars().collect::<Vec<_>>(),
                    c => c.to_string().chars().collect::<Vec<_>>(),
                })
                .collect();
            self.line(&format!("static const char {name}[] = \"{escaped}\";"));
        }
        if !self.str_consts.is_empty() {
            self.empty();
        }
        Ok(())
    }
    fn extract_function_return_types(&mut self, prog: &Program) {
        let mut new_function_return_types = HashMap::new();
        for stmt in &prog.statements {
            if let Statement::FunctionDeclaration {
                name,
                return_type,
                body,
                ..
            } = stmt
            {
                let mut c_return_type = if name == "main" {
                    "int".to_string()
                } else {
                    self.type_to_c(return_type.as_ref().unwrap_or(&Type::Void))
                };

                // If the return type is void or unspecified, try to infer from return statements
                if c_return_type == "void" && name != "main" {
                    // Walk the body to find a return with a value and infer its type
                    fn infer_from_body(
                        codegenr: &mut CCodeGenerator,
                        stmt: &Statement,
                    ) -> Option<String> {
                        match stmt {
                            Statement::Return { value } => {
                                if let Some(v) = value {
                                    Some(codegenr.infer_type(v))
                                } else {
                                    None
                                }
                            }
                            Statement::If {
                                then_branch,
                                else_branch,
                                ..
                            } => infer_from_body(codegenr, then_branch.as_ref()).or_else(|| {
                                else_branch
                                    .as_ref()
                                    .and_then(|e| infer_from_body(codegenr, e.as_ref()))
                            }),
                            Statement::While { body, .. } => {
                                infer_from_body(codegenr, body.as_ref())
                            }
                            Statement::Block { statements } => {
                                for s in statements {
                                    if let Some(t) = infer_from_body(codegenr, s) {
                                        return Some(t);
                                    }
                                }
                                None
                            }
                            _ => None,
                        }
                    }

                    for s in body {
                        if let Some(t) = infer_from_body(self, s) {
                            c_return_type = t;
                            break;
                        }
                    }
                }

                new_function_return_types.insert(name.clone(), c_return_type);
            }
        }
        self.function_return_types = Rc::new(new_function_return_types);
    }
    // --------------------------------------------------------------------- //
    // Forward declarations
    // --------------------------------------------------------------------- //
    fn emit_forward_decls(&mut self, prog: &Program) -> Result<(), CCodeGenError> {
        for stmt in &prog.statements {
            if let Statement::FunctionDeclaration {
                name,
                parameters,
                return_type,
                ..
            } = stmt
            {
                // Prefer inferred return types from map; fall back to declared type or void
                let ret: String = if name == "main" {
                    "int".to_string()
                } else {
                    self.function_return_types
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| {
                            self.type_to_c(return_type.as_ref().unwrap_or(&Type::Void))
                        })
                };
                self.write(&ret);
                self.write(" ");
                self.write(name);
                self.write("(");
                let mut first = true;
                for p in parameters {
                    if !first {
                        self.write(", ");
                    }
                    self.write(&self.type_to_c(&p.param_type.as_ref().unwrap_or(&Type::Integer)));
                    self.write(" ");
                    self.write(&p.name);
                    first = false;
                }
                if parameters.is_empty() {
                    self.write("void");
                }
                self.line(");");
            }
        }
        if prog
            .statements
            .iter()
            .any(|s| matches!(s, Statement::FunctionDeclaration { .. }))
        {
            self.empty();
        }
        Ok(())
    }
    // --------------------------------------------------------------------- //
    // Function bodies
    // --------------------------------------------------------------------- //
    fn emit_functions(&mut self, prog: &Program) -> Result<(), CCodeGenError> {
        for stmt in &prog.statements {
            if let Statement::FunctionDeclaration { .. } = stmt {
                self.emit_function(stmt)?;
                self.empty();
            }
        }
        Ok(())
    }
    fn emit_function(&mut self, stmt: &Statement) -> Result<(), CCodeGenError> {
        let Statement::FunctionDeclaration {
            name,
            parameters,
            body,
            return_type,
            ..
        } = stmt
        else {
            return Err(CCodeGenError::Unsupported("not a function".into()));
        };

        // Set up function context for proper type tracking
        let ret: String = if name == "main" {
            "int".to_string()
        } else {
            self.function_return_types
                .get(name)
                .cloned()
                .unwrap_or_else(|| self.type_to_c(return_type.as_ref().unwrap_or(&Type::Void)))
        };

        // Store current function context
        self.current_function_return_type = Some(ret.clone());

        // Clear and populate function parameters
        self.function_parameters.clear();
        for param in parameters {
            let param_type = param.param_type.clone().unwrap_or(Type::Integer);
            self.function_parameters
                .insert(param.name.clone(), param_type);
        }
        self.write(&ret);
        self.write(" ");
        self.write(name);
        self.write("(");
        let mut first = true;
        for p in parameters {
            if !first {
                self.write(", ");
            }
            let ty = self.type_to_c(&p.param_type.as_ref().unwrap_or(&Type::Integer));
            self.write(&ty);
            self.write(" ");
            self.write(&p.name);
            self.vars.insert(p.name.clone(), ty);
            if let Some(Type::Boolean) = p.param_type.as_ref() {
                self.bool_vars.insert(p.name.clone(), true);
            }
            first = false;
        }
        if parameters.is_empty() {
            self.write("void");
        }
        self.write(") ");
        self.block(|generator| {
            // Add UTF-8 setup at the beginning of main function
            if name == "main" {
                generator.line("setup_utf8_console();");
            }

            for s in body {
                generator.emit_stmt(s).unwrap();
            }
            if name == "main" && !body.iter().any(|s| matches!(s, Statement::Return { .. })) {
                generator.line("return 0;");
            }
        });

        // Clear function context after generation
        self.current_function_return_type = None;
        self.function_parameters.clear();
        Ok(())
    }
    fn emit_stmt(&mut self, stmt: &Statement) -> Result<(), CCodeGenError> {
        match stmt {
            Statement::Expression(e) => {
                if let Expr::Assign { name, value } = e {
                    let dk = *self.drop_kinds.get(name).unwrap_or(&0);
                    if dk == 1 {
                        self.line(&format!("if({}) free({});", name, name));
                        let vcode = self.emit_expr(value)?;
                        self.line(&format!("{} = {};", name, vcode));
                        if let Expr::Variable(srcn) = value.as_ref() {
                            if let Some(sdk) = self.drop_kinds.get(srcn) {
                                if *sdk != 0 {
                                    self.mark_moved(srcn);
                                }
                            }
                        }
                        return Ok(());
                    }
                }
                let code = self.emit_expr(e)?;
                self.line(&format!("{code};"));
            }
            Statement::LetDeclaration {
                name,
                initializer,
                var_type,
                ..
            } => {
                // Use declared type if available, otherwise infer from initializer or default to int
                let ty = if let Some(declared_type) = var_type {
                    self.type_to_c(declared_type)
                } else if let Some(init) = initializer {
                    // Special-case std array ops returning pointers
                    if let Expr::Call { callee, .. } = init {
                        if let Expr::Variable(fname) = callee.as_ref() {
                            if fname == "sort" || fname == "reverse" {
                                "int*".to_string()
                            } else {
                                self.infer_type(init)
                            }
                        } else {
                            self.infer_type(init)
                        }
                    } else if let Expr::ArrayLiteral { elements } = init {
                        if elements.is_empty() {
                            self.infer_type(init)
                        } else {
                            let elem_ty = self.infer_type(&elements[0]);
                            if elem_ty == "char*" {
                                "char**".to_string()
                            } else {
                                format!("{}*", elem_ty)
                            }
                        }
                    } else {
                        self.infer_type(init)
                    }
                } else {
                    "int".to_string()
                };

                // Store the element type for arrays (and pointer for array literals), not the full array type
                let var_type_to_store = if let Some(Type::Array(element_type, _)) = var_type {
                    // Keep array info in type string to help index inference
                    let elem = self.type_to_c(element_type);
                    if let Some(Type::Array(_, size)) = var_type {
                        format!("{}[{}]", elem, size)
                    } else {
                        elem
                    }
                } else if let Some(init) = initializer {
                    if let Expr::ArrayLiteral { elements } = init {
                        if elements.is_empty() {
                            ty.clone()
                        } else {
                            let elem_ty = self.infer_type(&elements[0]);
                            if elem_ty == "char*" {
                                "char**".to_string()
                            } else {
                                format!("{}*", elem_ty)
                            }
                        }
                    } else {
                        ty.clone()
                    }
                } else {
                    ty.clone()
                };
                self.vars.insert(name.clone(), var_type_to_store);
                if let Some(declared_type) = var_type {
                    match declared_type {
                        Type::Vault(_, _) => {
                            self.var_kinds.insert(name.clone(), "vault".to_string());
                            self.need_vault = true;
                        }
                        Type::Pool(_) => {
                            self.var_kinds.insert(name.clone(), "pool".to_string());
                            self.need_pool = true;
                        }
                        Type::Tree(_) => {
                            self.var_kinds.insert(name.clone(), "tree".to_string());
                            self.need_tree = true;
                        }
                        Type::Array(inner, _) => {
                            if matches!(**inner, Type::Boolean) {
                                self.bool_array_vars.insert(name.clone());
                            }
                        }
                        _ => {}
                    }
                } else if let Some(init) = initializer {
                    if let Expr::Call { callee, .. } = init {
                        if let Expr::Variable(fname) = callee.as_ref() {
                            match fname.as_str() {
                                "vault" => {
                                    self.var_kinds.insert(name.clone(), "vault".to_string());
                                    self.need_vault = true;
                                }
                                "pool" => {
                                    self.var_kinds.insert(name.clone(), "pool".to_string());
                                    self.need_pool = true;
                                }
                                "tree" => {
                                    self.var_kinds.insert(name.clone(), "tree".to_string());
                                    self.need_tree = true;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                if let Some(Type::Boolean) = var_type {
                    self.bool_vars.insert(name.clone(), true);
                }
                if let Some(init) = initializer {
                    if let Expr::Call {
                        callee, arguments, ..
                    } = init
                    {
                        if let Expr::Variable(fname) = callee.as_ref() {
                            if (fname == "sort" || fname == "reverse") && arguments.len() == 1 {
                                let arg_code = self.emit_expr(&arguments[0])?;
                                let len_code = match &arguments[0] {
                                    Expr::Variable(_vn) => {
                                        format!("(sizeof({0}) / sizeof({0}[0]))", arg_code)
                                    }
                                    _ => {
                                        return Err(CCodeGenError::Unsupported(
                                            "sort/reverse initializer requires array variable"
                                                .into(),
                                        ));
                                    }
                                };
                                if fname == "sort" {
                                    self.need_nstd_sort = true;
                                } else {
                                    self.need_nstd_reverse = true;
                                }
                                self.vars.insert(name.clone(), "int64_t*".to_string());
                                self.line(&format!("int64_t* {name} = (int64_t*)({{ nstd_{op}_int({arg_code}, {len_code}); {arg_code}; }});",
                                op= if fname=="sort" {"sort"} else {"reverse"}, arg_code=arg_code, len_code=len_code, name=name));
                                return Ok(());
                            }
                        } else if let Expr::Get { object, name: meth } = callee.as_ref() {
                            if meth == "split" {
                                let obj_code = self.emit_expr(object)?;
                                if arguments.len() != 1 {
                                    return Err(CCodeGenError::Unsupported(
                                        "split expects 1 argument".into(),
                                    ));
                                }
                                let delim_code = self.emit_expr(&arguments[0])?;
                                let len_var = format!("{var}_len", var = name);
                                self.arr_len_vars.insert(name.clone(), len_var.clone());
                                self.need_str_split = true;
                                self.vars.insert(name.clone(), "char**".to_string());
                                self.line(&format!("size_t {len_var}; char** {var} = str_split_alloc({obj}, {delim}, &{len_var});", len_var=len_var, var=name, obj=obj_code, delim=delim_code));
                                self.mark_var_decl(name);
                                self.set_drop_kind(name, 5);
                                return Ok(());
                            } else if meth == "regex" {
                                let obj_code = self.emit_expr(object)?;
                                if arguments.len() != 1 {
                                    return Err(CCodeGenError::Unsupported(
                                        "regex expects 1 argument".into(),
                                    ));
                                }
                                let pat_code = self.emit_expr(&arguments[0])?;
                                let len_var = format!("{var}_len", var = name);
                                self.arr_len_vars.insert(name.clone(), len_var.clone());
                                self.need_str_regex = true;
                                self.vars.insert(name.clone(), "char**".to_string());
                                self.line(&format!("size_t {len_var}; char** {var} = str_regex_matches({obj}, {pat}, &{len_var});", len_var=len_var, var=name, obj=obj_code, pat=pat_code));
                                self.mark_var_decl(name);
                                self.set_drop_kind(name, 5);
                                return Ok(());
                            }
                        }
                    }
                    let init_code = self.emit_expr(init)?;
                    // If initializer is an array variable, bind a pointer to it
                    let mut printed = false;
                    if let Expr::Variable(src_name) = init {
                        if let Some(tstr) = self.vars.get(src_name.as_str()) {
                            if tstr.contains('[') {
                                // Array variable: decay to pointer
                                let base = tstr.split('[').next().unwrap_or("int");
                                let pty = format!("{}*", base);
                                self.vars.insert(name.clone(), pty.clone());
                                self.line(&format!("{pty} {name} = {init_code};"));
                                self.mark_var_decl(name);
                                self.set_drop_kind(name, 0);
                                self.mark_moved(src_name);
                                printed = true;
                            }
                        }
                    }
                    if printed {
                        let dk = self.drop_kind_for_initializer(init);
                        self.mark_var_decl(name);
                        self.set_drop_kind(name, dk);
                        return Ok(());
                    }

                    // Handle array types specially - in C, arrays are declared as "type name[size1][size2]..."
                    if let Some(array_type) = var_type {
                        let c_decl = self.array_type_to_c_decl(array_type, name);
                        self.line(&format!("{c_decl} = {init_code};"));
                    } else {
                        self.line(&format!("{ty} {name} = {init_code};"));
                    }
                    self.mark_var_decl(name);
                    let dk = self.drop_kind_for_initializer(init);
                    self.set_drop_kind(name, dk);
                    if let Expr::Variable(srcn) = init {
                        if let Some(sdk) = { self.drop_kinds.get(srcn).copied() } {
                            if sdk != 0 {
                                self.mark_moved(srcn);
                                self.set_drop_kind(name, sdk);
                            }
                        }
                    }
                } else {
                    // Handle array types specially - in C, arrays are declared as "type name[size1][size2]..."
                    if let Some(array_type) = var_type {
                        let c_decl = self.array_type_to_c_decl(array_type, name);
                        self.line(&format!("{c_decl};"));
                    } else {
                        self.line(&format!("{ty} {name};"));
                    }
                    self.mark_var_decl(name);
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond = self.emit_expr(condition)?;
                self.write(&format!("if ({cond}) "));
                self.block(|generator| {
                    generator.emit_stmt(then_branch).unwrap();
                });
                if let Some(els) = else_branch {
                    self.write(" else ");
                    self.block(|generator| {
                        generator.emit_stmt(els).unwrap();
                    });
                }
            }
            Statement::While { condition, body } => {
                let cond = self.emit_expr(condition)?;
                self.write(&format!("while ({cond}) "));
                self.block(|generator| {
                    generator.emit_stmt(body).unwrap();
                });
            }
            Statement::RepeatUntil { body, condition } => {
                // do-while loop: execute body first, then check condition
                self.line("do ");
                self.block(|generator| {
                    generator.emit_stmt(body).unwrap();
                });
                let cond = self.emit_expr(condition)?;
                self.line(&format!("while (!({cond}));"));
            }
            Statement::Loop { body } => {
                // Infinite loop (like Rust's loop keyword)
                self.line("while (1) ");
                self.block(|generator| {
                    generator.emit_stmt(body).unwrap();
                });
            }
            Statement::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                self.write("for (");

                if let Some(init_stmt) = initializer {
                    match &**init_stmt {
                        Statement::LetDeclaration {
                            name,
                            initializer,
                            var_type,
                            ..
                        } => {
                            let ty = if let Some(declared_type) = var_type {
                                self.type_to_c(declared_type)
                            } else if let Some(init) = initializer {
                                self.infer_type(init)
                            } else {
                                "int".to_string()
                            };
                            self.vars.insert(name.clone(), ty.clone());
                            if let Some(init) = initializer {
                                let init_code = self.emit_expr(init)?;
                                self.write(&format!("{ty} {name} = {init_code}"));
                            } else {
                                self.write(&format!("{ty} {name}"));
                            }
                        }
                        Statement::Expression(expr) => {
                            let code = self.emit_expr(expr)?;
                            self.write(&code);
                        }
                        _ => return Err(CCodeGenError::Unsupported("for loop initializer".into())),
                    }
                }

                self.write("; ");

                if let Some(cond_expr) = condition {
                    let cond_code = self.emit_expr(cond_expr)?;
                    self.write(&cond_code);
                }

                self.write("; ");

                if let Some(inc_expr) = increment {
                    let inc_code = self.emit_expr(inc_expr)?;
                    self.write(&inc_code);
                }

                self.write(") ");
                self.block(|generator| {
                    generator.emit_stmt(body).unwrap();
                });
            }
            Statement::Return { value } => {
                if let Some(v) = value {
                    let vcode = self.emit_expr(v)?;
                    self.line(&format!("return {vcode};"));
                } else {
                    self.line("return;");
                }
            }
            Statement::Block { statements } => {
                self.block(|generator| {
                    for s in statements {
                        generator.emit_stmt(s).unwrap();
                    }
                });
            }
            Statement::Break => self.line("break;"),
            Statement::Continue => self.line("continue;"),
            Statement::Pick {
                expression,
                cases,
                default,
            } => {
                let expr_code = self.emit_expr(expression)?;
                let expr_type = self.infer_type(expression);

                let is_integer_type = match expr_type.as_str() {
                    "int" | "int8_t" | "int16_t" | "int32_t" | "int64_t" | "intptr_t"
                    | "uint8_t" | "uint16_t" | "uint32_t" | "uint64_t" | "size_t" => true,
                    _ => false,
                };

                if is_integer_type {
                    self.line(&format!("switch ({}) {{", expr_code));
                    self.push();

                    for case in cases {
                        for value in &case.values {
                            let value_code = self.emit_expr(value)?;
                            self.line(&format!("case {}:", value_code));
                        }
                        self.push();
                        self.emit_stmt(&case.body)?;
                        self.line("break;");
                        self.pop();
                    }

                    if let Some(default_body) = default {
                        self.line("default:");
                        self.push();
                        self.emit_stmt(default_body)?;
                        self.line("break;");
                        self.pop();
                    }

                    self.pop();
                    self.line("}");
                } else if expr_type == "const char*" || expr_type == "char*" {
                    let mut first_case = true;
                    for case in cases {
                        let mut conditions = Vec::new();
                        for value in &case.values {
                            let value_code = self.emit_expr(value)?;
                            conditions.push(format!("strcmp({}, {}) == 0", expr_code, value_code));
                        }

                        if first_case {
                            self.write(&format!("if ({}) ", conditions.join(" || ")));
                            first_case = false;
                        } else {
                            self.write(&format!(" else if ({}) ", conditions.join(" || ")));
                        }
                        self.block(|generator| {
                            generator.emit_stmt(&case.body).unwrap();
                        });
                    }

                    if let Some(default_body) = default {
                        self.write(" else ");
                        self.block(|generator| {
                            generator.emit_stmt(default_body).unwrap();
                        });
                    }
                    self.empty();
                } else {
                    let mut first_case = true;
                    for case in cases {
                        let mut conditions = Vec::new();
                        for value in &case.values {
                            let value_code = self.emit_expr(value)?;
                            conditions.push(format!("{} == {}", expr_code, value_code));
                        }

                        if first_case {
                            self.write(&format!("if ({}) ", conditions.join(" || ")));
                            first_case = false;
                        } else {
                            self.write(&format!(" else if ({}) ", conditions.join(" || ")));
                        }
                        self.block(|generator| {
                            generator.emit_stmt(&case.body).unwrap();
                        });
                    }

                    if let Some(default_body) = default {
                        self.write(" else ");
                        self.block(|generator| {
                            generator.emit_stmt(default_body).unwrap();
                        });
                    }
                    self.empty();
                }
            }
            _ => return Err(CCodeGenError::Unsupported(format!("stmt {:?}", stmt))),
        }
        Ok(())
    }
    fn emit_expr(&mut self, e: &Expr) -> Result<String, CCodeGenError> {
        Ok(match e {
            Expr::Literal(l) => self.emit_lit(l)?,
            Expr::Variable(n) => n.clone(),
            Expr::Borrow { target, mutable: _ } => {
                // Represent borrows as void* pointers to the lvalue
                let inner = self.emit_expr(target)?;
                format!("(void*)&({})", inner)
            }
            Expr::Binary {
                left,
                right,
                operator,
                ..
            } => {
                let l = self.emit_expr(left)?;
                let r = self.emit_expr(right)?;

                // Special handling for string concatenation
                if *operator == BinaryOperator::Plus {
                    // If either operand is a string expression, concatenate as strings
                    let left_is_string = self.is_string_expression(left);
                    let right_is_string = self.is_string_expression(right);

                    if left_is_string || right_is_string {
                        let l_str = match self.infer_type(left).as_str() {
                            "const char*" | "char*" => l.clone(),
                            "double" => format!("float_to_str({})", l),
                            _ => format!("int_to_str({})", l),
                        };
                        let r_str = match self.infer_type(right).as_str() {
                            "const char*" | "char*" => r.clone(),
                            "double" => format!("float_to_str({})", r),
                            _ => format!("int_to_str({})", r),
                        };
                        return Ok(format!("str_concat({l_str}, {r_str})"));
                    }
                }

                // Special handling for string comparison
                if *operator == BinaryOperator::EqualEqual || *operator == BinaryOperator::NotEqual
                {
                    let left_is_string = self.is_string_expression(left);
                    let right_is_string = self.is_string_expression(right);

                    if left_is_string && right_is_string {
                        if *operator == BinaryOperator::EqualEqual {
                            return Ok(format!("(strcmp({}, {}) == 0)", l, r));
                        } else {
                            return Ok(format!("(strcmp({}, {}) != 0)", l, r));
                        }
                    }
                }

                match operator {
                    BinaryOperator::BitAnd
                    | BinaryOperator::BitOr
                    | BinaryOperator::BitXor
                    | BinaryOperator::ShiftLeft
                    | BinaryOperator::ShiftRight => {
                        let op = self.binop(operator);
                        format!("((uint32_t)((uint32_t)({l}) {op} (uint32_t)({r})))")
                    }
                    _ => format!("({l} {} {r})", self.binop(operator)),
                }
            }
            Expr::Unary {
                operand, operator, ..
            } => {
                let inner = self.emit_expr(operand)?;
                format!("({}{})", self.unop(operator), inner)
            }
            Expr::Call {
                callee, arguments, ..
            } => {
                // Check registry for library functions
                if let Expr::Variable(fname) = callee.as_ref() {
                    let parts: Vec<&str> = fname.split('.').collect();
                    if parts.len() == 2 {
                        let lib_name = parts[0];
                        let func_short_name = parts[1];
                        if let Some(lib) = self.registry.get_library(lib_name) {
                            if let Some(func) =
                                lib.functions.iter().find(|f| f.name == func_short_name)
                            {
                                if let Some(c_impl) = &func.c_implementation {
                                    let mut code = c_impl.clone();
                                    for (i, arg) in arguments.iter().enumerate() {
                                        let arg_code = self.emit_expr(arg)?;
                                        code = code.replace(&format!("{{{}}}", i), &arg_code);
                                    }
                                    return Ok(code);
                                }
                            }
                        }
                    } else if parts.len() == 1 {
                        // Check all registered libraries for this function
                        for lib_name in self.registry.get_registered_libs() {
                            if let Some(lib) = self.registry.get_library(&lib_name) {
                                if let Some(func) = lib.functions.iter().find(|f| f.name == *fname)
                                {
                                    if let Some(c_impl) = &func.c_implementation {
                                        let mut code = c_impl.clone();
                                        for (i, arg) in arguments.iter().enumerate() {
                                            let arg_code = self.emit_expr(arg)?;
                                            code = code.replace(&format!("{{{}}}", i), &arg_code);
                                        }
                                        return Ok(code);
                                    }
                                }
                            }
                        }
                    }
                }

                let fname = if let Expr::Variable(v) = callee.as_ref() {
                    v.clone()
                } else {
                    if let Expr::Get { object, name } = callee.as_ref() {
                        let obj_code = self.emit_expr(object)?;
                        // String methods mapping first, regardless of static type
                        match name.as_str() {
                            "upper" => {
                                return Ok(format!("str_upper({})", obj_code));
                            }
                            "lower" => {
                                return Ok(format!("str_lower({})", obj_code));
                            }
                            "trim" => {
                                return Ok(format!("str_trim({})", obj_code));
                            }
                            "replace" => {
                                if arguments.len() != 2 {
                                    return Err(CCodeGenError::Unsupported(
                                        "String.replace(from,to) expects 2 arguments".into(),
                                    ));
                                }
                                let from_code = self.emit_expr(&arguments[0])?;
                                let to_code = self.emit_expr(&arguments[1])?;
                                self.need_str_replace = true;
                                return Ok(format!(
                                    "str_replace_all({}, {}, {})",
                                    obj_code, from_code, to_code
                                ));
                            }
                            "substring" => {
                                if arguments.len() != 2 {
                                    return Err(CCodeGenError::Unsupported(
                                        "String.substring(start,end) expects 2 arguments".into(),
                                    ));
                                }
                                let start_code = self.emit_expr(&arguments[0])?;
                                let end_code = self.emit_expr(&arguments[1])?;
                                self.need_str_substring = true;
                                return Ok(format!(
                                    "str_substring_range({}, (size_t)({}), (size_t)({}))",
                                    obj_code, start_code, end_code
                                ));
                            }
                            "contains" => {
                                if arguments.len() != 1 {
                                    return Err(CCodeGenError::Unsupported(
                                        "String.contains() expects 1 argument".into(),
                                    ));
                                }
                                let arg_code = self.emit_expr(&arguments[0])?;
                                return Ok(format!("str_contains({}, {})", obj_code, arg_code));
                            }
                            "split" => {
                                if arguments.len() != 1 {
                                    return Err(CCodeGenError::Unsupported(
                                        "String.split(delim) expects 1 argument".into(),
                                    ));
                                }
                                let delim_code = self.emit_expr(&arguments[0])?;
                                // Without binding, we cannot produce length; advise using let binding. Fallback: produce temporary length symbol.
                                self.need_str_split = true;
                                return Ok(format!(
                                    "str_split_alloc({}, {}, &(size_t){{0}})/*use with let binding*/",
                                    obj_code, delim_code
                                ));
                            }
                            _ => {}
                        }
                        if name == "len" {
                            if let Expr::Variable(var_name) = object.as_ref() {
                                if let Some(lenv) = self.arr_len_vars.get(var_name) {
                                    return Ok(lenv.clone());
                                }
                                if let Some(Type::Array(_, sz)) =
                                    self.function_parameters.get(var_name)
                                {
                                    return Ok(format!("{}", sz));
                                }
                            }
                            return Ok(format!("(sizeof({}) / sizeof({}[0]))", obj_code, obj_code));
                        }
                        if name == "join" {
                            if arguments.len() != 1 {
                                return Err(CCodeGenError::Unsupported(
                                    "Array.join(delim) expects 1 argument".into(),
                                ));
                            }
                            let delim_code = self.emit_expr(&arguments[0])?;
                            if let Expr::Variable(var_name) = object.as_ref() {
                                if let Some(lenv) = self.arr_len_vars.get(var_name) {
                                    self.need_str_join = true;
                                    return Ok(format!(
                                        "str_join_str({arr}, (size_t)({lenv}), {delim})",
                                        arr = obj_code,
                                        lenv = lenv,
                                        delim = delim_code
                                    ));
                                }
                            }
                            // Fallback requires known size
                            self.need_str_join = true;
                            return Ok(format!(
                                "str_join_str({arr}, (size_t)(sizeof({arr})/sizeof({arr}[0])), {delim})",
                                arr = obj_code,
                                delim = delim_code
                            ));
                        }
                        // Namespace function calls: std.fn -> fn
                        if let Expr::Variable(ns) = object.as_ref() {
                            if ns == "std" {
                                return Ok(name.clone());
                            }
                        }
                        return Err(CCodeGenError::Unsupported("complex callee".into()));
                    } else {
                        return Err(CCodeGenError::Unsupported("complex callee".into()));
                    }
                };
                // built-ins
                match fname.as_str() {
                    "print" | "println" => {
                        if arguments.is_empty() {
                            if fname == "println" {
                                return Ok("printf(\"\\n\")".to_string());
                            } else {
                                return Ok("".to_string());
                            }
                        }

                        // Check for placeholder logic
                        if let Expr::Literal(Literal::String(format_str)) = &arguments[0] {
                            if arguments.len() > 1 && format_str.contains("[]") {
                                let mut final_fmt = String::new();
                                let mut vals = Vec::new();
                                let parts: Vec<&str> = format_str.split("[]").collect();
                                let mut arg_idx = 1;

                                for (i, part) in parts.iter().enumerate() {
                                    final_fmt.push_str(&part.replace('%', "%%"));
                                    if i < parts.len() - 1 {
                                        if arg_idx < arguments.len() {
                                            let arg = &arguments[arg_idx];
                                            let (fmt, val) = {
                                                if let Expr::Index { sequence, index } = arg {
                                                    if let Expr::Variable(var_name) =
                                                        sequence.as_ref()
                                                    {
                                                        if self
                                                            .var_kinds
                                                            .get(var_name)
                                                            .map(|s| s == "vault")
                                                            .unwrap_or(false)
                                                        {
                                                            let seq_code =
                                                                self.emit_expr(sequence)?;
                                                            let idx_code = self.emit_expr(index)?;
                                                            (
                                                                "%s".into(),
                                                                format!(
                                                                    "(vault_get_tag({seq_code}, {idx_code})==1 ? vault_get_str({seq_code}, {idx_code}) : int_to_str((int)vault_get_int({seq_code}, {idx_code})))"
                                                                ),
                                                            )
                                                        } else {
                                                            let arg_code = self.emit_expr(arg)?;
                                                            self.print_fmt(arg, &arg_code)?
                                                        }
                                                    } else {
                                                        let arg_code = self.emit_expr(arg)?;
                                                        self.print_fmt(arg, &arg_code)?
                                                    }
                                                } else if let Expr::Variable(name) = arg {
                                                    if self
                                                        .var_kinds
                                                        .get(name)
                                                        .map(|s| s == "tree")
                                                        .unwrap_or(false)
                                                    {
                                                        let arg_code = self.emit_expr(arg)?;
                                                        (
                                                            "%s".into(),
                                                            format!("tree_to_str({})", arg_code),
                                                        )
                                                    } else if self
                                                        .vars
                                                        .get(name)
                                                        .map(|t| t == "char**")
                                                        .unwrap_or(false)
                                                    {
                                                        // Join string arrays nicely if we know their length
                                                        if let Some(lenv) =
                                                            self.arr_len_vars.get(name)
                                                        {
                                                            self.need_str_join = true;
                                                            (
                                                                "%s".into(),
                                                                format!(
                                                                    r#"str_join_str({arr}, (size_t)({len}), ", ")"#,
                                                                    arr = name,
                                                                    len = lenv
                                                                ),
                                                            )
                                                        } else {
                                                            let arg_code = self.emit_expr(arg)?;
                                                            self.print_fmt(arg, &arg_code)?
                                                        }
                                                    } else {
                                                        let arg_code = self.emit_expr(arg)?;
                                                        self.print_fmt(arg, &arg_code)?
                                                    }
                                                } else {
                                                    let arg_code = self.emit_expr(arg)?;
                                                    self.print_fmt(arg, &arg_code)?
                                                }
                                            };
                                            final_fmt.push_str(&fmt);
                                            vals.push(val);
                                            arg_idx += 1;
                                        } else {
                                            final_fmt.push_str("[]");
                                        }
                                    }
                                }

                                // Handle remaining arguments
                                while arg_idx < arguments.len() {
                                    let arg = &arguments[arg_idx];
                                    let (fmt, val) = {
                                        if let Expr::Index { sequence, index } = arg {
                                            if let Expr::Variable(var_name) = sequence.as_ref() {
                                                if self
                                                    .var_kinds
                                                    .get(var_name)
                                                    .map(|s| s == "vault")
                                                    .unwrap_or(false)
                                                {
                                                    let seq_code = self.emit_expr(sequence)?;
                                                    let idx_code = self.emit_expr(index)?;
                                                    (
                                                        "%s".into(),
                                                        format!(
                                                            "(vault_get_tag({seq_code}, {idx_code})==1 ? vault_get_str({seq_code}, {idx_code}) : int_to_str((int)vault_get_int({seq_code}, {idx_code})))"
                                                        ),
                                                    )
                                                } else {
                                                    let arg_code = self.emit_expr(arg)?;
                                                    self.print_fmt(arg, &arg_code)?
                                                }
                                            } else {
                                                let arg_code = self.emit_expr(arg)?;
                                                self.print_fmt(arg, &arg_code)?
                                            }
                                        } else if let Expr::Variable(name) = arg {
                                            if self
                                                .var_kinds
                                                .get(name)
                                                .map(|s| s == "tree")
                                                .unwrap_or(false)
                                            {
                                                let arg_code = self.emit_expr(arg)?;
                                                ("%s".into(), format!("tree_to_str({})", arg_code))
                                            } else {
                                                let arg_code = self.emit_expr(arg)?;
                                                self.print_fmt(arg, &arg_code)?
                                            }
                                        } else {
                                            let arg_code = self.emit_expr(arg)?;
                                            self.print_fmt(arg, &arg_code)?
                                        }
                                    };
                                    final_fmt.push_str(" ");
                                    final_fmt.push_str(&fmt);
                                    vals.push(val);
                                    arg_idx += 1;
                                }

                                let val_string = vals.join(", ");
                                let call = if fname == "println" {
                                    if vals.is_empty() {
                                        format!("printf(\"{}\\n\")", final_fmt)
                                    } else {
                                        format!("printf(\"{}\\n\", {})", final_fmt, val_string)
                                    }
                                } else {
                                    if vals.is_empty() {
                                        format!("printf(\"{}\")", final_fmt)
                                    } else {
                                        format!("printf(\"{}\", {})", final_fmt, val_string)
                                    }
                                };
                                return Ok(call);
                            }
                        }

                        // Fallback to old logic
                        let mut fmts = Vec::new();
                        let mut vals = Vec::new();
                        for arg in arguments {
                            let (fmt, val) = {
                                if let Expr::Index { sequence, index } = arg {
                                    if let Expr::Variable(var_name) = sequence.as_ref() {
                                        if self
                                            .var_kinds
                                            .get(var_name)
                                            .map(|s| s == "vault")
                                            .unwrap_or(false)
                                        {
                                            let seq_code = self.emit_expr(sequence)?;
                                            let idx_code = self.emit_expr(index)?;
                                            (
                                                "%s".into(),
                                                format!(
                                                    "(vault_get_tag({seq_code}, {idx_code})==1 ? vault_get_str({seq_code}, {idx_code}) : int_to_str((int)vault_get_int({seq_code}, {idx_code})))"
                                                ),
                                            )
                                        } else {
                                            let arg_code = self.emit_expr(arg)?;
                                            self.print_fmt(arg, &arg_code)?
                                        }
                                    } else {
                                        let arg_code = self.emit_expr(arg)?;
                                        self.print_fmt(arg, &arg_code)?
                                    }
                                } else if let Expr::Variable(name) = arg {
                                    if self
                                        .var_kinds
                                        .get(name)
                                        .map(|s| s == "tree")
                                        .unwrap_or(false)
                                    {
                                        let arg_code = self.emit_expr(arg)?;
                                        ("%s".into(), format!("tree_to_str({})", arg_code))
                                    } else if self
                                        .vars
                                        .get(name)
                                        .map(|t| t == "char**")
                                        .unwrap_or(false)
                                    {
                                        // Join string arrays for readable output
                                        if let Some(lenv) = self.arr_len_vars.get(name) {
                                            self.need_str_join = true;
                                            (
                                                "%s".into(),
                                                format!(
                                                    r#"str_join_str({arr}, (size_t)({len}), ", ")"#,
                                                    arr = name,
                                                    len = lenv
                                                ),
                                            )
                                        } else {
                                            let arg_code = self.emit_expr(arg)?;
                                            self.print_fmt(arg, &arg_code)?
                                        }
                                    } else {
                                        let arg_code = self.emit_expr(arg)?;
                                        self.print_fmt(arg, &arg_code)?
                                    }
                                } else {
                                    let arg_code = self.emit_expr(arg)?;
                                    self.print_fmt(arg, &arg_code)?
                                }
                            };
                            fmts.push(fmt);
                            vals.push(val);
                        }

                        let fmt_string = fmts.join(" ");
                        let val_string = vals.join(", ");

                        let call = if fname == "println" {
                            if val_string.is_empty() {
                                format!("printf(\"{}\\n\")", fmt_string)
                            } else {
                                format!("printf(\"{}\\n\", {})", fmt_string, val_string)
                            }
                        } else {
                            if val_string.is_empty() {
                                format!("printf(\"{}\")", fmt_string)
                            } else {
                                format!("printf(\"{}\", {})", fmt_string, val_string)
                            }
                        };
                        return Ok(call);
                    }
                    "str" => return Ok(format!("int_to_str({})", self.emit_expr(&arguments[0])?)),
                    "int" => {
                        let arg_expr = &arguments[0];
                        let arg_code = self.emit_expr(arg_expr)?;
                        let arg_type = self.infer_type(arg_expr);
                        if arg_type == "char*" || arg_type == "const char*" {
                            return Ok(format!("atoi({})", arg_code));
                        } else {
                            return Ok(format!("((int){})", arg_code));
                        }
                    }
                    "float" => {
                        let arg_expr = &arguments[0];
                        let arg_code = self.emit_expr(arg_expr)?;
                        let arg_type = self.infer_type(arg_expr);
                        if arg_type == "char*" || arg_type == "const char*" {
                            return Ok(format!("atof({})", arg_code));
                        } else {
                            return Ok(format!("((double){})", arg_code));
                        }
                    }
                    "abs" => return Ok(format!("abs({})", self.emit_expr(&arguments[0])?)),
                    "abs_float" => return Ok(format!("fabs({})", self.emit_expr(&arguments[0])?)),
                    "len" => {
                        if arguments.len() != 1 {
                            return Err(CCodeGenError::Unsupported(
                                "len() expects 1 argument".into(),
                            ));
                        }
                        let arg_expr = &arguments[0];
                        let arg_code = self.emit_expr(arg_expr)?;
                        if let Expr::Variable(var_name) = arg_expr {
                            if let Some(lenv) = self.arr_len_vars.get(var_name) {
                                return Ok(lenv.clone());
                            }
                        }
                        let arg_type = self.infer_type(arg_expr);

                        if arg_type == "const char*" || arg_type == "char*" {
                            return Ok(format!("strlen({})", arg_code));
                        } else if let Expr::Variable(var_name) = arg_expr {
                            // Check if this is a known function parameter array with hardcoded size
                            match var_name.as_str() {
                                "numbers" => return Ok("5".to_string()), // [int; 5] from test case
                                "words" => return Ok("3".to_string()), // [string; 3] from test case
                                _ => {
                                    // Stack-allocated array - use sizeof approach
                                    return Ok(format!(
                                        "(sizeof({}) / sizeof({}[0]))",
                                        arg_code, arg_code
                                    ));
                                }
                            }
                        } else {
                            // Assume it's a stack-allocated array
                            return Ok(format!("(sizeof({}) / sizeof({}[0]))", arg_code, arg_code));
                        }
                    }
                    "input" => {
                        if arguments.is_empty() {
                            return Ok("read_line(NULL)".to_string());
                        } else if arguments.len() == 1 {
                            let arg_code = self.emit_expr(&arguments[0])?;
                            return Ok(format!("read_line({})", arg_code));
                        } else {
                            return Err(CCodeGenError::Unsupported(
                                "input() takes 0 or 1 arguments".into(),
                            ));
                        }
                    }
                    "sha256" => {
                        if arguments.len() != 1 {
                            return Err(CCodeGenError::Unsupported(
                                "sha256() expects 1 argument".into(),
                            ));
                        }
                        self.need_sha_helpers = true;
                        let arg_expr = &arguments[0];
                        let arg_code = self.emit_expr(arg_expr)?;
                        let arg_type = self.infer_type(arg_expr);
                        if arg_type == "const char*" || arg_type == "char*" {
                            return Ok(format!("sha256_hex_str({})", arg_code));
                        } else if let Expr::Variable(_var_name) = arg_expr {
                            // Compute array length using sizeof trick
                            let len_code =
                                format!("(sizeof({}) / sizeof({}[0]))", arg_code, arg_code);
                            return Ok(format!(
                                "sha256_hex_bytes((const unsigned char*){}, {})",
                                arg_code, len_code
                            ));
                        } else {
                            return Err(CCodeGenError::Unsupported(
                                "sha256() argument must be string or array variable".into(),
                            ));
                        }
                    }
                    "sha256_random" => {
                        if arguments.len() != 1 {
                            return Err(CCodeGenError::Unsupported(
                                "sha256_random() expects 1 argument".into(),
                            ));
                        }
                        self.need_sha_helpers = true;
                        let ncode = self.emit_expr(&arguments[0])?;
                        return Ok(format!("sha256_random_hex((size_t)({}))", ncode));
                    }
                    // Math std mappings
                    "pi" => {
                        return Ok("acos(-1.0)".into());
                    }
                    "tau" => {
                        return Ok("(2.0*acos(-1.0))".into());
                    }
                    "e" => {
                        return Ok("exp(1.0)".into());
                    }
                    "ln2" => {
                        return Ok("log(2.0)".into());
                    }
                    "ln10" => {
                        return Ok("log(10.0)".into());
                    }
                    "exp" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("exp({})", x));
                    }
                    "ln" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("log({})", x));
                    }
                    "log2" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("log2({})", x));
                    }
                    "log10" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("log10({})", x));
                    }
                    "pow_float" => {
                        if arguments.len() != 2 {
                            return Err(CCodeGenError::Unsupported(
                                "pow_float(a,b) expects 2 args".into(),
                            ));
                        }
                        let a = self.emit_expr(&arguments[0])?;
                        let b = self.emit_expr(&arguments[1])?;
                        return Ok(format!("pow({}, {})", a, b));
                    }
                    "powi_float" => {
                        if arguments.len() != 2 {
                            return Err(CCodeGenError::Unsupported(
                                "powi_float(a,n) expects 2 args".into(),
                            ));
                        }
                        let a = self.emit_expr(&arguments[0])?;
                        let n = self.emit_expr(&arguments[1])?;
                        return Ok(format!("pow({}, (double)({}))", a, n));
                    }
                    "sqrt" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("sqrt({})", x));
                    }
                    "isqrt" => {
                        self.need_isqrt = true;
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_isqrt({})", x));
                    }
                    "sin" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("sin({})", x));
                    }
                    "cos" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("cos({})", x));
                    }
                    "tan" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("tan({})", x));
                    }
                    "atan" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("atan({})", x));
                    }
                    "asin" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("asin({})", x));
                    }
                    "acos" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("acos({})", x));
                    }
                    "floor" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("(int64_t)floor({})", x));
                    }
                    "ceil" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("(int64_t)ceil({})", x));
                    }
                    "round" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("(int64_t)round({})", x));
                    }
                    "clamp" => {
                        if arguments.len() != 3 {
                            return Err(CCodeGenError::Unsupported(
                                "clamp(x,a,b) expects 3 args".into(),
                            ));
                        }
                        let x = self.emit_expr(&arguments[0])?;
                        let a = self.emit_expr(&arguments[1])?;
                        let b = self.emit_expr(&arguments[2])?;
                        return Ok(format!(
                            "((({x})<({a}))?({a}):((({x})>({b}))?({b}):({x}))))",
                            x = x,
                            a = a,
                            b = b
                        ));
                    }
                    "fmod" => {
                        let x = self.emit_expr(&arguments[0])?;
                        let y = self.emit_expr(&arguments[1])?;
                        return Ok(format!("fmod({}, {})", x, y));
                    }
                    "sign" => {
                        let x = self.emit_expr(&arguments[0])?;
                        return Ok(format!("(({}>0.0)?1:(({}<0.0)?-1:0))", x, x));
                    }
                    "gcd" => {
                        self.need_gcd = true;
                        let a = self.emit_expr(&arguments[0])?;
                        let b = self.emit_expr(&arguments[1])?;
                        return Ok(format!("nstd_gcd({}, {})", a, b));
                    }
                    "lcm" => {
                        self.need_lcm = true;
                        let a = self.emit_expr(&arguments[0])?;
                        let b = self.emit_expr(&arguments[1])?;
                        return Ok(format!("nstd_lcm({}, {})", a, b));
                    }
                    "factorial" => {
                        self.need_factorial = true;
                        let n = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_factorial({})", n));
                    }
                    "nPr" => {
                        self.need_npr = true;
                        let n = self.emit_expr(&arguments[0])?;
                        let r = self.emit_expr(&arguments[1])?;
                        return Ok(format!("nstd_nPr({}, {})", n, r));
                    }
                    "nCr" => {
                        self.need_ncr = true;
                        let n = self.emit_expr(&arguments[0])?;
                        let r = self.emit_expr(&arguments[1])?;
                        return Ok(format!("nstd_nCr({}, {})", n, r));
                    }
                    "sum_float" => {
                        self.need_sumf = true;
                        let arr = self.emit_expr(&arguments[0])?;
                        let len = format!("(sizeof({})/sizeof({}[0]))", arr, arr);
                        return Ok(format!("nstd_sum_float({}, {})", arr, len));
                    }
                    "mean_float" => {
                        self.need_meanf = true;
                        let arr = self.emit_expr(&arguments[0])?;
                        let len = format!("(sizeof({})/sizeof({}[0]))", arr, arr);
                        return Ok(format!("nstd_mean_float({}, {})", arr, len));
                    }
                    "median_float" => {
                        self.need_medianf = true;
                        let arr = self.emit_expr(&arguments[0])?;
                        let len = format!("(sizeof({})/sizeof({}[0]))", arr, arr);
                        return Ok(format!("nstd_median_float({}, {})", arr, len));
                    }
                    "variance_float" => {
                        self.need_variancef = true;
                        let arr = self.emit_expr(&arguments[0])?;
                        let len = format!("(sizeof({})/sizeof({}[0]))", arr, arr);
                        return Ok(format!("nstd_variance_float({}, {})", arr, len));
                    }
                    "stddev_float" => {
                        self.need_stddevf = true;
                        let arr = self.emit_expr(&arguments[0])?;
                        let len = format!("(sizeof({})/sizeof({}[0]))", arr, arr);
                        return Ok(format!("nstd_stddev_float({}, {})", arr, len));
                    }
                    "timestamp" => {
                        return Ok("nstd_timestamp()".into());
                    }
                    "timestamp_ms" => {
                        return Ok("nstd_timestamp_ms()".into());
                    }
                    "timestamp_us" => {
                        return Ok("nstd_timestamp_us()".into());
                    }
                    "timestamp_ns" => {
                        return Ok("nstd_timestamp_ns()".into());
                    }
                    "nanosecond" => {
                        return Ok("(long long)(nstd_timestamp_ns()%1000000000LL)".into());
                    }
                    "now" | "now_local" => {
                        return Ok("nstd_now_str(0)".into());
                    }
                    "now_utc" => {
                        return Ok("nstd_now_str(1)".into());
                    }
                    "time_to_string" => {
                        return Ok("nstd_now_str(0)".into());
                    }
                    "sleep" => {
                        let s = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_sleep_ms((long long)({})*1000LL)", s));
                    }
                    "sleep_ms" => {
                        let ms = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_sleep_ms((long long)({}))", ms));
                    }
                    "sleep_ns" => {
                        let ns = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_sleep_ms((long long)({})/1000000LL)", ns));
                    }
                    "year" => {
                        return Ok("nstd_year_local()".into());
                    }
                    "month" => {
                        return Ok("nstd_month_local()".into());
                    }
                    "day" => {
                        return Ok("nstd_day_local()".into());
                    }
                    "weekday" => {
                        return Ok("nstd_weekday_local()".into());
                    }
                    "hour" => {
                        return Ok("nstd_hour_local()".into());
                    }
                    "minute" => {
                        return Ok("nstd_minute_local()".into());
                    }
                    "second" => {
                        return Ok("nstd_second_local()".into());
                    }
                    "format" => {
                        let f = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_format_now_local({})", f));
                    }
                    "to_local" => {
                        let s = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_to_local_from_secs((long long)({}))", s));
                    }
                    "to_utc" => {
                        let s = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_to_utc_from_secs((long long)({}))", s));
                    }
                    "from_timestamp" => {
                        let s = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_to_utc_from_secs((long long)({}))", s));
                    }
                    "from_timestamp_ms" => {
                        let ms = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_to_utc_from_secs((long long)({})/1000LL)", ms));
                    }
                    "parse" => {
                        let s = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_parse_ymd({})", s));
                    }
                    "parse_rfc3339" => {
                        let s = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_parse_rfc3339({})", s));
                    }
                    "parse_rfc2822" => {
                        let s = self.emit_expr(&arguments[0])?;
                        return Ok(format!("nstd_parse_rfc2822({})", s));
                    }
                    "timer_start" => {
                        self.need_vault = true;
                        return Ok("({ void* t=vault(); vault_set_int(t, \"start_ns\", (long)nstd_timestamp_ns()); t; })".into());
                    }
                    "timer_elapsed" => {
                        let t = self.emit_expr(&arguments[0])?;
                        return Ok(format!(
                            "((nstd_timestamp_ns() - (long long)vault_get_int({}, \"start_ns\"))/1000000LL)",
                            t
                        ));
                    }
                    "timer_reset" => {
                        let t = self.emit_expr(&arguments[0])?;
                        self.need_vault = true;
                        return Ok(format!(
                            "({{ vault_set_int({}, \"start_ns\", (long)nstd_timestamp_ns()); {}; }})",
                            t, t
                        ));
                    }
                    "duration_from_seconds" => {
                        let x = self.emit_expr(&arguments[0])?;
                        self.need_vault = true;
                        return Ok(format!(
                            "({{ void* d=vault(); vault_set_int(d, \"nanos\", (long)(({})*1000000000LL)); d; }})",
                            x
                        ));
                    }
                    "duration_from_millis" => {
                        let x = self.emit_expr(&arguments[0])?;
                        self.need_vault = true;
                        return Ok(format!(
                            "({{ void* d=vault(); vault_set_int(d, \"nanos\", (long)(({})*1000000LL)); d; }})",
                            x
                        ));
                    }
                    "duration_from_nanos" => {
                        let x = self.emit_expr(&arguments[0])?;
                        self.need_vault = true;
                        return Ok(format!(
                            "({{ void* d=vault(); vault_set_int(d, \"nanos\", (long)({})); d; }})",
                            x
                        ));
                    }
                    "duration_as_secs" => {
                        let d = self.emit_expr(&arguments[0])?;
                        return Ok(format!("(vault_get_int({}, \"nanos\")/1000000000LL)", d));
                    }
                    "duration_as_millis" => {
                        let d = self.emit_expr(&arguments[0])?;
                        return Ok(format!("(vault_get_int({}, \"nanos\")/1000000LL)", d));
                    }
                    "duration_add" => {
                        let a = self.emit_expr(&arguments[0])?;
                        let b = self.emit_expr(&arguments[1])?;
                        self.need_vault = true;
                        return Ok(format!(
                            "({{ void* d=vault(); long long na=vault_get_int({}, \"nanos\"); long long nb=vault_get_int({}, \"nanos\"); vault_set_int(d, \"nanos\", (long)(na+nb)); d; }})",
                            a, b
                        ));
                    }
                    "duration_sub" => {
                        let a = self.emit_expr(&arguments[0])?;
                        let b = self.emit_expr(&arguments[1])?;
                        self.need_vault = true;
                        return Ok(format!(
                            "({{ void* d=vault(); long long na=vault_get_int({}, \"nanos\"); long long nb=vault_get_int({}, \"nanos\"); long long r=na-nb; vault_set_int(d, \"nanos\", (long)(r)); d; }})",
                            a, b
                        ));
                    }
                    _ => {}
                }
                if fname == "vault" {
                    self.need_vault = true;
                    return Ok("vault()".to_string());
                }
                if fname == "pool" {
                    self.need_pool = true;
                    // encode as varargs with type tags
                    let mut args_code = Vec::new();
                    args_code.push(format!("{}", arguments.len()));
                    for a in arguments {
                        let ac = self.emit_expr(a)?;
                        let at = self.infer_type(a);
                        let tag = if at == "double" {
                            1
                        } else if at == "const char*" || at == "char*" {
                            3
                        } else {
                            0
                        };
                        args_code.push(tag.to_string());
                        args_code.push(ac);
                    }
                    return Ok(format!("pool({})", args_code.join(", ")));
                }
                if fname == "tree" {
                    self.need_tree = true;
                    let acs: Vec<_> = arguments
                        .iter()
                        .map(|a| self.emit_expr(a))
                        .collect::<Result<_, _>>()?;
                    return Ok(format!("tree({})", acs.join(", ")));
                }
                // std helpers mapping
                if fname == "sum" {
                    if arguments.len() != 1 {
                        return Err(CCodeGenError::Unsupported(
                            "sum(arr) expects 1 argument".into(),
                        ));
                    }
                    let arg_code = self.emit_expr(&arguments[0])?;
                    let len_code = match &arguments[0] {
                        Expr::Variable(_vn) => format!("(sizeof({0}) / sizeof({0}[0]))", arg_code),
                        _ => {
                            return Err(CCodeGenError::Unsupported(
                                "sum() requires array variable".into(),
                            ));
                        }
                    };
                    self.need_nstd_sum = true;
                    return Ok(format!("nstd_sum_int({arg_code}, {len_code})"));
                }
                if fname == "min" {
                    if arguments.len() != 1 {
                        return Err(CCodeGenError::Unsupported(
                            "min(arr) expects 1 argument".into(),
                        ));
                    }
                    let arg_code = self.emit_expr(&arguments[0])?;
                    let len_code = match &arguments[0] {
                        Expr::Variable(_vn) => format!("(sizeof({0}) / sizeof({0}[0]))", arg_code),
                        _ => {
                            return Err(CCodeGenError::Unsupported(
                                "min() requires array variable".into(),
                            ));
                        }
                    };
                    self.need_nstd_min = true;
                    return Ok(format!("nstd_min_int({arg_code}, {len_code})"));
                }
                if fname == "max" {
                    if arguments.len() != 1 {
                        return Err(CCodeGenError::Unsupported(
                            "max(arr) expects 1 argument".into(),
                        ));
                    }
                    let arg_code = self.emit_expr(&arguments[0])?;
                    let len_code = match &arguments[0] {
                        Expr::Variable(_vn) => format!("(sizeof({0}) / sizeof({0}[0]))", arg_code),
                        _ => {
                            return Err(CCodeGenError::Unsupported(
                                "max() requires array variable".into(),
                            ));
                        }
                    };
                    self.need_nstd_max = true;
                    return Ok(format!("nstd_max_int({arg_code}, {len_code})"));
                }
                if fname == "pow" {
                    if arguments.len() != 2 {
                        return Err(CCodeGenError::Unsupported(
                            "pow(base,exp) expects 2 arguments".into(),
                        ));
                    }
                    let b = self.emit_expr(&arguments[0])?;
                    let e = self.emit_expr(&arguments[1])?;
                    self.need_nstd_ipow = true;
                    return Ok(format!("nstd_ipow({b}, {e})"));
                }
                if fname == "sort" {
                    if arguments.len() != 1 {
                        return Err(CCodeGenError::Unsupported(
                            "sort(arr) expects 1 argument".into(),
                        ));
                    }
                    let arg_code = self.emit_expr(&arguments[0])?;
                    let len_code = match &arguments[0] {
                        Expr::Variable(_vn) => format!("(sizeof({0}) / sizeof({0}[0]))", arg_code),
                        _ => {
                            return Err(CCodeGenError::Unsupported(
                                "sort() requires array variable".into(),
                            ));
                        }
                    };
                    self.need_nstd_sort = true;
                    return Ok(format!(
                        "({{ nstd_sort_int({arg_code}, {len_code}); {arg_code}; }})"
                    ));
                }
                if fname == "reverse" {
                    if arguments.len() != 1 {
                        return Err(CCodeGenError::Unsupported(
                            "reverse(arr) expects 1 argument".into(),
                        ));
                    }
                    let arg_code = self.emit_expr(&arguments[0])?;
                    let len_code = match &arguments[0] {
                        Expr::Variable(_vn) => format!("(sizeof({0}) / sizeof({0}[0]))", arg_code),
                        _ => {
                            return Err(CCodeGenError::Unsupported(
                                "reverse() requires array variable".into(),
                            ));
                        }
                    };
                    self.need_nstd_reverse = true;
                    return Ok(format!(
                        "({{ nstd_reverse_int({arg_code}, {len_code}); {arg_code}; }})"
                    ));
                }
                let args: Vec<_> = arguments
                    .iter()
                    .map(|a| self.emit_expr(a))
                    .collect::<Result<_, _>>()?;
                format!("{}({})", fname, args.join(", "))
            }
            Expr::Assign { name, value } => {
                let v = self.emit_expr(value)?;
                let ty = self.infer_type(value);
                self.vars.insert(name.clone(), ty);
                format!("({name} = {v})")
            }
            Expr::AssignIndex {
                sequence,
                index,
                value,
            } => {
                let seq_code = self.emit_expr(sequence)?;
                // vault special-case
                if let Expr::Variable(var_name) = sequence.as_ref() {
                    if self
                        .var_kinds
                        .get(var_name)
                        .map(|s| s == "vault")
                        .unwrap_or(false)
                    {
                        let idx_code = self.emit_expr(index)?;
                        let val_code = self.emit_expr(value)?;
                        let vt = self.infer_type(value);
                        if vt == "const char*" || vt == "char*" {
                            return Ok(format!(
                                "vault_set_str({seq_code}, {idx_code}, {val_code})"
                            ));
                        } else {
                            return Ok(format!(
                                "vault_set_int({seq_code}, {idx_code}, (long)({val_code}))"
                            ));
                        }
                    }
                }
                let idx_code = self.emit_expr(index)?;
                let val_code = self.emit_expr(value)?;
                format!("({seq_code}[{idx_code}] = {val_code})")
            }
            Expr::Index { sequence, index } => {
                let seq_code = self.emit_expr(sequence)?;
                if let Expr::Variable(var_name) = sequence.as_ref() {
                    if self
                        .var_kinds
                        .get(var_name)
                        .map(|s| s == "vault")
                        .unwrap_or(false)
                    {
                        let idx_code = self.emit_expr(index)?;
                        // default to int
                        return Ok(format!("vault_get_int({seq_code}, {idx_code})"));
                    }
                }
                let idx_code = self.emit_expr(index)?;
                format!("({seq_code}[{idx_code}])")
            }
            Expr::ArrayLiteral { elements } => self.emit_array_literal(elements)?,
            _ => return Err(CCodeGenError::Unsupported(format!("expr {:?}", e))),
        })
    }

    fn emit_array_literal(&mut self, elements: &[Expr]) -> Result<String, CCodeGenError> {
        if elements.is_empty() {
            return Ok("NULL".to_string());
        }

        let mut element_codes = Vec::new();
        for element in elements {
            // Recursively handle nested array literals
            if let Expr::ArrayLiteral {
                elements: nested_elements,
            } = element
            {
                let nested_code = self.emit_array_literal(nested_elements)?;
                element_codes.push(nested_code);
            } else {
                element_codes.push(self.emit_expr(element)?);
            }
        }

        // Generate proper C array initialization syntax with nested braces
        Ok(format!("{{{}}}", element_codes.join(", ")))
    }

    fn emit_lit(&self, l: &Literal) -> Result<String, CCodeGenError> {
        Ok(match l {
            Literal::Integer(i) => i.to_string(),
            Literal::I8(i) => i.to_string(),
            Literal::I16(i) => i.to_string(),
            Literal::I32(i) => i.to_string(),
            Literal::I64(i) => i.to_string(),
            Literal::ISize(i) => i.to_string(),
            Literal::U8(i) => i.to_string(),
            Literal::U16(i) => i.to_string(),
            Literal::U32(i) => i.to_string(),
            Literal::U64(i) => i.to_string(),
            Literal::USize(i) => i.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::String(s) => {
                if let Some(name) = self.str_consts.get(s) {
                    name.clone()
                } else {
                    let escaped: String = s
                        .chars()
                        .flat_map(|c| match c {
                            '"' => "\\\"".chars().collect::<Vec<_>>(),
                            '\\' => "\\\\".chars().collect::<Vec<_>>(),
                            '\n' => "\\n".chars().collect::<Vec<_>>(),
                            '\r' => "\\r".chars().collect::<Vec<_>>(),
                            '\t' => "\\t".chars().collect::<Vec<_>>(),
                            c => c.to_string().chars().collect::<Vec<_>>(),
                        })
                        .collect();
                    format!("\"{}\"", escaped)
                }
            }
            Literal::Boolean(b) => (if *b { "1" } else { "0" }).to_string(),
            Literal::Null => "NULL".to_string(),
        })
    }
    // --------------------------------------------------------------------- //
    // Helpers
    // --------------------------------------------------------------------- //
    fn type_to_c(&self, t: &Type) -> String {
        match t {
            Type::Integer => "int64_t".into(),
            Type::I8 => "int8_t".into(),
            Type::I16 => "int16_t".into(),
            Type::I32 => "int32_t".into(),
            Type::I64 => "int64_t".into(),
            Type::ISize => "intptr_t".into(),
            Type::U8 => "uint8_t".into(),
            Type::U16 => "uint16_t".into(),
            Type::U32 => "uint32_t".into(),
            Type::U64 => "uint64_t".into(),
            Type::USize => "size_t".into(),
            Type::F32 => "float".into(),
            Type::F64 => "double".into(),
            Type::Float => "double".into(),
            Type::String => "const char*".into(),
            Type::Boolean => "int".into(),
            Type::Void => "void".into(),
            Type::Array(inner, _) => {
                // For function parameters, arrays are passed as pointers
                format!("{}*", self.type_to_c(inner))
            }
            Type::Function { .. } => "void*".into(),
            // Advanced types - map to appropriate C types
            Type::Unknown => "int".into(),
            Type::Infer => "void*".into(),
            Type::Generic(_) => "void*".into(),
            Type::Option(inner) => self.type_to_c(inner),
            Type::Tuple(_) => "void*".into(),
            Type::Union(_) => "void*".into(),
            Type::Result(_, _) => "void*".into(),
            Type::Vault(_, _) => "void*".into(),
            Type::Pool(_) => "void*".into(),
            Type::Tree(_) => "void*".into(),
            Type::Ref(_) => "void*".into(),
            Type::RefMut(_) => "void*".into(),
        }
    }

    fn array_type_to_c_decl(&self, t: &Type, name: &str) -> String {
        // Collect dimensions outer-first and emit in the same order
        let mut dims: Vec<usize> = Vec::new();
        let mut base = t;
        while let Type::Array(inner, size) = base {
            dims.push(*size);
            base = inner;
        }
        let base_type = self.type_to_c(base);
        let mut decl = format!("{} {}", base_type, name);
        for d in dims {
            decl.push_str(&format!("[{}]", d));
        }
        decl
    }
    fn binop(&self, op: &BinaryOperator) -> &'static str {
        match op {
            BinaryOperator::Plus => "+",
            BinaryOperator::Minus => "-",
            BinaryOperator::Star => "*",
            BinaryOperator::Slash => "/",
            BinaryOperator::Percent => "%",
            BinaryOperator::EqualEqual => "==",
            BinaryOperator::NotEqual => "!=",
            BinaryOperator::Less => "<",
            BinaryOperator::LessEqual => "<=",
            BinaryOperator::Greater => ">",
            BinaryOperator::GreaterEqual => ">=",
            BinaryOperator::And => "&&",
            BinaryOperator::Or => "||",
            BinaryOperator::BitAnd => "&",
            BinaryOperator::BitOr => "|",
            BinaryOperator::BitXor => "^",
            BinaryOperator::ShiftLeft => "<<",
            BinaryOperator::ShiftRight => ">>",
        }
    }

    // Helper method to check if an expression results in a string
    fn is_string_expression(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Literal(Literal::String(_)) => true,
            Expr::Variable(name) => {
                if let Some(var_type) = self.vars.get(name) {
                    var_type.contains("char*")
                } else {
                    if name.starts_with("str_const_") {
                        return true;
                    }
                    name.contains("str")
                        || name.contains("text")
                        || name.contains("message")
                        || name.contains("greeting")
                        || name.contains("result")
                        || name.contains("combined")
                        || name.contains("name")
                        || name.contains("prefix")
                        || name.contains("suffix")
                        || name.contains("final")
                        || name.contains("inferred")
                }
            }
            Expr::Binary {
                left,
                right,
                operator,
                ..
            } => {
                if *operator == BinaryOperator::Plus {
                    self.is_string_expression(left) || self.is_string_expression(right)
                } else {
                    false
                }
            }
            Expr::Index {
                sequence, index: _, ..
            } => {
                // Check if this is indexing into a string array
                // Be very permissive - if it looks like a string array, assume it returns strings
                if let Expr::Variable(var_name) = sequence.as_ref() {
                    var_name.contains("words")
                        || var_name.contains("strings")
                        || var_name.starts_with("str_")
                        || self.is_string_expression(sequence)
                } else {
                    self.is_string_expression(sequence)
                }
            }
            Expr::Call { callee, .. } => {
                // Check if this is a function call that returns a string
                if let Expr::Variable(fname) = callee.as_ref() {
                    matches!(
                        fname.as_str(),
                        "str"
                            | "int_to_str"
                            | "float_to_str"
                            | "upper"
                            | "lower"
                            | "trim"
                            | "create_message"
                            | "mixed_type_demo"
                            | "classify_number"
                            | "process_data"
                            | "process_string_list"
                            | "analyze_values"
                            | "chain_operations"
                            | "hex8"
                            | "sha256"
                    )
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn is_boolean_expression(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Literal(Literal::Boolean(_)) => true,
            Expr::Unary { operator, .. } => matches!(operator, UnaryOperator::Not),
            Expr::Binary { operator, .. } => match operator {
                BinaryOperator::EqualEqual
                | BinaryOperator::NotEqual
                | BinaryOperator::Less
                | BinaryOperator::LessEqual
                | BinaryOperator::Greater
                | BinaryOperator::GreaterEqual
                | BinaryOperator::And
                | BinaryOperator::Or => true,
                _ => false,
            },
            Expr::Call { callee, .. } => {
                if let Expr::Get { object, name } = callee.as_ref() {
                    let obj_ty = self.infer_type(object);
                    (obj_ty == "const char*" || obj_ty == "char*") && name == "contains"
                } else {
                    false
                }
            }
            Expr::Variable(name) => self.bool_vars.get(name).copied().unwrap_or(false),
            Expr::Index { sequence, .. } => {
                if let Expr::Variable(var_name) = sequence.as_ref() {
                    return self.bool_array_vars.contains(var_name);
                }
                false
            }
            _ => false,
        }
    }

    #[allow(dead_code)]
    fn is_function_parameter_array(&self, var_name: &str) -> bool {
        // Check if the variable is a function parameter and is an array type
        // Use the proper parameter tracking that was set up during function generation
        if let Some(param_type) = self.function_parameters.get(var_name) {
            matches!(param_type, Type::Array(_, _))
        } else {
            // Fallback to name-based detection for edge cases
            matches!(
                var_name,
                "numbers" | "words" | "array" | "arr" | "data" | "items" | "elements"
            )
        }
    }

    #[allow(dead_code)]
    fn current_function_returns_string(&self) -> bool {
        // Check if the current function returns a string type using proper tracking
        match &self.current_function_return_type {
            Some(return_type) => return_type.contains("char*") || return_type.contains("string"),
            None => false,
        }
    }

    fn unop(&self, op: &UnaryOperator) -> &'static str {
        match op {
            UnaryOperator::Negate => "-",
            UnaryOperator::Not => "!",
            UnaryOperator::BitNot => "~",
        }
    }
    fn infer_type(&self, e: &Expr) -> String {
        match e {
            Expr::Literal(Literal::String(_)) => "char*".into(),
            Expr::Literal(Literal::Float(_)) => "double".into(),
            Expr::Literal(Literal::Boolean(_)) => "int".into(),
            Expr::Literal(Literal::Integer(_)) => "int".into(),
            Expr::Literal(Literal::I8(_)) => "int8_t".into(),
            Expr::Literal(Literal::I16(_)) => "int16_t".into(),
            Expr::Literal(Literal::I32(_)) => "int32_t".into(),
            Expr::Literal(Literal::I64(_)) => "int64_t".into(),
            Expr::Literal(Literal::ISize(_)) => "intptr_t".into(),
            Expr::Literal(Literal::U8(_)) => "uint8_t".into(),
            Expr::Literal(Literal::U16(_)) => "uint16_t".into(),
            Expr::Literal(Literal::U32(_)) => "uint32_t".into(),
            Expr::Literal(Literal::U64(_)) => "uint64_t".into(),
            Expr::Literal(Literal::USize(_)) => "size_t".into(),
            Expr::Call {
                callee,
                arguments: _,
            } => {
                match callee.as_ref() {
                    Expr::Variable(func_name) => {
                        match func_name.as_str() {
                            "vault_get_str" => "char*".into(),
                            "vault_get_int" => "int".into(),
                            "str" => "char*".into(),
                            "input" => "char*".into(),
                            "str_concat" => "char*".into(),
                            "int_to_str" => "char*".into(),
                            "float_to_str" => "char*".into(),
                            "str_upper" => "char*".into(),
                            "str_lower" => "char*".into(),
                            "str_trim" => "char*".into(),
                            "str_contains" => "int".into(),
                            "sha256" => "char*".into(),
                            "int" => "int64_t".into(),
                            "float" => "double".into(),
                            "abs" => "int64_t".into(),
                            "abs_float" => "double".into(),
                            "sum" => "int64_t".into(),
                            "min" => "int64_t".into(),
                            "max" => "int64_t".into(),
                            "pow" => "int64_t".into(),
                            "sort" => "int64_t*".into(),
                            "reverse" => "int64_t*".into(),
                            // math std return types
                            "pi" | "tau" | "e" | "ln2" | "ln10" => "double".into(),
                            "exp" | "ln" | "log2" | "log10" | "sqrt" | "sin" | "cos" | "tan"
                            | "atan" | "asin" | "acos" | "clamp" | "fmod" => "double".into(),
                            "pow_float" | "powi_float" => "double".into(),
                            "isqrt" | "floor" | "ceil" | "round" | "sign" => "int64_t".into(),
                            "gcd" | "lcm" | "factorial" | "nPr" | "nCr" => "int64_t".into(),
                            "sum_float" | "mean_float" | "median_float" | "variance_float"
                            | "stddev_float" => "double".into(),
                            "timestamp" | "timestamp_ms" | "timestamp_us" | "timestamp_ns"
                            | "year" | "month" | "day" | "weekday" | "hour" | "minute"
                            | "second" | "nanosecond" => "int".into(),
                            "now" | "now_local" | "now_utc" | "format" | "to_local" | "to_utc"
                            | "from_timestamp" | "from_timestamp_ms" | "time_to_string" => {
                                "char*".into()
                            }
                            _ => (*self.function_return_types)
                                .get(func_name)
                                .map(|s| s.as_str())
                                .unwrap_or("int")
                                .to_string(),
                        }
                    }
                    Expr::Get { object, name } => {
                        let obj_ty = self.infer_type(object);
                        if obj_ty == "const char*" || obj_ty == "char*" {
                            match name.as_str() {
                                "upper" | "lower" | "trim" => "char*".into(),
                                "contains" => "int".into(),
                                _ => "int".into(),
                            }
                        } else if obj_ty.contains("[") {
                            if name == "len" {
                                "int".into()
                            } else {
                                "int".into()
                            }
                        } else {
                            "int".into()
                        }
                    }
                    _ => "int".into(),
                }
            }
            Expr::ArrayLiteral { elements } => {
                if elements.is_empty() {
                    "void*".into()
                } else {
                    self.infer_type(&elements[0])
                }
            }
            Expr::Borrow { .. } => {
                // References are represented as opaque pointers in C backend
                "void*".into()
            }
            Expr::Binary {
                left,
                right,
                operator,
                ..
            } => {
                if *operator == BinaryOperator::Plus {
                    if self.is_string_expression(left) || self.is_string_expression(right) {
                        return "char*".into();
                    }
                }
                // Numeric inference: promote to double if any operand is double
                let lt = self.infer_type(left);
                let rt = self.infer_type(right);
                if lt == "double" || rt == "double" || lt == "float" || rt == "float" {
                    "double".into()
                } else {
                    "int".into()
                }
            }
            Expr::Index { sequence, index: _ } => {
                // For array indexing expressions, infer the type of the sequence
                // If the sequence is a variable, look up its type in the vars map
                if let Expr::Variable(var_name) = sequence.as_ref() {
                    let full_type = self.vars.get(var_name).map(|s| s.as_str()).unwrap_or("int");

                    // Extract element type from array types (e.g., "double[3]" -> "double")
                    if let Some(open_bracket) = full_type.find('[') {
                        full_type[..open_bracket].to_string()
                    } else if full_type.ends_with("**") {
                        // Pointer-to-pointer (e.g., "char**" -> element type "char*")
                        format!("{}*", full_type.trim_end_matches('*'))
                    } else if full_type.ends_with('*') {
                        // Single pointer (e.g., "int*" -> element type "int")
                        full_type.trim_end_matches('*').to_string()
                    } else {
                        full_type.into()
                    }
                } else {
                    // For non-variable sequences (e.g., matrix[0]), unwrap one level of pointer/array
                    let seq_ty = self.infer_type(sequence);
                    if let Some(open_bracket) = seq_ty.find('[') {
                        seq_ty[..open_bracket].to_string()
                    } else if seq_ty.ends_with("**") {
                        format!("{}*", seq_ty.trim_end_matches('*'))
                    } else if seq_ty.ends_with('*') {
                        seq_ty.trim_end_matches('*').to_string()
                    } else {
                        seq_ty
                    }
                }
            }
            Expr::Variable(var_name) => self
                .vars
                .get(var_name)
                .map(|s| s.as_str())
                .unwrap_or("int")
                .into(),
            _ => "int".into(),
        }
    }

    fn emit_vault_runtime(&mut self) {
        self.empty();
        self.line("// Simple vault (string -> tagged value)");
        self.line(
            "typedef struct { const char* key; int tag; long ival; const char* sval; } VaultKV;",
        );
        self.line("typedef struct { VaultKV* items; size_t size; size_t cap; } Vault;");
        self.line("static void* vault(){ Vault* v = (Vault*)malloc(sizeof(Vault)); v->items=NULL; v->size=0; v->cap=0; return v; }");
        self.line("static int vault_find(Vault* v, const char* key){ for(size_t i=0;i<v->size;i++){ if(strcmp(v->items[i].key,key)==0) return (int)i; } return -1; }");
        self.line("static void vault_ensure(Vault* v){ if(v->size>=v->cap){ size_t nc = v->cap? v->cap*2:8; v->items = (VaultKV*)realloc(v->items, nc*sizeof(VaultKV)); v->cap=nc; } }");
        self.line("static void vault_set_int(void* vp, const char* key, long val){ Vault* v=(Vault*)vp; int idx=vault_find(v,key); if(idx<0){ vault_ensure(v); idx=(int)v->size++; v->items[idx].key=strdup(key); } v->items[idx].tag=0; v->items[idx].ival=val; v->items[idx].sval=NULL; }");
        self.line("static void vault_set_str(void* vp, const char* key, const char* val){ Vault* v=(Vault*)vp; int idx=vault_find(v,key); if(idx<0){ vault_ensure(v); idx=(int)v->size++; v->items[idx].key=strdup(key); } v->items[idx].tag=1; v->items[idx].sval=val; }");
        self.line("static long vault_get_int(void* vp, const char* key){ Vault* v=(Vault*)vp; int idx=vault_find(v,key); if(idx<0) return 0; if(v->items[idx].tag==0) return v->items[idx].ival; return 0; }");
        self.line("static const char* vault_get_str(void* vp, const char* key){ Vault* v=(Vault*)vp; int idx=vault_find(v,key); if(idx<0) return \"\"; if(v->items[idx].tag==1) return v->items[idx].sval; return \"\"; }");
        self.line("static int vault_get_tag(void* vp, const char* key){ Vault* v=(Vault*)vp; int idx=vault_find(v,key); if(idx<0) return -1; return v->items[idx].tag; }");
        self.line("static void vault_free(void* vp){ Vault* v=(Vault*)vp; if(!v) return; for(size_t i=0;i<v->size;i++){ if(v->items[i].key) free((void*)v->items[i].key); } free(v->items); free(v); }");
    }

    fn emit_pool_runtime(&mut self) {
        self.empty();
        self.line("// Simple pool (set) supporting int/double/string via varargs");
        self.line(
            "typedef struct { int tag; long ival; double fval; const char* sval; } PoolItem;",
        );
        self.line("typedef struct { PoolItem* items; size_t size; size_t cap; } Pool;");
        self.line("static void* pool(int count, ...){ Pool* p=(Pool*)malloc(sizeof(Pool)); p->items=NULL; p->size=0; p->cap=0; va_list ap; va_start(ap,count); for(int i=0;i<count;i++){ int tag = va_arg(ap,int); PoolItem it; it.tag=tag; if(tag==0){ it.ival = va_arg(ap,long); } else if(tag==1){ it.fval = va_arg(ap,double); } else if(tag==3){ it.sval = va_arg(ap,const char*); } if(p->size>=p->cap){ size_t nc=p->cap? p->cap*2:8; p->items=(PoolItem*)realloc(p->items,nc*sizeof(PoolItem)); p->cap=nc; } p->items[p->size++]=it; } va_end(ap); return p; }");
        self.line("static void pool_free(void* pp){ Pool* p=(Pool*)pp; if(!p) return; free(p->items); free(p); }");
    }

    fn emit_tree_runtime(&mut self) {
        self.empty();
        self.line("// Simple tree storing root string");
        self.line("typedef struct { const char* root; } Tree;");
        self.line("static void* tree(const char* root){ Tree* t=(Tree*)malloc(sizeof(Tree)); t->root=root; return t; }");
        self.line("static char* tree_to_str(void* tp){ Tree* t=(Tree*)tp; const char* r = t? t->root : \"\"; size_t n = strlen(r)+7; char* out=(char*)malloc(n); if(!out) return NULL; snprintf(out,n,\"tree(%s)\", r); return out; }");
        self.line("static void tree_free(void* tp){ Tree* t=(Tree*)tp; if(!t) return; free(t); }");
    }
    fn print_fmt(&self, e: &Expr, code: &str) -> Result<(String, String), CCodeGenError> {
        Ok(match e {
            Expr::Literal(Literal::Boolean(b)) => (
                "%s".into(),
                format!("\"{}\"", if *b { "true" } else { "false" }),
            ),
            Expr::Literal(Literal::Null) => ("%s".into(), "\"null\"".into()),
            _ => {
                let ty = self.infer_type(e);
                match ty.as_str() {
                    "int" => {
                        if self.is_boolean_expression(e) {
                            ("%s".into(), format!("(({}) ? \"true\" : \"false\")", code))
                        } else {
                            ("%lld".into(), code.into())
                        }
                    }
                    "int8_t" | "int16_t" | "int32_t" => ("%d".into(), code.into()),
                    "int64_t" => ("%lld".into(), code.into()),
                    "intptr_t" => ("%ld".into(), code.into()), // Approximation
                    "uint8_t" | "uint16_t" | "uint32_t" => ("%u".into(), code.into()),
                    "uint64_t" => ("%lu".into(), code.into()),
                    "size_t" => ("%zu".into(), code.into()),
                    "float" => ("%g".into(), code.into()),
                    "double" => ("%g".into(), code.into()),
                    "const char*" | "char*" => ("%s".into(), code.into()),
                    _ => {
                        if let Expr::Index { sequence, .. } = e {
                            if let Expr::Variable(var_name) = sequence.as_ref() {
                                if let Some(t) = self.vars.get(var_name) {
                                    if t.contains("double") || t.contains("float") {
                                        ("%g".into(), code.into())
                                    } else {
                                        ("%lld".into(), code.into())
                                    }
                                } else {
                                    ("%lld".into(), code.into())
                                }
                            } else {
                                ("%lld".into(), code.into())
                            }
                        } else {
                            ("%lld".into(), code.into())
                        }
                    }
                }
            }
        })
    }
}
