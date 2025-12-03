use super::error::LexerError;
use super::token::{Token, TokenType};

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
            // Skip whitespace before setting start
            self.skip_whitespace();
            if self.is_at_end() {
                break;
            }

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

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                _ => break,
            }
        }
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
                } else if self.match_char('>') {
                    TokenType::FatArrow
                } else {
                    TokenType::Assign
                };
                self.add_token(token_type);
            }
            '<' => {
                let token_type = if self.match_char('=') {
                    TokenType::LessEqual
                } else if self.match_char('<') {
                    TokenType::ShiftLeft
                } else {
                    TokenType::Less
                };
                self.add_token(token_type);
            }
            '>' => {
                let token_type = if self.match_char('=') {
                    TokenType::GreaterEqual
                } else if self.match_char('>') {
                    TokenType::ShiftRight
                } else {
                    TokenType::Greater
                };
                self.add_token(token_type);
            }
            '-' => {
                if self.match_char('>') {
                    self.add_token(TokenType::Arrow);
                } else if self.peek().is_ascii_digit() {
                    // This is a negative number literal, parse as number with negative sign
                    self.number_with_sign(true)?
                } else {
                    self.add_token(TokenType::Minus)
                }
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
                    self.add_token(TokenType::BitAnd);
                }
            }
            '|' => {
                if self.match_char('|') {
                    self.add_token(TokenType::Or);
                } else {
                    self.add_token(TokenType::BitOr);
                }
            }
            '^' => self.add_token(TokenType::BitXor),
            '~' => self.add_token(TokenType::BitNot),
            '@' => {
                // Annotation: @mut
                if self.source[self.current..].starts_with("mut") {
                    for _ in 0..3 { self.advance(); }
                    self.add_token(TokenType::AtMut);
                } else {
                    return Err(LexerError { message: "Unknown annotation after '@'".to_string(), line: self.line });
                }
            }
            '"' => self.string()?,
            '0'..='9' => {
                // Hex literal quick path: 0x... or 0X...
                if self.source[self.start..].starts_with("0x") || self.source[self.start..].starts_with("0X") {
                    // Consume leading '0' already read; ensure 'x' consumed by number_with_sign or here
                    // We are in scan_token, so current points after first digit; consume 'x' and parse hex
                    self.advance(); // consume 'x' or 'X'
                    self.hex_number(false)?;
                    return Ok(());
                }

                // Check if this might be an identifier starting with a digit (for module names like 06_functions)
                let mut temp_pos = self.current;

                // Skip the initial digits - use byte-safe character access
                while temp_pos < self.source.len() {
                    if let Some(ch) = self.source[temp_pos..].chars().next() {
                        if ch.is_ascii_digit() {
                            temp_pos += ch.len_utf8();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // Check if the next character is alphabetic or underscore
                if temp_pos < self.source.len() {
                    let next_char = if let Some(ch) = self.source[temp_pos..].chars().next() {
                        ch
                    } else {
                        '\0'
                    };

                    // Check if this could be a type suffix
                    let remaining = &self.source[temp_pos..];
                    let is_type_suffix = remaining.starts_with("i8")
                        || remaining.starts_with("i16")
                        || remaining.starts_with("i32")
                        || remaining.starts_with("i64")
                        || remaining.starts_with("isize")
                        || remaining.starts_with("u8")
                        || remaining.starts_with("u16")
                        || remaining.starts_with("u32")
                        || remaining.starts_with("u64")
                        || remaining.starts_with("usize");

                    if (next_char.is_alphabetic() || next_char == '_') && !is_type_suffix {
                        // This is an identifier starting with digits (like 06_functions)
                        self.identifier();
                    } else {
                        // This is a regular number (possibly with type suffix)
                        self.number()?
                    }
                } else {
                    // At end; regular number
                    self.number()?
                }
            }
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
        while self.peek().is_alphanumeric() || self.peek() == '_'
        {
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
            "pick" => TokenType::Pick,
            "when" => TokenType::When,
            "default" => TokenType::Default,
            "repeat" => TokenType::Repeat,
            "until" => TokenType::Until,
            "loop" => TokenType::Loop,
            "mod" => TokenType::Mod,
            "sub" => TokenType::Sub,
            "entry" => TokenType::Entry,
            // 'vault', 'pool', 'tree' are treated as identifiers to allow brace literals
            _ => TokenType::Identifier(text.clone()),
        };

        self.add_token(token_type);
    }

    fn number(&mut self) -> Result<(), LexerError> {
        self.number_with_sign(false)
    }

    fn number_with_sign(&mut self, is_negative: bool) -> Result<(), LexerError> {
        // If this is a negative number, we need to advance past the minus sign first
        if is_negative {
            self.advance(); // consume the minus sign
        }

        // Hex literal detection when starting with 0x or 0X
        if self.peek() == '0' && (self.peek_next() == 'x' || self.peek_next() == 'X') {
            // consume '0' and 'x'
            self.advance();
            self.advance();
            return self.hex_number(is_negative);
        }

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
            let saved_current = self.current; // Save position before consuming 'e'/'E'
            self.advance(); // consume 'e' or 'E'

            // Optional + or - after e/E
            if self.peek() == '+' || self.peek() == '-' {
                self.advance();
            }

            // Must have at least one digit after e/E
            if !self.peek().is_ascii_digit() {
                // Invalid scientific notation, backtrack to before 'e'/'E'
                self.current = saved_current;
            } else {
                while self.peek().is_ascii_digit() {
                    self.advance();
                }
            }
        }

        let text_str = self.source[self.start..self.current].to_string();

        // Check for type suffixes
        if self.current < self.source.len() {
            let remaining = &self.source[self.current..];

            // Handle all integer type suffixes
            if remaining.starts_with("i8") {
                self.parse_typed_integer::<i8>(&text_str, is_negative, "i8", TokenType::I8Literal)?;
                return Ok(());
            } else if remaining.starts_with("i16") {
                self.parse_typed_integer::<i16>(&text_str, is_negative, "i16", TokenType::I16Literal)?;
                return Ok(());
            } else if remaining.starts_with("i32") {
                self.parse_typed_integer::<i32>(&text_str, is_negative, "i32", TokenType::I32Literal)?;
                return Ok(());
            } else if remaining.starts_with("i64") {
                self.parse_typed_integer::<i64>(&text_str, is_negative, "i64", TokenType::I64Literal)?;
                return Ok(());
            } else if remaining.starts_with("isize") {
                self.parse_typed_integer::<isize>(
                    &text_str,
                    is_negative,
                    "isize",
                    TokenType::ISizeLiteral,
                )?;
                return Ok(());
            } else if remaining.starts_with("u8") {
                self.parse_typed_integer::<u8>(&text_str, is_negative, "u8", TokenType::U8Literal)?;
                return Ok(());
            } else if remaining.starts_with("u16") {
                self.parse_typed_integer::<u16>(&text_str, is_negative, "u16", TokenType::U16Literal)?;
                return Ok(());
            } else if remaining.starts_with("u32") {
                self.parse_typed_integer::<u32>(&text_str, is_negative, "u32", TokenType::U32Literal)?;
                return Ok(());
            } else if remaining.starts_with("u64") {
                self.parse_typed_integer::<u64>(&text_str, is_negative, "u64", TokenType::U64Literal)?;
                return Ok(());
            } else if remaining.starts_with("usize") {
                self.parse_typed_integer::<usize>(
                    &text_str,
                    is_negative,
                    "usize",
                    TokenType::USizeLiteral,
                )?;
                return Ok(());
            }
        }

        if text_str.contains('.') || text_str.contains('e') || text_str.contains('E') {
            let parsed_text = if is_negative {
                // For negative numbers, the text includes the minus sign, so we can parse directly
                text_str
            } else {
                text_str
            };
            match parsed_text.parse::<f64>() {
                Ok(value) => self.add_token(TokenType::Float(value)),
                Err(e) => {
                    return Err(LexerError {
                        message: format!("Failed to parse float '{}': {}", parsed_text, e),
                        line: self.line,
                    });
                }
            }
        } else {
            let parsed_text = if is_negative {
                // For negative numbers, the text includes the minus sign, so we can parse directly
                text_str
            } else {
                text_str
            };

            // For default integer literals (without type suffix), try i64 first, then u64 for large positive numbers
            match parsed_text.parse::<i64>() {
                Ok(value) => self.add_token(TokenType::Integer(value)),
                Err(_) => {
                    // If parsing as i64 fails, try parsing as u64 for large positive numbers
                    if !is_negative {
                        match parsed_text.parse::<u64>() {
                            Ok(value) => {
                                // For large u64 values that don't fit in i64, we need to handle them appropriately
                                // Since TokenType::Integer only supports i64, we must emit a helpful error
                                if value > i64::MAX as u64 {
                                    return Err(LexerError {
                                        message: format!("Integer literal '{}' is too large for default int type. Consider adding a 'u64' suffix: {}u64", parsed_text, parsed_text),
                                        line: self.line,
                                    });
                                } else {
                                    self.add_token(TokenType::Integer(value as i64))
                                }
                            }
                            Err(e) => {
                                return Err(LexerError {
                                    message: format!("Failed to parse integer '{}': {}", parsed_text, e),
                                    line: self.line,
                                });
                            }
                        }
                    } else {
                        return Err(LexerError {
                            message: format!("Failed to parse integer '{}': number too large for i64", parsed_text),
                            line: self.line,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    // Parse hexadecimal integer literal with optional type suffixes
    fn hex_number(&mut self, is_negative: bool) -> Result<(), LexerError> {
        // Consume hex digits [0-9a-fA-F]
        let mut saw_digit = false;
        while {
            let ch = self.peek();
            match ch {
                '0'..='9' | 'a'..='f' | 'A'..='F' => { self.advance(); saw_digit = true; true }
                _ => false,
            }
        } {}

        if !saw_digit {
            return Err(LexerError { message: "Invalid hex literal: expected hex digits after 0x".to_string(), line: self.line });
        }

        let text_str = self.source[self.start..self.current].to_string();

        // Strip optional sign and 0x prefix for parsing
        let mut hex_part = text_str.clone();
        // Remove leading '-' if present
        if is_negative && hex_part.starts_with("-") {
            hex_part = hex_part[1..].to_string();
        }
        // Remove leading 0x/0X
        let prefix_len = 2; // '0x'
        if hex_part.len() >= prefix_len {
            hex_part = hex_part[prefix_len..].to_string();
        }

        // Check for type suffixes following the hex digits
        if self.current < self.source.len() {
            let remaining = &self.source[self.current..];
            if remaining.starts_with("i8") {
                let value = i8::from_str_radix(&hex_part, 16).map_err(|_| LexerError { message: format!("Failed to parse i8 hex literal '{}'", hex_part), line: self.line })?;
                for _ in 0..2 { self.advance(); } // consume suffix
                self.add_token(TokenType::I8Literal(if is_negative { -value } else { value }));
                return Ok(());
            } else if remaining.starts_with("i16") {
                let value = i16::from_str_radix(&hex_part, 16).map_err(|_| LexerError { message: format!("Failed to parse i16 hex literal '{}'", hex_part), line: self.line })?;
                for _ in 0..3 { self.advance(); }
                self.add_token(TokenType::I16Literal(if is_negative { -value } else { value }));
                return Ok(());
            } else if remaining.starts_with("i32") {
                let value = i32::from_str_radix(&hex_part, 16).map_err(|_| LexerError { message: format!("Failed to parse i32 hex literal '{}'", hex_part), line: self.line })?;
                for _ in 0..3 { self.advance(); }
                self.add_token(TokenType::I32Literal(if is_negative { -value } else { value }));
                return Ok(());
            } else if remaining.starts_with("i64") {
                let value = i64::from_str_radix(&hex_part, 16).map_err(|_| LexerError { message: format!("Failed to parse i64 hex literal '{}'", hex_part), line: self.line })?;
                for _ in 0..3 { self.advance(); }
                self.add_token(TokenType::I64Literal(if is_negative { -value } else { value }));
                return Ok(());
            } else if remaining.starts_with("u8") {
                let value = u8::from_str_radix(&hex_part, 16).map_err(|_| LexerError { message: format!("Failed to parse u8 hex literal '{}'", hex_part), line: self.line })?;
                for _ in 0..2 { self.advance(); }
                self.add_token(TokenType::U8Literal(value));
                return Ok(());
            } else if remaining.starts_with("u16") {
                let value = u16::from_str_radix(&hex_part, 16).map_err(|_| LexerError { message: format!("Failed to parse u16 hex literal '{}'", hex_part), line: self.line })?;
                for _ in 0..3 { self.advance(); }
                self.add_token(TokenType::U16Literal(value));
                return Ok(());
            } else if remaining.starts_with("u32") {
                let value = u32::from_str_radix(&hex_part, 16).map_err(|_| LexerError { message: format!("Failed to parse u32 hex literal '{}'", hex_part), line: self.line })?;
                for _ in 0..3 { self.advance(); }
                self.add_token(TokenType::U32Literal(value));
                return Ok(());
            } else if remaining.starts_with("u64") {
                let value = u64::from_str_radix(&hex_part, 16).map_err(|_| LexerError { message: format!("Failed to parse u64 hex literal '{}'", hex_part), line: self.line })?;
                for _ in 0..3 { self.advance(); }
                self.add_token(TokenType::U64Literal(value));
                return Ok(());
            }
        }

        // Default: parse as i64 (or error for large unsigned without suffix)
        match i64::from_str_radix(&hex_part, 16) {
            Ok(mut value) => {
                if is_negative { value = -value; }
                self.add_token(TokenType::Integer(value));
                Ok(())
            }
            Err(_) => {
                // Try u64 if positive
                if !is_negative {
                    match u64::from_str_radix(&hex_part, 16) {
                        Ok(value) => {
                            if value > i64::MAX as u64 {
                                return Err(LexerError { message: format!("Hex integer '{}' too large for default int; add 'u64' suffix", text_str), line: self.line });
                            } else {
                                self.add_token(TokenType::Integer(value as i64));
                                Ok(())
                            }
                        }
                        Err(e) => Err(LexerError { message: format!("Failed to parse hex integer '{}': {}", text_str, e), line: self.line }),
                    }
                } else {
                    Err(LexerError { message: format!("Failed to parse hex integer '{}': number too large for i64", text_str), line: self.line })
                }
            }
        }
    }

    fn parse_typed_integer<T: std::str::FromStr + std::fmt::Debug>(
        &mut self,
        text: &str,
        is_negative: bool,
        suffix: &str,
        token_type_constructor: fn(T) -> TokenType,
    ) -> Result<(), LexerError> {
        let parsed_text = if is_negative {
            // For negative numbers, the text includes the minus sign, so we can parse directly
            text.to_string()
        } else {
            text.to_string()
        };

        match parsed_text.parse::<T>() {
            Ok(value) => {
                // Consume the suffix
                for _ in 0..suffix.len() {
                    self.advance();
                }
                self.add_token(token_type_constructor(value));
                Ok(())
            }
            Err(_) => {
                Err(LexerError {
                    message: format!("Failed to parse {} literal '{}'", suffix, parsed_text),
                    line: self.line,
                })
            }
        }
    }

    fn string(&mut self) -> Result<(), LexerError> {
        let mut value = String::new();
        while self.peek() != '"' && !self.is_at_end() {
            let c = self.peek();
            if c == '\\' {
                self.advance(); // consume '\\'
                if self.is_at_end() {
                    return Err(LexerError {
                        message: "Unterminated string.".to_string(),
                        line: self.line,
                    });
                }
                let next_char = self.peek();
                self.advance(); // consume escaped char
                match next_char {
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    other => {
                        value.push('\\');
                        value.push(other);
                    }
                }
            } else {
                if c == '\n' {
                    self.line += 1;
                }
                self.advance();
                value.push(c);
            }
        }

        if self.is_at_end() {
            return Err(LexerError {
                message: "Unterminated string".to_string(),
                line: self.line,
            });
        }

        // The closing ".
        self.advance();

        self.add_token(TokenType::String(value));

        Ok(())
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.peek() != expected {
            return false;
        }

        self.advance();
        true
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            // Get the character at the current byte position
            let remaining = &self.source[self.current..];
            remaining.chars().next().unwrap_or('\0')
        }
    }

    fn peek_next(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            let remaining = &self.source[self.current..];
            let mut chars = remaining.chars();
            chars.next(); // Skip current character
            chars.next().unwrap_or('\0')
        }
    }

    fn advance(&mut self) -> char {
        if self.is_at_end() {
            return '\0';
        }

        let remaining = &self.source[self.current..];
        let ch = remaining.chars().next().unwrap_or('\0');
        self.current += ch.len_utf8(); // Move by the byte length of the character
        ch
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
