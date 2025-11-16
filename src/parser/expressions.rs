//! Expression parsing module
//! 
//! This module handles parsing of expressions using Pratt parsing technique.
//! Expressions are parsed with operator precedence in mind, from lowest to highest precedence.

use crate::lexer::TokenType;
use crate::ast::*;
use super::ParseError;

/// Main expression parsing entry point
pub fn parse_expression(parser: &mut super::Parser) -> Result<Expr, ParseError> {
    parse_assignment(parser)
}

/// Parses assignment expressions (lowest precedence)
pub fn parse_assignment(parser: &mut super::Parser) -> Result<Expr, ParseError> {
    let expr = parse_or(parser)?;
    
    if parser.match_token(&TokenType::Assign) {
        let equals = parser.previous().clone();
        let value = parse_assignment(parser)?;
        
        if let Expr::Variable(name) = expr {
            return Ok(Expr::Assign {
                name,
                value: Box::new(value),
            });
        }
        
        // Handle array assignment: arr[i] = value
        if let Expr::Index { sequence, index } = expr {
            return Ok(Expr::AssignIndex {
                sequence,
                index,
                value: Box::new(value),
            });
        }
        
        return Err(ParseError {
            message: "Invalid assignment target".to_string(),
            line: equals.line,
        });
    }
    
    Ok(expr)
}

/// Parses logical OR expressions
pub fn parse_or(parser: &mut super::Parser) -> Result<Expr, ParseError> {
    let mut expr = parse_and(parser)?;
    
    while parser.match_token(&TokenType::Or) {
        let operator = parser.previous().clone();
        let right = parse_and(parser)?;
        expr = Expr::Binary {
            left: Box::new(expr),
            operator: parser.binary_operator_from_token(&operator)?,
            right: Box::new(right),
        };
    }
    
    Ok(expr)
}

/// Parses logical AND expressions
pub fn parse_and(parser: &mut super::Parser) -> Result<Expr, ParseError> {
    let mut expr = parse_equality(parser)?;
    
    while parser.match_token(&TokenType::And) {
        let operator = parser.previous().clone();
        let right = parse_equality(parser)?;
        expr = Expr::Binary {
            left: Box::new(expr),
            operator: parser.binary_operator_from_token(&operator)?,
            right: Box::new(right),
        };
    }
    
    Ok(expr)
}

/// Parses equality expressions (==, !=)
pub fn parse_equality(parser: &mut super::Parser) -> Result<Expr, ParseError> {
    let mut expr = parse_comparison(parser)?;
    
    while parser.match_token(&TokenType::EqualEqual) || parser.match_token(&TokenType::NotEqual) {
        let operator = parser.previous().clone();
        let right = parse_comparison(parser)?;
        expr = Expr::Binary {
            left: Box::new(expr),
            operator: parser.binary_operator_from_token(&operator)?,
            right: Box::new(right),
        };
    }
    
    Ok(expr)
}

/// Parses comparison expressions (<, <=, >, >=)
pub fn parse_comparison(parser: &mut super::Parser) -> Result<Expr, ParseError> {
    let mut expr = parse_term(parser)?;
    
    while parser.match_token(&TokenType::Greater) 
        || parser.match_token(&TokenType::GreaterEqual) 
        || parser.match_token(&TokenType::Less) 
        || parser.match_token(&TokenType::LessEqual) {
        let operator = parser.previous().clone();
        let right = parse_term(parser)?;
        expr = Expr::Binary {
            left: Box::new(expr),
            operator: parser.binary_operator_from_token(&operator)?,
            right: Box::new(right),
        };
    }
    
    Ok(expr)
}

/// Parses term expressions (+, -)
pub fn parse_term(parser: &mut super::Parser) -> Result<Expr, ParseError> {
    let mut expr = parse_factor(parser)?;
    
    while parser.match_token(&TokenType::Plus) || parser.match_token(&TokenType::Minus) {
        let operator = parser.previous().clone();
        let right = parse_factor(parser)?;
        expr = Expr::Binary {
            left: Box::new(expr),
            operator: parser.binary_operator_from_token(&operator)?,
            right: Box::new(right),
        };
    }
    
    Ok(expr)
}

/// Parses factor expressions (*, /, %)
pub fn parse_factor(parser: &mut super::Parser) -> Result<Expr, ParseError> {
    let mut expr = parse_unary(parser)?;
    
    while parser.match_token(&TokenType::Star) 
        || parser.match_token(&TokenType::Slash) 
        || parser.match_token(&TokenType::Percent) {
        let operator = parser.previous().clone();
        let right = parse_unary(parser)?;
        expr = Expr::Binary {
            left: Box::new(expr),
            operator: parser.binary_operator_from_token(&operator)?,
            right: Box::new(right),
        };
    }
    
    Ok(expr)
}

/// Parses unary expressions (-, !, &, &@mut)
pub fn parse_unary(parser: &mut super::Parser) -> Result<Expr, ParseError> {
    if parser.match_token(&TokenType::Minus) || parser.match_token(&TokenType::Not) {
        let operator = parser.previous().clone();
        let right = parse_unary(parser)?;
        return Ok(Expr::Unary {
            operator: parser.unary_operator_from_token(&operator)?,
            operand: Box::new(right),
        });
    }
    // Borrow operators
    if parser.match_token(&TokenType::BitAnd) {
        let mutable = if parser.match_token(&TokenType::AtMut) { true } else { false };
        let target = parse_unary(parser)?;
        return Ok(Expr::Borrow { target: Box::new(target), mutable });
    }
    
    parse_call(parser)
}

/// Parses function calls, property access, and array indexing
pub fn parse_call(parser: &mut super::Parser) -> Result<Expr, ParseError> {
    let mut expr = parse_primary(parser)?;
    
    loop {
        if parser.match_token(&TokenType::LeftParen) {
            expr = finish_call(parser, expr)?;
        } else if parser.match_token(&TokenType::Dot) {
            // Parse property access: object.name
            let prop_name = if let TokenType::Identifier(name) = &parser.peek().token_type {
                name.clone()
            } else {
                return Err(ParseError {
                    message: "Expected property name after '.'".to_string(),
                    line: parser.peek().line,
                });
            };
            parser.consume(&TokenType::Identifier(prop_name.clone()), "Expected property name after '.'")?;
            expr = Expr::Get { object: Box::new(expr), name: prop_name };
        } else if parser.match_token(&TokenType::LeftBracket) {
            // Parse array indexing: array[index]
            let index = parse_expression(parser)?;
            parser.consume(&TokenType::RightBracket, "Expected ']' after index expression")?;
            expr = Expr::Index { sequence: Box::new(expr), index: Box::new(index) };
        } else if parser.match_token(&TokenType::LeftBrace) {
            // Special collection literals following keywords vault/pool/tree
            match expr {
                Expr::Variable(ref name) if name == "vault" => {
                    let mut entries: Vec<(String, Expr)> = Vec::new();
                    if !parser.check(&TokenType::RightBrace) {
                        loop {
                            // keys must be string literals
                            if !parser.match_token(&TokenType::String("".to_string())) {
                                return Err(ParseError { message: "Expected string key in vault literal".to_string(), line: parser.peek().line });
                            }
                            let key_tok = parser.previous().clone();
                            let key = if let TokenType::String(s) = key_tok.token_type { s } else { unreachable!() };
                            parser.consume(&TokenType::Colon, "Expected ':' after key")?;
                            let value_expr = parse_expression(parser)?;
                            entries.push((key, value_expr));
                            if !parser.match_token(&TokenType::Comma) { break; }
                        }
                    }
                    parser.consume(&TokenType::RightBrace, "Expected '}' after vault entries")?;
                    expr = Expr::VaultLiteral { entries };
                }
                Expr::Variable(ref name) if name == "pool" => {
                    let mut elements: Vec<Expr> = Vec::new();
                    if !parser.check(&TokenType::RightBrace) {
                        loop {
                            elements.push(parse_expression(parser)?);
                            if !parser.match_token(&TokenType::Comma) { break; }
                        }
                    }
                    parser.consume(&TokenType::RightBrace, "Expected '}' after pool elements")?;
                    expr = Expr::PoolLiteral { elements };
                }
                Expr::Variable(ref name) if name == "tree" => {
                    // tree literal: {root [ , child ... ]}
                    if parser.check(&TokenType::RightBrace) {
                        return Err(ParseError { message: "tree literal requires a root".to_string(), line: parser.peek().line });
                    }
                    let root = parse_expression(parser)?;
                    let mut children: Vec<Expr> = Vec::new();
                    while parser.match_token(&TokenType::Comma) {
                        children.push(parse_expression(parser)?);
                    }
                    parser.consume(&TokenType::RightBrace, "Expected '}' after tree literal")?;
                    expr = Expr::TreeLiteral { root: Box::new(root), children };
                }
                _ => {
                    return Err(ParseError { message: "Unexpected '{' after expression".to_string(), line: parser.peek().line });
                }
            }
        } else {
            break;
        }
    }
    
    Ok(expr)
}

/// Finishes parsing function call expressions
pub fn finish_call(parser: &mut super::Parser, callee: Expr) -> Result<Expr, ParseError> {
    let mut arguments = Vec::new();
    
    if !parser.check(&TokenType::RightParen) {
        loop {
            arguments.push(parse_expression(parser)?);
            if !parser.match_token(&TokenType::Comma) {
                break;
            }
        }
    }
    
    parser.consume(&TokenType::RightParen, "Expected ')' after arguments")?;
    
    Ok(Expr::Call {
        callee: Box::new(callee),
        arguments,
    })
}

/// Parses primary expressions (literals, variables, parenthesized expressions, array literals)
pub fn parse_primary(parser: &mut super::Parser) -> Result<Expr, ParseError> {
    // Integer literals
    if parser.match_token(&TokenType::Integer(0)) {
        let token = parser.previous();
        if let TokenType::Integer(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::Integer(value)));
        }
    }
    
    // I8 literals
    if parser.match_token(&TokenType::I8Literal(0)) {
        let token = parser.previous();
        if let TokenType::I8Literal(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::I8(value)));
        }
    }
    
    // I16 literals
    if parser.match_token(&TokenType::I16Literal(0)) {
        let token = parser.previous();
        if let TokenType::I16Literal(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::I16(value)));
        }
    }
    
    // I32 literals
    if parser.match_token(&TokenType::I32Literal(0)) {
        let token = parser.previous();
        if let TokenType::I32Literal(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::I32(value)));
        }
    }
    
    // I64 literals
    if parser.match_token(&TokenType::I64Literal(0)) {
        let token = parser.previous();
        if let TokenType::I64Literal(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::I64(value)));
        }
    }
    
    // ISize literals
    if parser.match_token(&TokenType::ISizeLiteral(0)) {
        let token = parser.previous();
        if let TokenType::ISizeLiteral(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::ISize(value)));
        }
    }
    
    // U8 literals
    if parser.match_token(&TokenType::U8Literal(0)) {
        let token = parser.previous();
        if let TokenType::U8Literal(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::U8(value)));
        }
    }
    
    // U16 literals
    if parser.match_token(&TokenType::U16Literal(0)) {
        let token = parser.previous();
        if let TokenType::U16Literal(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::U16(value)));
        }
    }
    
    // U32 literals
    if parser.match_token(&TokenType::U32Literal(0)) {
        let token = parser.previous();
        if let TokenType::U32Literal(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::U32(value)));
        }
    }
    
    // U64 literals
    if parser.match_token(&TokenType::U64Literal(0)) {
        let token = parser.previous();
        if let TokenType::U64Literal(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::U64(value)));
        }
    }
    
    // USize literals
    if parser.match_token(&TokenType::USizeLiteral(0)) {
        let token = parser.previous();
        if let TokenType::USizeLiteral(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::USize(value)));
        }
    }
    
    // Float literals
    if parser.match_token(&TokenType::Float(0.0)) {
        let token = parser.previous();
        if let TokenType::Float(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::Float(value)));
        }
    }
    
    // String literals
    if parser.match_token(&TokenType::String("".to_string())) {
        let token = parser.previous();
        if let TokenType::String(value) = token.token_type.clone() {
            return Ok(Expr::Literal(Literal::String(value)));
        }
    }
    
    // Boolean literals
    if parser.match_token(&TokenType::True) {
        return Ok(Expr::Literal(Literal::Boolean(true)));
    }
    
    if parser.match_token(&TokenType::False) {
        return Ok(Expr::Literal(Literal::Boolean(false)));
    }
    
    // Null literal
    if parser.match_token(&TokenType::Null) {
        return Ok(Expr::Literal(Literal::Null));
    }
    
    // Variable references
    if let TokenType::Identifier(name) = &parser.peek().token_type {
        let name = name.clone();
        parser.consume(&TokenType::Identifier(name.clone()), "Expected identifier")?;
        return Ok(Expr::Variable(name));
    }
    // Brace literals after identifiers 'vault', 'pool', 'tree'
    if let TokenType::Identifier(name) = &parser.peek().token_type {
        if name == "vault" {
            parser.consume(&TokenType::Identifier(name.clone()), "Expected identifier")?;
            if parser.match_token(&TokenType::LeftBrace) {
                let mut entries: Vec<(String, Expr)> = Vec::new();
                if !parser.check(&TokenType::RightBrace) {
                    loop {
                        if !parser.match_token(&TokenType::String("".to_string())) {
                            return Err(ParseError { message: "Expected string key in vault literal".to_string(), line: parser.peek().line });
                        }
                        let key_tok = parser.previous().clone();
                        let key = if let TokenType::String(s) = key_tok.token_type { s } else { unreachable!() };
                        parser.consume(&TokenType::Colon, "Expected ':' after key")?;
                        let value_expr = parse_expression(parser)?;
                        entries.push((key, value_expr));
                        if !parser.match_token(&TokenType::Comma) { break; }
                    }
                }
                parser.consume(&TokenType::RightBrace, "Expected '}' after vault entries")?;
                return Ok(Expr::VaultLiteral { entries });
            }
            return Ok(Expr::Variable("vault".to_string()));
        } else if name == "pool" {
            parser.consume(&TokenType::Identifier(name.clone()), "Expected identifier")?;
            if parser.match_token(&TokenType::LeftBrace) {
                let mut elements: Vec<Expr> = Vec::new();
                if !parser.check(&TokenType::RightBrace) {
                    loop {
                        elements.push(parse_expression(parser)?);
                        if !parser.match_token(&TokenType::Comma) { break; }
                    }
                }
                parser.consume(&TokenType::RightBrace, "Expected '}' after pool elements")?;
                return Ok(Expr::PoolLiteral { elements });
            }
            return Ok(Expr::Variable("pool".to_string()));
        } else if name == "tree" {
            parser.consume(&TokenType::Identifier(name.clone()), "Expected identifier")?;
            if parser.match_token(&TokenType::LeftBrace) {
                if parser.check(&TokenType::RightBrace) {
                    return Err(ParseError { message: "tree literal requires a root".to_string(), line: parser.peek().line });
                }
                let root = parse_expression(parser)?;
                let mut children: Vec<Expr> = Vec::new();
                while parser.match_token(&TokenType::Comma) {
                    children.push(parse_expression(parser)?);
                }
                parser.consume(&TokenType::RightBrace, "Expected '}' after tree literal")?;
                return Ok(Expr::TreeLiteral { root: Box::new(root), children });
            }
            return Ok(Expr::Variable("tree".to_string()));
        }
    }
    
    // Parenthesized expressions
    if parser.match_token(&TokenType::LeftParen) {
        let expr = parse_expression(parser)?;
        parser.consume(&TokenType::RightParen, "Expected ')' after expression")?;
        return Ok(expr);
    }
    
    // Array literals: [expr1, expr2, ...]
    if parser.match_token(&TokenType::LeftBracket) {
        let mut elements = Vec::new();
        
        if !parser.check(&TokenType::RightBracket) {
            loop {
                elements.push(parse_expression(parser)?);
                if !parser.match_token(&TokenType::Comma) {
                    break;
                }
            }
        }
        
        parser.consume(&TokenType::RightBracket, "Expected ']' after array elements")?;
        return Ok(Expr::ArrayLiteral { elements });
    }
    
    Err(ParseError {
        message: format!("Expected expression, got {:?}", parser.peek().token_type),
        line: parser.peek().line,
    })
}