//! Declaration parsing module
//! 
//! This module handles parsing of various declaration types including:
//! - Variable declarations (`store` statements)
//! - Function declarations (`def` statements)
//! - Import/export declarations
//! - Type annotations and parsing

use crate::lexer::TokenType;
use crate::ast::*;
use super::ParseError;

/// Parses variable declarations (store statements)
pub fn parse_let_declaration(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    // Optional '@mut' before variable name when called standalone (e.g., in declarations context)
    let is_mutable = if parser.match_token(&TokenType::AtMut) { true } else { false };
    let name = if let TokenType::Identifier(name) = &parser.peek().token_type {
        name.clone()
    } else {
        return Err(ParseError {
            message: "Expected variable name".to_string(),
            line: parser.peek().line,
        });
    };
    
    parser.consume(&TokenType::Identifier(name.clone()), "Expected variable name")?;
    
    // Check for type annotation
    let mut var_type = None;
    if parser.match_token(&TokenType::Colon) {
        var_type = Some(parser.parse_type()?);
    }
    
    let mut initializer = None;
    if parser.match_token(&TokenType::Assign) {
        initializer = Some(parser.expression()?);
    }
    
    parser.consume(&TokenType::Semicolon, "Expected ';' after variable declaration")?;
    
    Ok(Statement::LetDeclaration { 
        name, 
        initializer,
        var_type,
        is_mutable,
        is_exported: false 
    })
}

/// Parses export declarations (export store/def statements)
pub fn parse_export_declaration(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    if parser.match_token(&TokenType::Store) {
        let mut stmt = parse_let_declaration(parser)?;
        if let Statement::LetDeclaration { ref mut is_exported, .. } = stmt {
            *is_exported = true;
        }
        Ok(stmt)
    } else if parser.match_token(&TokenType::Def) {
        let mut stmt = parse_function_declaration(parser)?;
        if let Statement::FunctionDeclaration { ref mut is_exported, .. } = stmt {
            *is_exported = true;
        }
        Ok(stmt)
    } else {
        // Check for "export submodule1, submodule2;" syntax in export.nlang
        // This is used for re-exporting submodules in entry mod declarations
        // We'll handle this as a special case and skip it during regular parsing
        // since it's only meaningful in export.nlang files
        Err(ParseError {
            message: "Expected 'store' or 'def' after 'export'".to_string(),
            line: parser.peek().line,
        })
    }
}

/// Parses function declarations (def statements)
pub fn parse_function_declaration(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    let name = if let TokenType::Identifier(name) = &parser.peek().token_type {
        name.clone()
    } else {
        return Err(ParseError {
            message: "Expected function name".to_string(),
            line: parser.peek().line,
        });
    };
    
    parser.consume(&TokenType::Identifier(name.clone()), "Expected function name")?;
    parser.consume(&TokenType::LeftParen, "Expected '(' after function name")?;
    
    let mut parameters = Vec::new();
    if !parser.check(&TokenType::RightParen) {
        loop {
            let param_name = if let TokenType::Identifier(name) = &parser.peek().token_type {
                name.clone()
            } else {
                return Err(ParseError {
                    message: "Expected parameter name".to_string(),
                    line: parser.peek().line,
                });
            };
            
            parser.consume(&TokenType::Identifier(param_name.clone()), "Expected parameter name")?;
            
            let param_type = if parser.match_token(&TokenType::Colon) {
                Some(parser.parse_type()?)
            } else {
                None
            };

            parameters.push(Parameter {
                name: param_name,
                param_type,
                inferred_type: None,
            });
            
            if !parser.match_token(&TokenType::Comma) {
                break;
            }
        }
    }
    
    parser.consume(&TokenType::RightParen, "Expected ')' after parameters")?;
    
    // For now, we'll assume return type is void unless specified
    let return_type = if parser.match_token(&TokenType::Arrow) {
        // For now, just consume the type - we'll implement proper type parsing later
        Some(parser.parse_type()?)
    } else {
        None
    };
    
    let body = if parser.check(&TokenType::LeftBrace) {
        parser.block()?
    } else {
        return Err(ParseError {
            message: "Expected function body".to_string(),
            line: parser.peek().line,
        });
    };
    
    Ok(Statement::FunctionDeclaration {
        name,
        parameters,
        body,
        return_type,
        is_exported: false,
    })
}

/// Parses import declarations
pub fn parse_import_declaration(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    parser.consume(&TokenType::Import, "Expected 'import' keyword")?;
    
    let module = if let TokenType::Identifier(name) = &parser.peek().token_type {
        name.clone()
    } else if let TokenType::String(name) = &parser.peek().token_type {
        name.clone()
    } else {
        return Err(ParseError {
            message: "Expected module name".to_string(),
            line: parser.peek().line,
        });
    };
    
    // Consume either identifier or string token
    match &parser.peek().token_type {
        TokenType::Identifier(_) => {
            parser.consume(&TokenType::Identifier(module.clone()), "Expected module name")?;
        }
        TokenType::String(_) => {
            parser.consume(&TokenType::String(module.clone()), "Expected module name")?;
        }
        _ => {
            return Err(ParseError {
                message: "Expected module name".to_string(),
                line: parser.peek().line,
            });
        }
    }
    
    if parser.match_token(&TokenType::As) {
        let alias = if let TokenType::Identifier(name) = &parser.peek().token_type {
            Some(name.clone())
        } else {
            return Err(ParseError {
                message: "Expected alias name".to_string(),
                line: parser.peek().line,
            });
        };
        
        parser.consume(&TokenType::Identifier(alias.clone().unwrap()), "Expected alias name")?;
        parser.consume(&TokenType::Semicolon, "Expected ';' after import statement")?;
        
        Ok(Statement::Import { module, alias })
    } else if parser.match_token(&TokenType::From) {
        // Handle from ... import ...
        let items = parse_import_list(parser)?;
        parser.consume(&TokenType::Semicolon, "Expected ';' after import statement")?;
        Ok(Statement::ImportFrom { module, items })
    } else {
        parser.consume(&TokenType::Semicolon, "Expected ';' after import statement")?;
        Ok(Statement::Import { module, alias: None })
    }
}

/// Parses from import declarations
pub fn parse_from_import_declaration(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    parser.consume(&TokenType::From, "Expected 'from' keyword")?;
    
    let module = if let TokenType::Identifier(name) = &parser.peek().token_type {
        name.clone()
    } else if let TokenType::String(name) = &parser.peek().token_type {
        name.clone()
    } else {
        return Err(ParseError {
            message: "Expected module name".to_string(),
            line: parser.peek().line,
        });
    };
    
    // Consume either identifier or string token
    match &parser.peek().token_type {
        TokenType::Identifier(_) => {
            parser.consume(&TokenType::Identifier(module.clone()), "Expected module name")?;
        }
        TokenType::String(_) => {
            parser.consume(&TokenType::String(module.clone()), "Expected module name")?;
        }
        _ => {
            return Err(ParseError {
                message: "Expected module name".to_string(),
                line: parser.peek().line,
            });
        }
    }
    parser.consume(&TokenType::Import, "Expected 'import' keyword")?;
    
    let items = parse_import_list(parser)?;
    parser.consume(&TokenType::Semicolon, "Expected ';' after import statement")?;
    Ok(Statement::ImportFrom { module, items })
}

/// Parses import lists for import declarations
pub fn parse_import_list(parser: &mut super::Parser) -> Result<Vec<(String, Option<String>)>, ParseError> {
    let mut items = Vec::new();
    
    // Check if we have braces for destructuring syntax
    let has_braces = parser.match_token(&TokenType::LeftBrace);
    
    // Parse comma-separated list
    loop {
        let item = if let TokenType::Identifier(name) = &parser.peek().token_type {
            name.clone()
        } else {
            return Err(ParseError {
                message: "Expected import item".to_string(),
                line: parser.peek().line,
            });
        };
        
        parser.consume(&TokenType::Identifier(item.clone()), "Expected import item")?;
        
        let alias = if parser.match_token(&TokenType::As) {
            let alias = if let TokenType::Identifier(name) = &parser.peek().token_type {
                Some(name.clone())
            } else {
                return Err(ParseError {
                    message: "Expected alias name".to_string(),
                    line: parser.peek().line,
                });
            };
            
            parser.consume(&TokenType::Identifier(alias.clone().unwrap()), "Expected alias name")?;
            alias
        } else {
            None
        };
        
        items.push((item, alias));
        
        if !parser.match_token(&TokenType::Comma) {
            break;
        }
    }
    
    // If we started with braces, we need to close them
    if has_braces {
        parser.consume(&TokenType::RightBrace, "Expected '}' after import list")?;
    }
    
    Ok(items)
}

/// Parses ASSIGN_MAIN declarations
pub fn parse_assign_main_declaration(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    parser.consume(&TokenType::Minus, "Expected '-' after 'ASSIGN_MAIN'")?;
    parser.consume(&TokenType::Greater, "Expected '>' after '-'")?;
    
    let function_name = if let TokenType::String(name) = &parser.peek().token_type {
        name.clone()
    } else {
        return Err(ParseError {
            message: "Expected string literal for function name".to_string(),
            line: parser.peek().line,
        });
    };
    
    parser.advance(); // consume the string
    parser.consume(&TokenType::Semicolon, "Expected ';' after ASSIGN_MAIN declaration")?;
    
    Ok(Statement::AssignMain { function_name })
}

/// Parses sub module declarations: `sub mod MODULE_NAME;`
pub fn parse_sub_mod_declaration(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    parser.consume(&TokenType::Sub, "Expected 'sub' keyword")?;
    parser.consume(&TokenType::Mod, "Expected 'mod' keyword after 'sub'")?;
    
    let name = if let TokenType::Identifier(name) = &parser.peek().token_type {
        name.clone()
    } else {
        return Err(ParseError {
            message: "Expected module name after 'sub mod'".to_string(),
            line: parser.peek().line,
        });
    };
    
    parser.consume(&TokenType::Identifier(name.clone()), "Expected module name")?;
    parser.consume(&TokenType::Semicolon, "Expected ';' after sub mod declaration")?;
    
    Ok(Statement::SubModDeclaration { name })
}

/// Parses entry module declarations: `entry mod MODULE_NAME;` 
/// followed by optional `export submodule1, submodule2;`
pub fn parse_entry_mod_declaration(parser: &mut super::Parser) -> Result<Statement, ParseError> {
    parser.consume(&TokenType::Entry, "Expected 'entry' keyword")?;
    parser.consume(&TokenType::Mod, "Expected 'mod' keyword after 'entry'")?;
    
    let name = if let TokenType::Identifier(name) = &parser.peek().token_type {
        name.clone()
    } else {
        return Err(ParseError {
            message: "Expected module name after 'entry mod'".to_string(),
            line: parser.peek().line,
        });
    };
    
    parser.consume(&TokenType::Identifier(name.clone()), "Expected module name")?;
    parser.consume(&TokenType::Semicolon, "Expected ';' after entry mod declaration")?;
    
    // The exports list is handled separately when we encounter "export submodule1, submodule2;"
    // For now, we'll return with an empty exports list
    // The semantic analyzer will read the export.nlang file directly
    Ok(Statement::EntryModDeclaration { 
        name,
        exports: Vec::new() 
    })
}
