//! Statement parsing module
//! 
//! This module handles parsing of various statement types including:
//! - Control flow statements (if, while, for, pick, repeat, loop)
//! - Block statements
//! - Expression statements
//! - Jump statements (return, break, continue)

use crate::lexer::TokenType;
use crate::ast::*;
use super::ParseError;

/// Parses block statements (enclosed in {})
pub fn parse_block(parser: &mut super::Parser) -> Result<Vec<Statement>, ParseError> {
    parser.consume(&TokenType::LeftBrace, "Expected '{' before block")?;
    
    let mut statements = Vec::new();
    while !parser.check(&TokenType::RightBrace) && !parser.is_at_end() {
        statements.push(parser.declaration()?);
    }
    
    parser.consume(&TokenType::RightBrace, "Expected '}' after block")?;
    Ok(statements)
}

/// Parses if statements
pub fn parse_if_statement(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    parser.consume(&TokenType::LeftParen, "Expected '(' after 'if'")?;
    let condition = Box::new(parser.expression()?);
    parser.consume(&TokenType::RightParen, "Expected ')' after if condition")?;
    
    let then_branch = Box::new(parser.statement()?);
    
    let else_branch = if parser.match_token(&TokenType::Else) {
        Some(Box::new(parser.statement()?))
    } else {
        None
    };
    
    Ok(Statement::If {
        condition,
        then_branch,
        else_branch,
    })
}

/// Parses while statements
pub fn parse_while_statement(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    parser.consume(&TokenType::LeftParen, "Expected '(' after 'while'")?;
    let condition = Box::new(parser.expression()?);
    parser.consume(&TokenType::RightParen, "Expected ')' after while condition")?;
    
    let body = Box::new(parser.statement()?);
    
    Ok(Statement::While { condition, body })
}

/// Parses for statements
pub fn parse_for_statement(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    parser.consume(&TokenType::LeftParen, "Expected '(' after 'for'")?;

    let initializer = if parser.match_token(&TokenType::Semicolon) {
        None
    } else if parser.match_token(&TokenType::Store) {
        let name = if let TokenType::Identifier(name) = &parser.peek().token_type {
            name.clone()
        } else {
            return Err(ParseError {
                message: "Expected variable name".to_string(),
                line: parser.peek().line,
            });
        };
        parser.consume(&TokenType::Identifier(name.clone()), "Expected variable name")?;
        
        let mut var_type = None;
        if parser.match_token(&TokenType::Colon) {
            var_type = Some(parser.parse_type()?);
        }
        
        let mut initializer_expr = None;
        if parser.match_token(&TokenType::Assign) {
            initializer_expr = Some(parser.expression()?);
        }
        
        parser.consume(&TokenType::Semicolon, "Expected ';' after for loop initializer")?;

        let let_stmt = Statement::LetDeclaration {
            name,
            initializer: initializer_expr,
            var_type,
            is_mutable: false,
            is_exported: false,
        };
        Some(Box::new(let_stmt))
    } else {
        let expr = parser.expression()?;
        parser.consume(&TokenType::Semicolon, "Expected ';' after for loop initializer")?;
        Some(Box::new(Statement::Expression(expr)))
    };

    let condition = if parser.check(&TokenType::Semicolon) {
        None
    } else {
        Some(Box::new(parser.expression()?))
    };
    parser.consume(&TokenType::Semicolon, "Expected ';' after loop condition")?;

    let increment = if parser.check(&TokenType::RightParen) {
        None
    } else {
        Some(Box::new(parser.expression()?))
    };
    parser.consume(&TokenType::RightParen, "Expected ')' after for clauses")?;

    let body = Box::new(parser.statement()?);

    Ok(Statement::For {
        initializer,
        condition,
        increment,
        body,
    })
}

/// Parses return statements
pub fn parse_return_statement(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    let value = if parser.check(&TokenType::Semicolon) {
        None
    } else {
        Some(Box::new(parser.expression()?))
    };
    
    parser.consume(&TokenType::Semicolon, "Expected ';' after return value")?;
    Ok(Statement::Return { value })
}

/// Parses break statements
pub fn parse_break_statement(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    parser.consume(&TokenType::Semicolon, "Expected ';' after 'break'")?;
    Ok(Statement::Break)
}

/// Parses continue statements
pub fn parse_continue_statement(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    parser.consume(&TokenType::Semicolon, "Expected ';' after 'continue'")?;
    Ok(Statement::Continue)
}

/// Parses pick statements (pattern matching)
pub fn parse_pick_statement(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    let expression = Box::new(parser.expression()?);
    parser.consume(&TokenType::LeftBrace, "Expected '{' after pick expression")?;

    let mut cases = Vec::new();
    let mut default = None;

    while !parser.check(&TokenType::RightBrace) && !parser.is_at_end() {
        if parser.match_token(&TokenType::When) {
            let mut values = Vec::new();
            loop {
                values.push(parser.expression()?);
                if !parser.match_token(&TokenType::Comma) {
                    break;
                }
            }
            parser.consume(&TokenType::FatArrow, "Expected '=>' after when values")?;
            let body = Box::new(parser.statement()?);
            cases.push(WhenCase { values, body });
        } else if parser.match_token(&TokenType::Default) {
            if default.is_some() {
                return Err(ParseError {
                    message: "Multiple default cases in pick statement".to_string(),
                    line: parser.previous().line,
                });
            }
            parser.consume(&TokenType::FatArrow, "Expected '=>' after 'default'")?;
            default = Some(Box::new(parser.statement()?));
        } else {
            return Err(ParseError {
                message: "Expected 'when' or 'default' inside pick statement".to_string(),
                line: parser.peek().line,
            });
        }
    }

    parser.consume(&TokenType::RightBrace, "Expected '}' after pick body")?;

    Ok(Statement::Pick {
        expression,
        cases,
        default,
    })
}

/// Parses repeat-until statements
pub fn parse_repeat_until_statement(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    // Parse the body (block statement)
    let body = Box::new(parser.statement()?);
    
    // Expect the 'until' keyword
    parser.consume(&TokenType::Until, "Expected 'until' after repeat body")?;
    
    // Parse the condition (expression)
    let condition = Box::new(parser.expression()?);
    
    // Expect semicolon after the condition
    parser.consume(&TokenType::Semicolon, "Expected ';' after until condition")?;
    
    Ok(Statement::RepeatUntil { body, condition })
}

/// Parses infinite loop statements
pub fn parse_loop_statement(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    // Parse the body (block statement)
    let body = Box::new(parser.statement()?);
    
    // Expect semicolon after the loop body
    parser.consume(&TokenType::Semicolon, "Expected ';' after loop body")?;
    
    Ok(Statement::Loop { body })
}

/// Parses expression statements
pub fn parse_expression_statement(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    let expr = parser.expression()?;
    parser.consume(&TokenType::Semicolon, "Expected ';' after expression")?;
    Ok(Statement::Expression(expr))
}