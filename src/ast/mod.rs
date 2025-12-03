use std::fmt;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Statement {
    Expression(Expr),
    LetDeclaration {
        name: String,
        initializer: Option<Expr>,
        var_type: Option<Type>,
        is_mutable: bool,
        is_exported: bool,
    },
    FunctionDeclaration {
        name: String,
        parameters: Vec<Parameter>,
        body: Vec<Statement>,
        return_type: Option<Type>,
        is_exported: bool,
    },
    Block {
        statements: Vec<Statement>,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    While {
        condition: Box<Expr>,
        body: Box<Statement>,
    },
    For {
        initializer: Option<Box<Statement>>,
        condition: Option<Box<Expr>>,
        increment: Option<Box<Expr>>,
        body: Box<Statement>,
    },
    Return {
        value: Option<Box<Expr>>,
    },
    Break,
    Continue,
    Import {
        module: String,
        alias: Option<String>,
    },
    ImportFrom {
        module: String,
        items: Vec<(String, Option<String>)>,
    },
    AssignMain {
        function_name: String,
    },
    Pick {
        expression: Box<Expr>,
        cases: Vec<WhenCase>,
        default: Option<Box<Statement>>,
    },
    RepeatUntil {
        body: Box<Statement>,
        condition: Box<Expr>,
    },
    Loop {
        body: Box<Statement>,
    },
    // Module system statements
    SubModDeclaration {
        name: String,
    },
    EntryModDeclaration {
        name: String,
        exports: Vec<String>,  // List of submodule names to re-export
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct WhenCase {
    pub values: Vec<Expr>,
    pub body: Box<Statement>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: Option<Type>,
    pub inferred_type: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Type {
    Integer,
    I8,
    I16,
    I32,
    I64,
    ISize,
    U8,
    U16,
    U32,
    U64,
    USize,
    F32,
    F64,
    Float,
    Boolean,
    String,
    Array(Box<Type>, usize),
    Function { params: Vec<Type>, return_type: Box<Type> },
    Void,
    // Advanced type system features
    Unknown,           // For type inference
    Infer,            // Placeholder for type inference
    Generic(String),   // Generic type parameter
    Option(Box<Type>), // Optional type
    Result(Box<Type>, Box<Type>), // Result<T, E>
    Union(Vec<Type>),  // Union type
    Tuple(Vec<Type>),  // Tuple type
    Vault(Box<Type>, Box<Type>),
    Pool(Box<Type>),
    Tree(Box<Type>),
    Ref(Box<Type>),
    RefMut(Box<Type>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Expr {
    Literal(Literal),
    Variable(String),
    Binary {
        left: Box<Expr>,
        operator: BinaryOperator,
        right: Box<Expr>,
    },
    Unary {
        operator: UnaryOperator,
        operand: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    Function {
        parameters: Vec<Parameter>,
        body: Vec<Statement>,
        return_type: Option<Type>,
    },
    Get {
        object: Box<Expr>,
        name: String,
    },
    Set {
        object: Box<Expr>,
        name: String,
        value: Box<Expr>,
    },
    Index {
        sequence: Box<Expr>,
        index: Box<Expr>,
    },
    Assign {
        name: String,
        value: Box<Expr>,
    },
    AssignIndex {
        sequence: Box<Expr>,
        index: Box<Expr>,
        value: Box<Expr>,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
    },
    Tuple {
        elements: Vec<Expr>,
    },
    IfExpression {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Match {
        expression: Box<Expr>,
        cases: Vec<MatchCase>,
    },
    VaultLiteral {
        entries: Vec<(String, Expr)>,
    },
    PoolLiteral {
        elements: Vec<Expr>,
    },
    TreeLiteral {
        root: Box<Expr>,
        children: Vec<Expr>,
    },
    Borrow {
        target: Box<Expr>,
        mutable: bool,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Pattern {
    Literal(Literal),
    Variable(String),
    Tuple(Vec<Pattern>),
    Wildcard,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Literal {
    Integer(i64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    ISize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    USize(usize),
    Float(f64),
    Boolean(bool),
    String(String),
    Null,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum BinaryOperator {
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum UnaryOperator {
    Negate,
    Not,
    BitNot,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Integer => write!(f, "int"),
            Type::I8 => write!(f, "i8"),
            Type::I16 => write!(f, "i16"),
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::ISize => write!(f, "isize"),
            Type::U8 => write!(f, "u8"),
            Type::U16 => write!(f, "u16"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "u64"),
            Type::USize => write!(f, "usize"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            // Legacy alias retained for backward compatibility; treat as f64
            Type::Float => write!(f, "f64"),
            Type::Boolean => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Array(inner, size) => write!(f, "[{}; {}]", inner, size),
            Type::Function { params, return_type } => {
                let param_types: Vec<String> = params.iter().map(|p| format!("{}", p)).collect();
                write!(f, "fn({}) -> {}", param_types.join(", "), return_type)
            }
            Type::Void => write!(f, "void"),
            // Advanced types
            Type::Unknown => write!(f, "unknown"),
            Type::Infer => write!(f, "_"),
            Type::Generic(name) => write!(f, "{}", name),
            Type::Option(inner) => write!(f, "Option<{}>", inner),
            Type::Result(ok, err) => write!(f, "Result<{}, {}>", ok, err),
            Type::Union(types) => {
                let type_strings: Vec<String> = types.iter().map(|t| format!("{}", t)).collect();
                write!(f, "Union<{}>", type_strings.join(" | "))
            },
            Type::Tuple(types) => {
                let type_strings: Vec<String> = types.iter().map(|t| format!("{}", t)).collect();
                write!(f, "({})", type_strings.join(", "))
            },
            Type::Vault(k, v) => write!(f, "vault<{}, {}>", k, v),
            Type::Pool(t) => write!(f, "pool<{}>", t),
            Type::Tree(t) => write!(f, "tree<{}>", t),
            Type::Ref(inner) => write!(f, "&{}", inner),
            Type::RefMut(inner) => write!(f, "&mut {}", inner),
        }
    }
}
