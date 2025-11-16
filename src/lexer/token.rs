use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Keywords
    Store,
    Def,
    If,
    Else,
    While,
    For,
    Return,
    Break,
    Continue,
    Import,
    As,
    From,
    Export,
    AssignMain,
    True,
    False,
    Null,
    Pick,
    When,
    Default,
    Repeat,
    Until,
    Loop,
    // Annotation
    AtMut,
    
    // Identifiers and literals
    Identifier(String),
    String(String),
    Integer(i64),
    I8Literal(i8),
    I16Literal(i16),
    I32Literal(i32),
    I64Literal(i64),
    ISizeLiteral(isize),
    U8Literal(u8),
    U16Literal(u16),
    U32Literal(u32),
    U64Literal(u64),
    USizeLiteral(usize),
    Float(f64),
    
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Not,
    // Bitwise and shifts
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    ShiftLeft,
    ShiftRight,
    
    // Delimiters
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Colon,
    Semicolon,
    Comma,
    Dot,
    Arrow,
    FatArrow,
    
    // Assignment
    Assign,
    
    // Other
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub line: usize,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} '{}'", self.token_type, self.lexeme)
    }
}
