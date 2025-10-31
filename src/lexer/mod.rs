use std::fmt;

#[cfg(test)]
mod tests;

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
    
    // Identifiers and literals
    Identifier(String),
    String(String),
    Integer(i64),
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

#[derive(Debug)]
pub struct LexerError {
    pub message: String,
    pub line: usize,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lexer error on line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for LexerError {}

pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError> {
    let mut lexer = Lexer::new(source);
    lexer.tokenize()
}

pub struct Lexer {
    source: String,
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            tokens: Vec::new(),
            start: 0,
            current: 0,
            line: 1,
        }
    }
    
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token()?;
        }
        
        self.tokens.push(Token {
            token_type: TokenType::Eof,
            lexeme: "".to_string(),
            line: self.line,
        });
        
        Ok(self.tokens.clone())
    }
    
    fn scan_token(&mut self) -> Result<(), LexerError> {
        let c = self.advance();
        match c {
            '(' => self.add_token(TokenType::LeftParen),
            ')' => self.add_token(TokenType::RightParen),
            '{' => self.add_token(TokenType::LeftBrace),
            '}' => self.add_token(TokenType::RightBrace),
            '[' => self.add_token(TokenType::LeftBracket),
            ']' => self.add_token(TokenType::RightBracket),
            ';' => self.add_token(TokenType::Semicolon),
            ':' => self.add_token(TokenType::Colon),
            ',' => self.add_token(TokenType::Comma),
            '.' => self.add_token(TokenType::Dot),
            '-' => self.add_token(TokenType::Minus),
            '+' => self.add_token(TokenType::Plus),
            '*' => self.add_token(TokenType::Star),
            '!' => {
                let token_type = if self.match_char('=') {
                    TokenType::NotEqual
                } else {
                    TokenType::Not
                };
                self.add_token(token_type);
            }
            '=' => {
                let token_type = if self.match_char('=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Assign
                };
                self.add_token(token_type);
            }
            '<' => {
                let token_type = if self.match_char('=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                self.add_token(token_type);
            }
            '>' => {
                let token_type = if self.match_char('=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                self.add_token(token_type);
            }
            '/' => {
                if self.match_char('/') {
                    // A comment goes until the end of the line
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                } else {
                    self.add_token(TokenType::Slash);
                }
            }
            '%' => self.add_token(TokenType::Percent),
            '&' => {
                if self.match_char('&') {
                    self.add_token(TokenType::And);
                } else {
                    return Err(LexerError {
                        message: "Unexpected character: &".to_string(),
                        line: self.line,
                    });
                }
            }
            '|' => {
                if self.match_char('|') {
                    self.add_token(TokenType::Or);
                } else {
                    return Err(LexerError {
                        message: "Unexpected character: |".to_string(),
                        line: self.line,
                    });
                }
            }
            ' ' | '\r' | '\t' => {
                // Ignore whitespace
            }
            '\n' => {
                self.line += 1;
            }
            '"' => self.string()?,
            '0'..='9' => self.number(),
            'a'..='z' | 'A'..='Z' | '_' => self.identifier(),
            _ => {
                return Err(LexerError {
                    message: format!("Unexpected character: {}", c),
                    line: self.line,
                });
            }
        }
        
        Ok(())
    }
    
    fn identifier(&mut self) {
        while self.peek().is_alphanumeric() || self.peek() == '_' {
            self.advance();
        }
        
        let text = self.source[self.start..self.current].to_string();
        let token_type = match text.as_str() {
            "store" => TokenType::Store,
            "def" => TokenType::Def,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "while" => TokenType::While,
            "for" => TokenType::For,
            "return" => TokenType::Return,
            "break" => TokenType::Break,
            "continue" => TokenType::Continue,
            "import" => TokenType::Import,
            "as" => TokenType::As,
            "from" => TokenType::From,
            "export" => TokenType::Export,
            "ASSIGN_MAIN" => TokenType::AssignMain,
            "true" => TokenType::True,
            "false" => TokenType::False,
            "null" => TokenType::Null,
            _ => TokenType::Identifier(text.clone()),
        };
        
        self.add_token(token_type);
    }
    
    fn number(&mut self) {
        while self.peek().is_ascii_digit() {
            self.advance();
        }
        
        // Look for a fractional part
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            // Consume the "."
            self.advance();
            
            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }
        
        // Look for scientific notation (e or E)
        if self.peek() == 'e' || self.peek() == 'E' {
            self.advance(); // consume 'e' or 'E'
            
            // Optional + or - after e/E
            if self.peek() == '+' || self.peek() == '-' {
                self.advance();
            }
            
            // Must have at least one digit after e/E
            if !self.peek().is_ascii_digit() {
                // Invalid scientific notation, backtrack
                // This is a simple approach - in a real lexer you might want better error handling
                return;
            }
            
            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }
        
        let text = &self.source[self.start..self.current];
        
        if text.contains('.') || text.contains('e') || text.contains('E') {
            let value: f64 = text.parse().unwrap();
            self.add_token(TokenType::Float(value));
        } else {
            let value: i64 = text.parse().unwrap();
            self.add_token(TokenType::Integer(value));
        }
    }
    
    fn string(&mut self) -> Result<(), LexerError> {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }
        
        if self.is_at_end() {
            return Err(LexerError {
                message: "Unterminated string".to_string(),
                line: self.line,
            });
        }
        
        // The closing ".
        self.advance();
        
        // Trim the surrounding quotes.
        let value = self.source[(self.start + 1)..(self.current - 1)].to_string();
        self.add_token(TokenType::String(value));
        
        Ok(())
    }
    
    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.source.chars().nth(self.current).unwrap() != expected {
            return false;
        }
        
        self.current += 1;
        true
    }
    
    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source.chars().nth(self.current).unwrap()
        }
    }
    
    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            '\0'
        } else {
            self.source.chars().nth(self.current + 1).unwrap()
        }
    }
    
    fn advance(&mut self) -> char {
        self.current += 1;
        self.source.chars().nth(self.current - 1).unwrap()
    }
    
    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }
    
    fn add_token(&mut self, token_type: TokenType) {
        let lexeme = self.source[self.start..self.current].to_string();
        self.tokens.push(Token {
            token_type,
            lexeme,
            line: self.line,
        });
    }
}