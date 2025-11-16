//! Type parsing module
//! 
//! This module handles parsing of type annotations and type expressions.

use crate::lexer::TokenType;
use crate::ast::Type;
use super::ParseError;

/// Parses type annotations and expressions
pub fn parse_type(parser: &mut super::Parser) -> Result<Type, ParseError> {
    // Reference types: &T
    if parser.match_token(&TokenType::BitAnd) {
        let inner = parse_type(parser)?;
        return Ok(Type::Ref(Box::new(inner)));
    }
    // Check for array type syntax: [T; N]
    if parser.match_token(&TokenType::LeftBracket) {
        let element_type = parse_type(parser)?;
        
        parser.consume(&TokenType::Semicolon, "Expected ';' in array type")?;
        
        // Parse array size (must be a positive integer literal)
        let size_token = parser.advance();
        let size = match &size_token.token_type {
            TokenType::Integer(n) if *n > 0 => *n as usize,
            _ => {
                return Err(ParseError {
                    message: "Array size must be a positive integer".to_string(),
                    line: size_token.line,
                })
            }
        };
        
        parser.consume(&TokenType::RightBracket, "Expected ']' after array type")?;
        
        Ok(Type::Array(Box::new(element_type), size))
    } else if parser.match_identifier("int") {
        Ok(Type::Integer)
    } else if parser.match_identifier("i8") {
        Ok(Type::I8)
    } else if parser.match_identifier("i16") {
        Ok(Type::I16)
    } else if parser.match_identifier("i32") {
        Ok(Type::I32)
    } else if parser.match_identifier("i64") {
        Ok(Type::I64)
    } else if parser.match_identifier("isize") {
        Ok(Type::ISize)
    } else if parser.match_identifier("u8") {
        Ok(Type::U8)
    } else if parser.match_identifier("u16") {
        Ok(Type::U16)
    } else if parser.match_identifier("u32") {
        Ok(Type::U32)
    } else if parser.match_identifier("u64") {
        Ok(Type::U64)
    } else if parser.match_identifier("usize") {
        Ok(Type::USize)
    } else if parser.match_identifier("f32") {
        Ok(Type::F32)
    } else if parser.match_identifier("f64") {
        Ok(Type::F64)
    } else if parser.match_identifier("float") {
        // Legacy alias: treat `float` as f64
        Ok(Type::F64)
    } else if parser.match_identifier("bool") {
        Ok(Type::Boolean)
    } else if parser.match_identifier("string") {
        Ok(Type::String)
    } else if parser.match_identifier("void") {
        Ok(Type::Void)
    } else {
        Err(ParseError {
            message: "Expected type".to_string(),
            line: parser.peek().line,
        })
    }
}

/// Helper function to match identifier tokens for type parsing
pub fn match_identifier(parser: &mut super::Parser, name: &str) -> bool {
    if let Some(token) = parser.tokens.get(parser.current) {
        if let TokenType::Identifier(id) = &token.token_type {
            if id == name {
                parser.current += 1;
                return true;
            }
        }
    }
    false
}